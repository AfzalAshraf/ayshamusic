//! Bounded, seekable compressed-media input for the native MP4/AAC decoder.
//! MP4 may place its moov/index after mdat: an unseekable FIFO is not a file.
use std::io::{self, Read, Seek, SeekFrom};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::time::Duration;

pub const BLOCK: usize = 64 * 1024;
const MAX_MEDIA: u64 = 512 * 1024 * 1024;
fn failure(message: &str) -> io::Error { io::Error::other(message) }

pub struct Block {
    pub bytes: Vec<u8>,
    pub total: u64,
}
pub trait Fetch: Send + Sync {
    fn fetch(&self, start: u64, end: u64) -> io::Result<Block>;
}
impl Fetch for Box<dyn Fetch> {
    fn fetch(&self, start:u64, end:u64)->io::Result<Block> { (**self).fetch(start,end) }
}
pub struct HttpFetch {
    client: reqwest::blocking::Client,
    url: String,
    user_agent: &'static str,
}
impl HttpFetch {
    pub fn new(url: String, user_agent: &'static str) -> io::Result<Self> {
        if !crate::youtube::is_allowed_media_url(&url) { return Err(failure("media_url_rejected")); }
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(12))
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .pool_max_idle_per_host(1)
            .build().map_err(|_| failure("media_http_client_unavailable"))?;
        Ok(Self { client, url, user_agent })
    }
}
impl Fetch for HttpFetch {
    fn fetch(&self, start: u64, end: u64) -> io::Result<Block> {
        let response = self.client.get(&self.url)
            .header(reqwest::header::USER_AGENT, self.user_agent)
            .header(reqwest::header::ACCEPT_ENCODING, "identity")
            .header(reqwest::header::RANGE, format!("bytes={start}-{end}"))
            .send().map_err(|_| failure("media_range_connection_or_timeout"))?;
        if response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(failure(&format!("media_range_http_{}", response.status().as_u16())));
        }
        if response.headers().get(reqwest::header::CONTENT_ENCODING).is_some_and(|v| v != "identity") {
            return Err(failure("media_range_encoding_rejected"));
        }
        let content_type = response.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("");
        if !matches!(content_type.split(';').next().unwrap_or("").trim(), "audio/mp4" | "video/mp4" | "application/octet-stream") {
            return Err(failure("media_range_content_type_rejected"));
        }
        let raw = response.headers().get(reqwest::header::CONTENT_RANGE).and_then(|v| v.to_str().ok()).ok_or_else(|| failure("media_range_header_missing"))?;
        let (last, total) = content_range(raw, start, end)?;
        let expected = (last - start + 1) as usize;
        let mut bytes = Vec::with_capacity(expected);
        response.take(expected as u64 + 1).read_to_end(&mut bytes).map_err(|_| failure("media_range_read_failed"))?;
        if bytes.len() != expected { return Err(failure("media_range_length_mismatch")); }
        Ok(Block { bytes, total })
    }
}
fn content_range(raw: &str, start: u64, end: u64) -> io::Result<(u64, u64)> {
    let parse = || -> Option<(u64, u64, u64)> {
        let (range, total) = raw.strip_prefix("bytes ")?.split_once('/')?;
        let (first, last) = range.split_once('-')?;
        Some((first.parse().ok()?, last.parse().ok()?, total.parse().ok()?))
    };
    let (first, last, total) = parse().ok_or_else(|| failure("media_range_header_invalid"))?;
    if first != start || total == 0 || total > MAX_MEDIA || last != end.min(total - 1) || last < first {
        return Err(failure("media_range_bounds_rejected"));
    }
    Ok((last, total))
}

