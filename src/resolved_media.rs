//! Opt-in compatibility resolver + native HTTP audio transfer.
//! yt-dlp/optional JS is still needed during resolution. NOT all-Rust extraction.
use axum::{body::{Body, Bytes}, http::{HeaderMap, HeaderName, HeaderValue, StatusCode}, response::Response};
use futures_util::{Stream, StreamExt};
use serde::Deserialize;
use std::{io, pin::Pin, sync::OnceLock, task::{Context, Poll}, time::Duration};
use tokio::{process::Command, sync::{Semaphore, SemaphorePermit}};

const RECORD_MAX: usize = 256 * 1024;
const CHUNK_MAX: usize = 256 * 1024;
const PROJECTION: &str = "%(.{id,url,http_headers,protocol,vcodec,acodec,ext,is_live,live_status})j";
static STREAM_SLOT: Semaphore = Semaphore::const_new(1);
static CLIENT: OnceLock<Result<reqwest::Client, ()>> = OnceLock::new();
type Upstream = Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>;

pub fn is_active() -> bool { STREAM_SLOT.available_permits() == 0 }

// Never Debug/Serialize this record: URL and headers can contain sensitive values.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    id: String,
    url: String,
    http_headers: serde_json::Map<String, serde_json::Value>,
    protocol: String,
    vcodec: String,
    acodec: String,
    ext: String,
    #[serde(default)] is_live: Option<bool>,
    #[serde(default)] live_status: Option<String>,
}
struct Selection { url: reqwest::Url, headers: HeaderMap, mime: &'static str }

fn valid_id(id: &str) -> bool {
    id.len() == 11 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
fn youtube_url(url: &reqwest::Url) -> bool {
    url.scheme() == "https" && url.username().is_empty() && url.password().is_none()
        && url.port().is_none() && url.fragment().is_none()
        && matches!(url.host_str(), Some("youtube.com" | "www.youtube.com" | "m.youtube.com" | "music.youtube.com" | "youtu.be"))
}
fn expected_id(value: &str) -> Option<String> {
    let url = reqwest::Url::parse(value).ok()?;
    if !youtube_url(&url) { return None; }
    let id = if url.host_str() == Some("youtu.be") {
        url.path().strip_prefix('/')?.to_owned()
    } else if url.path() == "/watch" {
        let ids: Vec<_> = url.query_pairs().filter(|(key, _)| key == "v").map(|(_, id)| id.into_owned()).collect();
        if ids.len() != 1 { return None; }
        ids.into_iter().next()?
    } else if let Some(id) = url.path().strip_prefix("/shorts/") { id.to_owned() }
    else { return None; };
    valid_id(&id).then_some(id)
}
fn cdn_url(value: &str) -> Option<reqwest::Url> {
    if value.len() > 16 * 1024 { return None; }
    let url = reqwest::Url::parse(value).ok()?;
    (url.scheme() == "https" && url.username().is_empty() && url.password().is_none()
        && url.port().is_none() && url.fragment().is_none() && url.path() == "/videoplayback"
        && url.host_str().is_some_and(|host| host.ends_with(".googlevideo.com"))).then_some(url)
}
fn headers(values: serde_json::Map<String, serde_json::Value>) -> Result<HeaderMap, &'static str> {
    if values.is_empty() || values.len() > 8 { return Err("resolver_headers_rejected"); }
    let mut result = HeaderMap::new();
    let mut size = 0usize;
    for (key, value) in values {
        let text = value.as_str().ok_or("resolver_headers_rejected")?;
        size += key.len() + text.len();
        if size > 8192 || text.len() > 4096 { return Err("resolver_headers_rejected"); }
        let name = HeaderName::from_bytes(key.as_bytes()).map_err(|_| "resolver_headers_rejected")?;
        if result.contains_key(&name) { return Err("resolver_headers_rejected"); }
        match name.as_str() {
            "user-agent" | "accept" | "accept-language" => (),
            "sec-fetch-mode" if text == "navigate" => (),
            "origin" | "referer" => {
                let url = reqwest::Url::parse(text).map_err(|_| "resolver_headers_rejected")?;
                if !youtube_url(&url) || (name.as_str() == "origin" && (url.path() != "/" || url.query().is_some())) {
                    return Err("resolver_headers_rejected");
                }
            }
            // Includes cookies/auth/proxy/host/range/encoding/hop-by-hop overrides.
            _ => return Err("resolver_headers_rejected"),
        }
        result.insert(name, HeaderValue::from_str(text).map_err(|_| "resolver_headers_rejected")?);
    }
    if !result.contains_key("user-agent") { return Err("resolver_headers_rejected"); }
    result.insert("accept-encoding", HeaderValue::from_static("identity"));
    Ok(result)
}
fn parse_record(bytes: &[u8], expected: &str) -> Result<Selection, &'static str> {
    if bytes.len() > RECORD_MAX { return Err("resolver_record_limit"); }
    let record: Record = serde_json::from_slice(bytes).map_err(|_| "resolver_record_invalid")?;
    if record.id != expected || !valid_id(&record.id) { return Err("resolver_identity_mismatch"); }
    if record.is_live == Some(true) || record.live_status.as_deref().is_some_and(|value| value != "not_live") {
        return Err("resolver_live_unsupported");
    }
    if record.protocol != "https" || record.vcodec != "none" { return Err("resolver_protocol_unsupported"); }
    let mime = match (record.ext.as_str(), record.acodec.as_str()) {
        ("webm", "opus" | "vorbis") => "audio/webm",
        ("m4a" | "mp4", "mp4a.40.2" | "mp4a.40.5") => "audio/mp4",
        _ => return Err("resolver_codec_unsupported"),
    };
    Ok(Selection { url: cdn_url(&record.url).ok_or("resolver_origin_rejected")?, headers: headers(record.http_headers)?, mime })
}
fn resolver_command(url: &str, runtime: Option<&str>) -> Command {
    let mut command = Command::new("yt-dlp");
    command.args(["--ignore-config", "--no-cache-dir", "--no-playlist", "--no-progress", "--no-warnings",
                  "--simulate", "-f", "bestaudio", "--print", PROJECTION,
                  "--socket-timeout", "15", "--retries", "1"]);
    if let Some(runtime) = runtime.filter(|value| !value.is_empty()) { command.args(["--js-runtimes", runtime]); }
    command.args(["--", url]);
    for key in ["HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy"] { command.env_remove(key); }
    command
}
async fn resolve(command: Command, expected: &str) -> Result<Selection, &'static str> {
    // Success is returned only after the bounded helper exits and is reaped.
    // No media request starts while the resolver child is alive.
    let bytes = crate::metadata_budget::helper_output(command).await?;
    parse_record(&bytes, expected)
}
fn client() -> Result<&'static reqwest::Client, &'static str> {
    CLIENT.get_or_init(|| reqwest::Client::builder().https_only(true).http1_only()
        .no_proxy().redirect(reqwest::redirect::Policy::none()).pool_max_idle_per_host(0)
        .connect_timeout(Duration::from_secs(10)).read_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(2 * 60 * 60)).build().map_err(|_| ()))
        .as_ref().map_err(|_| "native_client_unavailable")
}
fn response_allowed(response: &reqwest::Response, mime: &str) -> bool {
    if response.status() != StatusCode::OK || response.headers().contains_key("content-range") { return false; }
    if response.headers().get("content-encoding").is_some_and(|value| value != "identity") { return false; }
    if let Some(kind) = response.headers().get("content-type") {
        let Ok(kind) = kind.to_str() else { return false };
        let kind = kind.split(';').next().unwrap_or("").trim();
        if kind != mime && kind != "application/octet-stream" { return false; }
    }
    !response.content_length().is_some_and(|size| size < 12)
}
async fn prefix(upstream: &mut Upstream) -> Result<(Bytes, Option<Bytes>), &'static str> {
    let mut bytes = [0u8; 12]; let mut used = 0;
    while used < bytes.len() {
        let chunk = upstream.next().await.ok_or("native_stream_short_header")?
            .map_err(|_| "native_stream_read_failed")?;
        if chunk.len() > CHUNK_MAX { return Err("native_stream_chunk_limit"); }
        let take = (bytes.len() - used).min(chunk.len());
        bytes[used..used + take].copy_from_slice(&chunk[..take]); used += take;
        if used == bytes.len() {
            return Ok((Bytes::copy_from_slice(&bytes), (chunk.len() > take).then(|| chunk.slice(take..))));
        }
    }
    unreachable!("fixed nonempty header")
}
async fn native_response(response: reqwest::Response, mime: &'static str,
                         permit: SemaphorePermit<'static>) -> Result<Response, &'static str> {
    if !response_allowed(&response, mime) { return Err("native_stream_response_rejected"); }
    let mut upstream: Upstream = Box::pin(response.bytes_stream());
    let (first, pending) = prefix(&mut upstream).await?;
    if crate::media::content_type(&first) != Some(mime) { return Err("native_stream_container_rejected"); }
    let stream = NativeAudioStream { upstream: Some(upstream), first: Some(first), pending, permit: Some(permit) };
    Response::builder().header("Content-Type", mime).header("Cache-Control", "no-store")
        .header("Accept-Ranges", "none").header("X-Aysha-Transport", "rust-resolved")
        .body(Body::from_stream(stream)).map_err(|_| "native_stream_response_failed")
}

pub async fn open_stream(url: &str) -> Result<Response, StatusCode> {
    let expected = expected_id(url).ok_or(StatusCode::BAD_REQUEST)?;
    // Trial is deliberately one active stream, no unbounded waiter queue.
    let permit = STREAM_SLOT.try_acquire().map_err(|_| StatusCode::TOO_MANY_REQUESTS)?;
    let runtime = std::env::var("MUSIC_YTDLP_JS_RUNTIME").ok();
    let command = resolver_command(url, runtime.as_deref());
    let trial = tokio::time::timeout(Duration::from_secs(45), async {
        let selected = resolve(command, &expected).await?;
        let response = client()?.get(selected.url).headers(selected.headers).send().await
            .map_err(|_| "native_stream_request_failed")?;
        native_response(response, selected.mime, permit).await
    }).await;
    match trial {
        Ok(Ok(response)) => Ok(response),
        Ok(Err(code)) => {
            // Fixed code only. reqwest errors may contain signed URLs: never print them.
            eprintln!("Rust-resolved stream: {code}");
            Err(if code == "metadata_busy" { StatusCode::SERVICE_UNAVAILABLE } else { StatusCode::BAD_GATEWAY })
        }
        Err(_) => { eprintln!("Rust-resolved stream: native_startup_timeout"); Err(StatusCode::GATEWAY_TIMEOUT) }
    }
}

struct NativeAudioStream {
    upstream: Option<Upstream>, first: Option<Bytes>, pending: Option<Bytes>,
    permit: Option<SemaphorePermit<'static>>,
}
impl NativeAudioStream {
    fn finish(&mut self) { self.upstream.take(); self.permit.take(); }
}
impl Stream for NativeAudioStream {
    type Item = Result<Bytes, io::Error>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if let Some(first) = this.first.take() { return Poll::Ready(Some(Ok(first))); }
        if let Some(pending) = this.pending.take() { return Poll::Ready(Some(Ok(pending))); }
        let Some(upstream) = &mut this.upstream else { return Poll::Ready(None) };
        match upstream.as_mut().poll_next(cx) {
            Poll::Ready(None) => { this.finish(); Poll::Ready(None) }
            Poll::Ready(Some(Ok(chunk))) if chunk.len() <= CHUNK_MAX => Poll::Ready(Some(Ok(chunk))),
            Poll::Ready(Some(_)) => {
                this.finish();
                Poll::Ready(Some(Err(io::Error::other("native_stream_transfer_failed"))))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    const ID: &str = "fixture0001";
    fn fixture() -> serde_json::Value {
        json!({"id":ID, "url":"https://rr1.googlevideo.com/videoplayback?fixture=1",
            "http_headers":{"User-Agent":"fixture","Accept":"*/*","Sec-Fetch-Mode":"navigate"},
            "protocol":"https","vcodec":"none","acodec":"opus","ext":"webm","is_live":false,"live_status":"not_live"})
    }
    fn parse(value: serde_json::Value) -> Result<Selection, &'static str> {
        parse_record(&serde_json::to_vec(&value).unwrap(), ID)
    }
    fn lease() -> (&'static Semaphore, SemaphorePermit<'static>) {
        let slots: &'static Semaphore = Box::leak(Box::new(Semaphore::new(1)));
        (slots, slots.try_acquire().unwrap())
    }
    #[test]
    fn ordinary_watch_short_and_share_urls_have_an_unambiguous_identity() {
        for url in ["https://www.youtube.com/watch?v=fixture0001", "https://youtu.be/fixture0001?t=1", "https://www.youtube.com/shorts/fixture0001"] {
            assert_eq!(expected_id(url).as_deref(), Some(ID));
        }
        for url in ["https://example.com/watch?v=fixture0001", "https://www.youtube.com/watch?v=fixture0001&v=other000001",
                    "https://youtu.be/fixture0001/extra", "https://www.youtube.com/playlist?list=fixture",
                    "https://user@www.youtube.com/watch?v=fixture0001", "http://www.youtube.com/watch?v=fixture0001"] {
            assert!(expected_id(url).is_none());
        }
    }
    #[test]
    fn selected_audio_identity_protocol_live_and_codec_guards_do_not_choose_a_lower_format() {
        assert_eq!(parse(fixture()).unwrap().mime, "audio/webm");
        for (key,value) in [("id",json!("other000001")),("is_live",json!(true)),("live_status",json!("is_upcoming")),
                           ("protocol",json!("m3u8_native")),("vcodec",json!("avc1")),("acodec",json!("none")),("ext",json!("mhtml"))] {
            let mut record = fixture(); record[key] = value; assert!(parse(record).is_err(), "accepted {key}");
        }
        let mut aac = fixture(); aac["ext"] = json!("m4a"); aac["acodec"] = json!("mp4a.40.2");
        assert_eq!(parse(aac).unwrap().mime,"audio/mp4");
    }
    #[test]
    fn resolver_records_are_single_bounded_json_objects() {
        assert!(parse_record(&vec![b' '; RECORD_MAX + 1], ID).is_err());
        assert!(parse_record(b"{}\n{}", ID).is_err());
        assert!(parse_record(b"[]", ID).is_err());
        let mut record=fixture(); record["unexpected"]=json!("fixture");assert!(parse(record).is_err());
        let serialized=serde_json::to_string(&fixture()).unwrap();
        let duplicate=serialized.replacen("{", "{\"id\":\"fixture0001\",",1);
        assert!(parse_record(duplicate.as_bytes(), ID).is_err());
    }
    #[test]
    fn media_origin_cannot_be_arbitrary_or_follow_a_redirect_to_local_networks() {
        for url in ["http://rr1.googlevideo.com/videoplayback", "https://googlevideo.com.evil.test/videoplayback",
                    "https://127.0.0.1/videoplayback", "https://user@rr1.googlevideo.com/videoplayback",
                    "https://rr1.googlevideo.com:8443/videoplayback", "https://rr1.googlevideo.com/other",
                    "https://rr1.googlevideo.com/videoplayback#fragment"] {
            assert!(cdn_url(url).is_none(), "accepted fixture origin");
        }
    }
    #[test]
    fn sensitive_unsupported_oversized_and_case_duplicate_headers_fail_closed() {
        for (key,value) in [("Cookie","private"),("Authorization","private"),("Host","private"),("Range","bytes=1-"),
                           ("Connection","keep-alive"),("Accept-Encoding","gzip"),("X-Private","private"),
                           ("user-agent","duplicate"),("Referer","http://127.0.0.1/private"),("Origin","https://www.youtube.com/watch?v=fixture0001")] {
            let mut record=fixture();record["http_headers"][key]=json!(value);assert!(parse(record).is_err(),"accepted {key}");
        }
        let mut record=fixture(); record["http_headers"]["User-Agent"]=json!("x".repeat(4097));assert!(parse(record).is_err());
        let mut record=fixture(); record["http_headers"]["Referer"]=json!("https://www.youtube.com/watch?v=fixture0001");
        assert!(parse(record).is_ok());
    }
    #[test]
    fn resolver_command_only_prints_projection_and_never_downloads_or_adds_credentials() {
        let command=resolver_command("https://www.youtube.com/watch?v=fixture0001", Some("node:/fixture/node"));
        let args:Vec<_>=command.as_std().get_args().map(|arg|arg.to_str().unwrap()).collect();
        for flag in ["--simulate","--ignore-config","--no-cache-dir","--no-playlist"] { assert!(args.contains(&flag)); }
        assert!(args.windows(2).any(|pair| pair==["-f","bestaudio"]));
        assert!(args.windows(2).any(|pair| pair==["--print",PROJECTION]));
        for flag in ["-o","--exec","--cookies","--cookies-from-browser"] { assert!(!args.contains(&flag)); }
    }
    #[tokio::test]
    async fn fixture_resolver_is_reaped_before_a_selection_is_returned() {
        let mut command=Command::new("printf");command.arg("%s").arg(serde_json::to_string(&fixture()).unwrap());
        let selected=resolve(command,ID).await.unwrap();
        assert_eq!(selected.mime,"audio/webm");
    }
    #[tokio::test]
    async fn native_body_preserves_exact_bytes_and_releases_slot_at_eof() {
        let bytes=b"\x1aE\xdf\xa312345678fixture-bit-exact".to_vec();
        let response=reqwest::Response::from(axum::http::Response::builder().header("content-type","audio/webm").body(bytes.clone()).unwrap());
        let (slots,permit)=lease();
        let response=native_response(response,"audio/webm",permit).await.unwrap();
        assert_eq!(response.headers()["X-Aysha-Transport"],"rust-resolved");
        assert_eq!(response.headers()["Accept-Ranges"],"none");
        assert_eq!(&axum::body::to_bytes(response.into_body(),1024).await.unwrap()[..],&bytes);
        assert_eq!(slots.available_permits(),1);
    }
    #[tokio::test]
    async fn upstream_redirects_partial_errors_encoding_and_wrong_containers_are_rejected() {
        for status in [302,403,206,500] {
            let response=reqwest::Response::from(axum::http::Response::builder().status(status).body(b"not audio".to_vec()).unwrap());
            let (slots,permit)=lease();assert!(native_response(response,"audio/webm",permit).await.is_err());
            assert_eq!(slots.available_permits(),1);
        }
        for (key,value) in [("content-type","text/html"),("content-encoding","gzip"),("content-range","bytes 0-15/16")] {
            let response=reqwest::Response::from(axum::http::Response::builder().header(key,value).body(b"\x1aE\xdf\xa312345678fixture".to_vec()).unwrap());
            let (_,permit)=lease();assert!(native_response(response,"audio/webm",permit).await.is_err());
        }
        let response=reqwest::Response::from(axum::http::Response::new(b"<html>not audio</html>".to_vec()));
        let (_,permit)=lease();assert!(native_response(response,"audio/webm",permit).await.is_err());
    }
    #[tokio::test]
    async fn fragmented_prefix_and_early_drop_are_bounded_and_release_ownership() {
        let chunks=vec![Ok(Bytes::from_static(b"\x1aE")),Ok(Bytes::from_static(b"\xdf\xa312345678tail"))];
        let mut upstream:Upstream=Box::pin(futures_util::stream::iter(chunks));
        let (first,pending)=prefix(&mut upstream).await.unwrap();assert_eq!(&first[..],b"\x1aE\xdf\xa312345678");
        assert_eq!(&pending.unwrap()[..],b"tail");
        let (slots,permit)=lease();
        let stream=NativeAudioStream { upstream:Some(upstream),first:Some(first),pending:None,permit:Some(permit) };
        drop(stream);assert_eq!(slots.available_permits(),1);
        let mut oversized:Upstream=Box::pin(futures_util::stream::iter(vec![Ok(Bytes::from(vec![0;CHUNK_MAX+1]))]));
        assert!(prefix(&mut oversized).await.is_err());
    }
    #[tokio::test]
    async fn late_oversized_chunks_fail_and_release_even_if_stream_object_is_retained() {
        let (slots,permit)=lease();
        let upstream:Upstream=Box::pin(futures_util::stream::iter(vec![Ok(Bytes::from(vec![0;CHUNK_MAX+1]))]));
        let mut stream=NativeAudioStream { upstream:Some(upstream),first:None,pending:None,permit:Some(permit) };
        assert!(stream.next().await.unwrap().is_err());
        assert_eq!(slots.available_permits(),1);assert!(stream.next().await.is_none());
    }
}
