//! Bounded, seekable compressed-media input for the native MP4/AAC decoder.
//! MP4 may place its moov/index after mdat: an unseekable FIFO is not a file.
use std::io::{self, Read, Seek, SeekFrom};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::time::{Duration, Instant};

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
        Self::with_client(url, user_agent, client)
    }
    /// For the bounded CI harness. Caller supplies a no-redirect HTTP client;
    /// redirects are validated manually here, before any next request is sent.
    pub(crate) fn with_client(url:String,user_agent:&'static str,client:reqwest::blocking::Client)->io::Result<Self>{
        if !crate::youtube::is_allowed_media_url(&url){return Err(failure("media_url_rejected"));}
        Ok(Self {client,url,user_agent})
    }
}
impl Fetch for HttpFetch {
    fn fetch(&self,start:u64,end:u64)->io::Result<Block>{
        self.fetch_with(start,end,|current,timeout|{
            self.client.get(current).timeout(timeout)
                .header(reqwest::header::USER_AGENT,self.user_agent)
                .header(reqwest::header::ACCEPT_ENCODING,"identity")
                .header(reqwest::header::RANGE,format!("bytes={start}-{end}"))
                .send().map_err(|_|failure("media_range_connection_or_timeout"))
        })
    }
}
impl HttpFetch {
    // Private transport seam for local fixtures; production always uses the
    // guarded client above, never a provider allowlist exception.
    fn fetch_with<F>(&self,start:u64,end:u64,send:F)->io::Result<Block>
    where F:FnMut(&str,Duration)->io::Result<reqwest::blocking::Response>{
        // A redirect belongs to THIS request/range. A temporary signed target
        // must not become the origin for the next block (including tail seeks).
        let (response,_)=redirected_response(&self.url,send)?;
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
fn redirected_response<F>(initial:&str,mut send:F)->io::Result<(reqwest::blocking::Response,String)>
where F:FnMut(&str,Duration)->io::Result<reqwest::blocking::Response>{
    let deadline=Instant::now()+Duration::from_secs(12);
    let mut current=initial.to_string();let mut visited=vec![current.clone()];
    loop {
        let timeout=deadline.checked_duration_since(Instant::now()).ok_or_else(||failure("media_range_deadline"))?;
        let response=send(&current,timeout)?;
        if !matches!(response.status().as_u16(),301|302|303|307|308){return Ok((response,current));}
        let location=response.headers().get(reqwest::header::LOCATION).and_then(|v|v.to_str().ok())
            .filter(|v|v.len()<=8192).ok_or_else(||failure("media_redirect_location_rejected"))?;
        let next=redirect_target(&current,location)?;
        if visited.contains(&next){return Err(failure("media_redirect_loop"));}
        if visited.len()>=4{return Err(failure("media_redirect_limit"));}
        visited.push(next.clone());current=next;
    }
}

fn redirect_target(current:&str,location:&str)->io::Result<String>{
    let target=reqwest::Url::parse(current).and_then(|url|url.join(location))
        .map_err(|_|failure("media_redirect_url_invalid"))?;
    if !crate::youtube::is_allowed_media_url(target.as_str()){
        return Err(failure("media_redirect_target_rejected"));
    }
    Ok(target.into())
}

/// Before Media3 can start, prove both the beginning and the MP4 tail/index can
/// be read. Reuse those bytes instead of discarding the probe and downloading it
/// again. Still bounded to 2*64KiB; not a claim of complete-file availability.
pub struct PreparedFetch {fetch:Box<dyn Fetch>,first:Block,tail:Option<(u64,Block)>}
impl PreparedFetch {
    pub fn new(fetch:Box<dyn Fetch>,expected:Option<u64>)->io::Result<Self>{
        let first=fetch.fetch(0,BLOCK as u64-1).map_err(|e|failure(&format!("media_prepare_head_failed: {e}")))?;
        let total=first.total;
        if total==0||total>MAX_MEDIA||first.bytes.len() as u64!=total.min(BLOCK as u64)
            ||expected.is_some_and(|n|n!=total){return Err(failure("media3_input_length_rejected"));}
        let tail=if total>BLOCK as u64 {
            let start=total-BLOCK as u64;let tail=fetch.fetch(start,total-1).map_err(|e|failure(&format!("media_prepare_tail_failed: {e}")))?;
            if tail.total!=total||tail.bytes.len()!=BLOCK{return Err(failure("media3_input_tail_rejected"));}
            Some((start,tail))
        }else{None};
        Ok(Self{fetch,first,tail})
    }
    pub fn len(&self)->u64{self.first.total}
}
impl Fetch for PreparedFetch {
    fn fetch(&self,start:u64,end:u64)->io::Result<Block>{
        if start>end||end>=self.len()||end-start>=BLOCK as u64{return Err(failure("media_range_bounds_rejected"));}
        for (offset,block) in std::iter::once((0,&self.first)).chain(self.tail.as_ref().map(|(offset,block)|(*offset,block))){
            if start>=offset&&end<offset+block.bytes.len() as u64 {
                return Ok(Block{bytes:block.bytes[(start-offset) as usize..=(end-offset) as usize].to_vec(),total:self.len()});
            }
        }
        let block=self.fetch.fetch(start,end)?;
        if block.total!=self.len()||block.bytes.len() as u64!=end-start+1{return Err(failure("media_changed_or_truncated"));}
        Ok(block)
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
    #[test]
    fn redirect_destinations_keep_https_cdn_boundary_and_signed_query(){
        let base="https://r1.googlevideo.com/videoplayback?sig=one&n=a%2Bb";
        assert_eq!(redirect_target(base,"//r2.googlevideo.com/videoplayback?sig=two&n=a%2Bb").unwrap(),"https://r2.googlevideo.com/videoplayback?sig=two&n=a%2Bb");
        assert_eq!(redirect_target(base,"?sig=next").unwrap(),"https://r1.googlevideo.com/videoplayback?sig=next");
        for target in ["http://r2.googlevideo.com/a","https://youtube.com/a","https://googlevideo.com.evil.test/a","https://127.0.0.1/a","https://user:pw@r.googlevideo.com/a","https://r.googlevideo.com:444/a","file:///tmp/test"]{
            assert_eq!(redirect_target(base,target).unwrap_err().to_string(),"media_redirect_target_rejected");
        }
    }
    // Real local HTTP responses, but logical provider URLs stay separate from
    // fixture transport: no allowlist exception or external network in tests.
    fn fixture_response(status:u16,location:Option<&str>)->reqwest::blocking::Response {
        fixture_response_body(status,location,None,&[])
    }
    fn fixture_response_body(status:u16,location:Option<&str>,range:Option<&str>,body:&[u8])->reqwest::blocking::Response {
        use std::io::Write;
        let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();let addr=listener.local_addr().unwrap();
        let location=location.map(|v|format!("Location: {v}\r\n")).unwrap_or_default();
        let range=range.map(|v|format!("Content-Range: {v}\r\n")).unwrap_or_default();
        let body=body.to_vec();
        let server=std::thread::spawn(move||{
            let(mut socket,_)=listener.accept().unwrap();socket.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
            socket.set_write_timeout(Some(Duration::from_secs(3))).unwrap();
            let mut raw=Vec::new();while !raw.ends_with(b"\r\n\r\n"){raw.push(socket.read_u8_fixture());assert!(raw.len()<8192);}
            write!(socket,"HTTP/1.1 {status} fixture\r\n{location}{range}Content-Type: audio/mp4\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len()).unwrap();
            socket.write_all(&body).unwrap();
        });
        let response=reqwest::blocking::Client::builder().redirect(reqwest::redirect::Policy::none()).no_proxy().build().unwrap()
            .get(format!("http://{addr}/")).send().unwrap();server.join().unwrap();response
    }
    #[test]
    fn temporary_range_redirect_is_not_promoted_to_next_range_origin(){
        let origin="https://r1.googlevideo.com/videoplayback?sig=original";
        let fetch=HttpFetch::with_client(origin.into(),"fixture",reqwest::blocking::Client::new()).unwrap();
        for start in [0u64,4,28,12,0] {
            let end=start+3;
            let scoped=format!("https://r2.googlevideo.com/videoplayback?range={start}-{end}&sig=request_specific");
            let mut calls=0;
            let block=fetch.fetch_with(start,end,|url,_|{
                calls+=1;
                Ok(if url==origin {fixture_response(302,Some(&scoped))}
                else if url==scoped {fixture_response_body(206,None,Some(&format!("bytes {start}-{end}/32")),&[1,2,3,4])}
                else {fixture_response(403,None)})
            }).unwrap();
            assert_eq!(calls,2);assert_eq!(block.bytes,[1,2,3,4]);
        }
    }
    trait FixtureRead{fn read_u8_fixture(&mut self)->u8;}
    impl FixtureRead for std::net::TcpStream {fn read_u8_fixture(&mut self)->u8{let mut b=[0];self.read_exact(&mut b).unwrap();b[0]}}
    #[test]
    fn redirects_are_bounded_and_rejections_never_send_to_disallowed_targets(){
        let base="https://r1.googlevideo.com/a";
        let mut calls=Vec::new();
        let (_,target)=redirected_response(base,|url,_|{calls.push(url.to_string());Ok(if calls.len()==1{fixture_response(302,Some("https://r2.googlevideo.com/b?sig=exact"))}else{fixture_response(206,None)})}).unwrap();
        assert_eq!(target,"https://r2.googlevideo.com/b?sig=exact");assert_eq!(calls.len(),2);
        for location in [None,Some("https://evil.test/x"),Some(base)]{
            let mut calls=0;assert!(redirected_response(base,|_,_|{calls+=1;Ok(fixture_response(302,location))}).is_err());assert_eq!(calls,1);
        }
        let mut calls=0;
        let error=redirected_response(base,|_,_|{calls+=1;Ok(fixture_response(307,Some(&format!("https://r1.googlevideo.com/{calls}"))))}).unwrap_err();
        assert_eq!(error.to_string(),"media_redirect_limit");assert_eq!(calls,4);
        let mut calls=0;let (response,_)=redirected_response(base,|_,_|{calls+=1;Ok(fixture_response(403,None))}).unwrap();assert_eq!(response.status().as_u16(),403);assert_eq!(calls,1);
    }
    struct Counted {calls:Arc<Mutex<Vec<(u64,u64)>>>,tail_fails:bool}
    impl Fetch for Counted {fn fetch(&self,start:u64,end:u64)->io::Result<Block>{
        self.calls.lock().unwrap().push((start,end));
        if self.tail_fails&&start>0{return Err(failure("media_range_http_403"));}
        let total=4*BLOCK as u64;let end=end.min(total-1);
        Ok(Block{total,bytes:(start..=end).map(|n|(n%251) as u8).collect()})
    }}
    #[test]
    fn media3_preflight_checks_tail_reuses_both_probes_and_does_not_hide_later_failures(){
        let calls=Arc::new(Mutex::new(Vec::new()));
        let prepared=PreparedFetch::new(Box::new(Counted{calls:calls.clone(),tail_fails:false}),Some(4*BLOCK as u64)).unwrap();
        assert_eq!(calls.lock().unwrap().len(),2);
        assert_eq!(prepared.fetch(0,99).unwrap().bytes,(0..100).map(|n|(n%251) as u8).collect::<Vec<_>>());
        prepared.fetch(4*BLOCK as u64-100,4*BLOCK as u64-1).unwrap();assert_eq!(calls.lock().unwrap().len(),2);
        prepared.fetch(BLOCK as u64,2*BLOCK as u64-1).unwrap();assert_eq!(calls.lock().unwrap().len(),3);
        assert!(prepared.fetch(0,BLOCK as u64).is_err());
        assert!(PreparedFetch::new(Box::new(Counted{calls:calls.clone(),tail_fails:true}),None).is_err());
    }
    #[test]
    fn long_media_streams_every_block_with_bounded_probe_memory_and_correct_late_offsets(){
        struct Long {calls:Arc<AtomicUsize>}
        const TOTAL:u64=10*1024*1024+123;
        use std::sync::atomic::AtomicUsize;
        impl Fetch for Long {fn fetch(&self,start:u64,end:u64)->io::Result<Block>{
            self.calls.fetch_add(1,Ordering::AcqRel);
            let last=end.min(TOTAL-1);
            Ok(Block{total:TOTAL,bytes:(start..=last).map(|p|(p%251) as u8).collect()})
        }}
        let calls=Arc::new(AtomicUsize::new(0));
        let prepared=PreparedFetch::new(Box::new(Long{calls:calls.clone()}),Some(TOTAL)).unwrap();
        assert_eq!(calls.load(Ordering::Acquire),2);
        let mut start=0;
        while start<TOTAL {
            let end=(start+BLOCK as u64-1).min(TOTAL-1);
            let block=prepared.fetch(start,end).unwrap();
            assert_eq!(block.bytes.len() as u64,end-start+1);
            assert!(block.bytes.iter().enumerate().all(|(i,b)|*b==((start+i as u64)%251) as u8));
            start=end+1;
        }
        assert!(calls.load(Ordering::Acquire)>150,"must cross many later source reads, not just cached first/tail");
        assert!(prepared.first.bytes.len()+prepared.tail.as_ref().unwrap().1.bytes.len()<=2*BLOCK);
        let before=calls.load(Ordering::Acquire);
        prepared.fetch(TOTAL-100,TOTAL-1).unwrap();prepared.fetch(0,99).unwrap();
        assert_eq!(calls.load(Ordering::Acquire),before);
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