pub struct RangeReader<F: Fetch> {
    fetch: F,
    cancel: Arc<AtomicBool>,
    total: u64,
    position: u64,
    cache_start: u64,
    cache: Vec<u8>,
    requests: usize,
    health: Arc<Mutex<Option<String>>>,
}
impl<F: Fetch> RangeReader<F> {
    pub fn new(fetch: F, expected: Option<u64>, cancel: Arc<AtomicBool>) -> io::Result<Self> {
        if cancel.load(Ordering::Acquire) { return Err(failure("playback_cancelled")); }
        let block = fetch.fetch(0, BLOCK as u64 - 1)?;
        if block.total == 0 || block.total > MAX_MEDIA || block.bytes.len() != block.total.min(BLOCK as u64) as usize || expected.is_some_and(|n| n != block.total) {
            return Err(failure("media_length_rejected"));
        }
        Ok(Self { fetch, cancel, total: block.total, position: 0, cache_start: 0, cache: block.bytes, requests: 1, health: Arc::new(Mutex::new(None)) })
    }
    pub fn len(&self) -> u64 { self.total }
    pub fn health(&self) -> Arc<Mutex<Option<String>>> { self.health.clone() }
    fn check_cancel(&self) -> io::Result<()> {
        if self.cancel.load(Ordering::Acquire) { Err(failure("playback_cancelled")) } else { Ok(()) }
    }
}
impl<F: Fetch> RangeReader<F> {
    fn read_inner(&mut self, out: &mut [u8]) -> io::Result<usize> {
        self.check_cancel()?;
        if out.is_empty() || self.position >= self.total { return Ok(0); }
        if self.position < self.cache_start || self.position >= self.cache_start + self.cache.len() as u64 {
            if self.requests >= 16384 { return Err(failure("media_range_request_limit")); }
            self.requests += 1;
            let end = (self.position + BLOCK as u64 - 1).min(self.total - 1);
            let block = self.fetch.fetch(self.position, end)?;
            self.check_cancel()?;
            if block.total != self.total || block.bytes.len() != (end - self.position + 1) as usize { return Err(failure("media_changed_or_truncated")); }
            self.cache = block.bytes;
            self.cache_start = self.position;
        }
        let offset = (self.position - self.cache_start) as usize;
        let count = out.len().min(self.cache.len() - offset);
        out[..count].copy_from_slice(&self.cache[offset..offset + count]);
        self.position += count as u64;
        Ok(count)
    }
}
impl<F: Fetch> Read for RangeReader<F> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let result = self.read_inner(out);
        if let Err(error) = &result {
            if !self.cancel.load(Ordering::Acquire) { *self.health.lock().unwrap() = Some(error.to_string()); }
        }
        result
    }
}
impl<F: Fetch> Seek for RangeReader<F> {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        self.check_cancel()?;
        let position = match from {
            SeekFrom::Start(n) => i128::from(n),
            SeekFrom::End(n) => i128::from(self.total) + i128::from(n),
            SeekFrom::Current(n) => i128::from(self.position) + i128::from(n),
        };
        if !(0..=i128::from(self.total)).contains(&position) { return Err(failure("media_seek_out_of_bounds")); }
        self.position = position as u64;
        Ok(self.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Memory(Arc<Vec<u8>>);
    impl Fetch for Memory {
        fn fetch(&self, start: u64, end: u64) -> io::Result<Block> {
            let last = (end as usize + 1).min(self.0.len());
            Ok(Block { bytes: self.0[start as usize..last].to_vec(), total: self.0.len() as u64 })
        }
    }
    #[test]
    fn bounded_seek_reads_tail_and_returns_to_audio_without_full_download() {
        let bytes = Arc::new((0..BLOCK * 4).map(|n| (n % 251) as u8).collect::<Vec<_>>());
        let cancel = Arc::new(AtomicBool::new(false));
        let mut reader = RangeReader::new(Memory(bytes.clone()), None, cancel.clone()).unwrap();
        reader.seek(SeekFrom::End(-100)).unwrap();
        let mut out = [0; 100]; reader.read_exact(&mut out).unwrap();
        assert_eq!(&out[..], &bytes[bytes.len()-100..]);
        reader.seek(SeekFrom::Start(16)).unwrap(); reader.read_exact(&mut out).unwrap();
        assert_eq!(&out[..], &bytes[16..116]); assert!(reader.cache.len() <= BLOCK);
        assert_eq!(reader.requests, 3);
        assert!(reader.seek(SeekFrom::End(1)).is_err());
        cancel.store(true, Ordering::Release); assert!(reader.read(&mut out).is_err());
    }
    #[test]
    fn range_contract_rejects_wrong_offsets_unknown_total_and_short_response() {
        assert_eq!(content_range("bytes 0-99/100", 0, 65535).unwrap(), (99, 100));
        for value in ["bytes 1-99/100", "bytes 0-99/*", "bytes 0-100/100", "bytes 0-50/100", "other"] {
            assert!(content_range(value, 0, 65535).is_err(), "{value}");
        }
        assert!(RangeReader::new(Memory(Arc::new(vec![1; 10])), Some(11), Arc::new(AtomicBool::new(false))).is_err());
    }
    #[test]
    fn http_range_reader_validates_real_response_before_decoder_sees_bytes() {
        use std::io::Write;
        for (status, range, okay) in [(206, "Content-Range: bytes 0-4/5\r\n", true), (200, "", false), (403, "", false), (206, "", false)] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let server = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                socket.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                let mut request = Vec::new();
                loop {
                    let mut byte = [0]; socket.read_exact(&mut byte).unwrap(); request.push(byte[0]);
                    if request.ends_with(b"\r\n\r\n") { break; }
                    assert!(request.len() < 8192);
                }
                write!(socket, "HTTP/1.1 {status} fixture\r\nContent-Type: audio/mp4\r\n{range}Content-Length: 5\r\nConnection: close\r\n\r\nabcde").unwrap();
                String::from_utf8(request).unwrap().to_lowercase()
            });
            let fetch = HttpFetch { client: reqwest::blocking::Client::new(), url:format!("http://{address}/fixture"), user_agent:"fixture" };
            let reader = RangeReader::new(fetch, Some(5), Arc::new(AtomicBool::new(false)));
            assert_eq!(reader.is_ok(), okay);
            let request = server.join().unwrap();
            assert!(request.contains("range: bytes=0-65535"));
            assert!(request.contains("accept-encoding: identity"));
        }
    }
    // Required in Android CI: real AAC demux + decode, no sound device or YouTube.
    #[test]
    fn decodes_real_aac_with_index_at_tail_and_front() {
        use rodio::Source;
        let Ok(root) = std::env::var("AYSHA_MEDIA_FIXTURES") else { return; };
        for name in ["tail.m4a", "front.m4a"] {
            let data = Arc::new(std::fs::read(std::path::Path::new(&root).join(name)).unwrap());
            // Reproduce the old transport's MP4 limitation without YouTube.
            if name == "tail.m4a" {
                struct Forward(std::io::Cursor<Vec<u8>>);
                impl Read for Forward { fn read(&mut self, b: &mut [u8]) -> io::Result<usize> { self.0.read(b) } }
                impl Seek for Forward { fn seek(&mut self, _: SeekFrom) -> io::Result<u64> { Err(failure("unseekable")) } }
                let old = rodio::Decoder::builder().with_data(Forward(std::io::Cursor::new((*data).clone())))
                    .with_hint("m4a").with_seekable(false).build();
                assert!(old.is_err(), "old unseekable path must reproduce the tail-index defect");
            }
            let reader = RangeReader::new(Memory(data), None, Arc::new(AtomicBool::new(false))).unwrap();
            let len = reader.len();
            let decoder = rodio::Decoder::builder().with_data(reader).with_byte_len(len).with_hint("m4a").with_seekable(true).build().unwrap();
            assert!(decoder.total_duration().is_some());
            let samples: Vec<f32> = decoder.take(100000).collect();
            assert!(samples.len() > 1000, "{name}");
            assert!(samples.iter().any(|sample| sample.abs() > 0.01), "{name}");
        }
    }
}
