//! Bounded-startup audio proxy with container detection and downloader ownership.
use axum::{body::{Body, Bytes}, http::StatusCode, response::Response};
use futures_util::stream::Stream;
use std::{pin::Pin, process::Stdio, task::{Context, Poll}, time::Duration};
use tokio::{io::{AsyncRead, AsyncReadExt, ReadBuf}, process::{Child, ChildStdout}};

fn content_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x1aE\xdf\xa3") {
        Some("audio/webm")
    } else if bytes.len() >= 8 && &bytes[4..8] == b"ftyp" {
        Some("audio/mp4")
    } else if bytes.starts_with(b"OggS") {
        Some("audio/ogg")
    } else if bytes.starts_with(b"ID3")
        || (bytes.len() >= 2 && bytes[0] == 255 && bytes[1] & 224 == 224) {
        Some("audio/mpeg")
    } else {
        None
    }
}

fn allowed_url(value: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(value) else { return false };
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && matches!(url.host_str(), Some("youtube.com" | "www.youtube.com" | "m.youtube.com" | "music.youtube.com" | "youtu.be"))
}

pub async fn open_stream(url: &str) -> Result<Response, StatusCode> {
    if !allowed_url(url) { return Err(StatusCode::BAD_REQUEST); }
    let child = tokio::process::Command::new("yt-dlp")
        .args(["-f", "bestaudio", "-o", "-", "--no-playlist", "--no-progress",
            "--socket-timeout", "15", "--retries", "1", "--", url])
        .stdout(Stdio::piped())
        // Failures are visible in journalctl instead of silently discarded.
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| {
            eprintln!("Unable to start yt-dlp: {error}");
            StatusCode::SERVICE_UNAVAILABLE
        })?;
    response_from_child(child).await
}

async fn response_from_child(mut child: Child) -> Result<Response, StatusCode> {
    let mut stdout = child.stdout.take().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut prefix = [0u8; 12];
    match tokio::time::timeout(Duration::from_secs(40), stdout.read_exact(&mut prefix)).await {
        Ok(Ok(_)) => (),
        Ok(Err(error)) => {
            eprintln!("yt-dlp returned no usable audio header: {error}");
            return Err(StatusCode::BAD_GATEWAY);
        }
        Err(_) => return Err(StatusCode::GATEWAY_TIMEOUT),
    }
    let mime = content_type(&prefix).ok_or(StatusCode::BAD_GATEWAY)?;
    let stream = AudioStream {
        stdout,
        _child: child,
        first: Some(Bytes::copy_from_slice(&prefix)),
        buf: [0; 8192],
        eof: false,
    };
    Response::builder()
        .header("Content-Type", mime)
        .header("Cache-Control", "no-store")
        // This sequential proxy does not implement Range/206 seeking.
        .header("Accept-Ranges", "none")
        .body(Body::from_stream(stream))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

struct AudioStream {
    stdout: ChildStdout,
    // Dropping the HTTP body closes stdout and terminates a still-running downloader.
    _child: Child,
    first: Option<Bytes>,
    buf: [u8; 8192],
    eof: bool,
}

impl Stream for AudioStream {
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if let Some(first) = this.first.take() { return Poll::Ready(Some(Ok(first))); }
        if this.eof { return Poll::Ready(None); }
        let mut rb = ReadBuf::new(&mut this.buf);
        match Pin::new(&mut this.stdout).poll_read(cx, &mut rb) {
            Poll::Ready(Ok(())) => {
                if rb.filled().is_empty() {
                    this.eof = true;
                    Poll::Ready(None)
                } else {
                    Poll::Ready(Some(Ok(Bytes::copy_from_slice(rb.filled()))))
                }
            }
            Poll::Ready(Err(error)) => {
                this.eof = true;
                Poll::Ready(Some(Err(error)))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;

    #[test]
    fn detects_supported_containers() {
        assert_eq!(content_type(b"\x1aE\xdf\xa3webm"), Some("audio/webm"));
        assert_eq!(content_type(b"\0\0\0\x20ftypM4A "), Some("audio/mp4"));
        assert_eq!(content_type(b"OggSdata"), Some("audio/ogg"));
        assert_eq!(content_type(b"ID3data"), Some("audio/mpeg"));
        assert_eq!(content_type(b"\xff\xfbdata"), Some("audio/mpeg"));
    }

    #[test]
    fn rejects_non_audio_headers() {
        for bytes in [b"".as_slice(), b"{\"error\":true}", b"<html>failed"] {
            assert_eq!(content_type(bytes), None);
        }
    }

    #[test]
    fn restricts_downloader_input() {
        assert!(allowed_url("https://www.youtube.com/watch?v=abc"));
        assert!(allowed_url("https://youtu.be/abc"));
        for url in ["--exec=sh", "file:///etc/passwd", "http://127.0.0.1/",
            "https://youtube.com.evil.test/", "https://user@youtube.com/",
            "https://youtube.com:8443/", "https://example.com/"] {
            assert!(!allowed_url(url), "accepted {url}");
        }
    }

    #[tokio::test]
    async fn invalid_url_fails_before_spawning() {
        assert_eq!(open_stream("file:///etc/passwd").await.unwrap_err(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn stream_preserves_prefix_and_terminates_at_eof() {
        let mut child = tokio::process::Command::new("sh")
            .args(["-c", "printf remainder"]).stdout(Stdio::piped())
            .kill_on_drop(true).spawn().unwrap();
        let mut stream = AudioStream {
            stdout: child.stdout.take().unwrap(), _child: child,
            first: Some(Bytes::from_static(b"prefix")), buf: [0; 8192], eof: false,
        };
        let mut result = Vec::new();
        while let Some(chunk) = stream.next().await { result.extend_from_slice(&chunk.unwrap()); }
        assert_eq!(result, b"prefixremainder");
        assert!(stream.next().await.is_none());
    }
    #[tokio::test]
    async fn response_matches_webm_bytes_without_claiming_ranges() {
        let child = tokio::process::Command::new("sh")
            .args(["-c", "printf '\\032E\\337\\24312345678body'"])
            .stdout(Stdio::piped()).kill_on_drop(true).spawn().unwrap();
        let response = response_from_child(child).await.unwrap();
        assert_eq!(response.headers()["Content-Type"], "audio/webm");
        assert_eq!(response.headers()["Accept-Ranges"], "none");
        let bytes = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
        assert_eq!(&bytes[..], b"\x1aE\xdf\xa312345678body");
    }

    #[tokio::test]
    async fn failed_downloader_does_not_return_http_200() {
        let child = tokio::process::Command::new("sh")
            .args(["-c", "exit 1"]).stdout(Stdio::piped())
            .kill_on_drop(true).spawn().unwrap();
        assert_eq!(response_from_child(child).await.unwrap_err(), StatusCode::BAD_GATEWAY);
    }

}
