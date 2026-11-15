//! One explicitly approved CI source experiment, never an automatic live test.
//! No accounts, cookies, supplied tokens, downloaded files or external executables.
use futures_util::FutureExt;
use rusty_ytdl::{reqwest,RequestOptions,Video,VideoOptions};
use serde_json::{json,Value};
use std::{io,sync::{Arc,atomic::{AtomicUsize,Ordering}},time::{Duration,Instant}};
use tokio::{io::{AsyncRead,AsyncReadExt,AsyncWrite,AsyncWriteExt},net::{TcpListener,TcpStream}};

// The approved experiment was consumed by run43737916216. Re-enabling this
// requires a new explicit user approval; even --approved-once is now offline.
const LIVE_APPROVAL_AVAILABLE:bool=false;
const LIMIT:usize=8*1024*1024;
const REQUESTS:usize=12;
const VIDEO:&str="dQw4w9WgXcQ";
const REV:&str="b390352a5db4b57aa630e8dc7b427fb560feccd5";
const UA:&str="com.google.android.apps.youtube.vr.oculus/1.71.26 (Linux; U; Android 12L; eureka-user Build/SQ3A.220605.009.A1) gzip";
struct Budget {start:Instant,requests:AtomicUsize,remaining:AtomicUsize}
impl Budget {
    fn new()->Self{Self{start:Instant::now(),requests:AtomicUsize::new(0),remaining:AtomicUsize::new(LIMIT)}}
    fn request(&self)->io::Result<()> {
        if self.start.elapsed()>=Duration::from_secs(55){return Err(io::Error::other("time_budget"));}
        self.requests.fetch_update(Ordering::AcqRel,Ordering::Acquire,|n|(n<REQUESTS).then_some(n+1)).map_err(|_|io::Error::other("request_budget"))?;Ok(())
    }
    // Reserve BEFORE each read: even concurrent encrypted transfers cannot
    // receive/send more than the total8MiB transport cap. Refund unread bytes.
    fn reserve(&self,max:usize)->io::Result<usize>{
        let old=self.remaining.fetch_update(Ordering::AcqRel,Ordering::Acquire,|n|(n>0).then_some(n-n.min(max))).map_err(|_|io::Error::other("byte_budget"))?;
        Ok(old.min(max))
    }
}
fn authority(raw:&str)->Option<&str>{
    let host=raw.strip_suffix(":443")?;
    if !host.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'.'||b==b'-'){return None;}
    (host=="youtube.com"||host.ends_with(".youtube.com")||host=="googlevideo.com"||host.ends_with(".googlevideo.com")).then_some(host)
}
async fn relay<R:AsyncRead+Unpin,W:AsyncWrite+Unpin>(mut read:R,mut write:W,budget:Arc<Budget>)->io::Result<()> {
    let mut buffer=[0u8;4096];
    loop {
        let reserved=budget.reserve(buffer.len())?;
        let result=read.read(&mut buffer[..reserved]).await;
        let n=match result {Ok(n)=>n,Err(e)=>{budget.remaining.fetch_add(reserved,Ordering::AcqRel);return Err(e);}};
        budget.remaining.fetch_add(reserved-n,Ordering::AcqRel);
        if n==0{write.shutdown().await?;return Ok(());}
        write.write_all(&buffer[..n]).await?;
    }
}
async fn tunnel(mut socket:TcpStream,budget:Arc<Budget>)->io::Result<()> {
    let mut header=Vec::new();
    // Read CONNECT one byte at a time so no already-received TLS bytes are lost.
    while !header.ends_with(b"\r\n\r\n") {
        if header.len()>=4096{return Err(io::Error::other("proxy_header_limit"));}
        header.push(socket.read_u8().await?);
    }
    let text=std::str::from_utf8(&header).map_err(|_|io::Error::other("proxy_header"))?;
    let mut parts=text.lines().next().unwrap_or("").split_whitespace();
    if parts.next()!=Some("CONNECT"){return Err(io::Error::other("https_only"));}
    let host=authority(parts.next().unwrap_or("")).ok_or_else(||io::Error::other("host_rejected"))?;
    budget.request()?;
    let upstream=tokio::time::timeout(Duration::from_secs(8),TcpStream::connect((host,443))).await.map_err(|_|io::Error::other("connect_timeout"))??;
    socket.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await?;
    let (client_read,client_write)=socket.into_split();let (up_read,up_write)=upstream.into_split();
    tokio::try_join!(relay(client_read,up_write,budget.clone()),relay(up_read,client_write,budget))?;Ok(())
}
async fn proxy(listener:TcpListener,budget:Arc<Budget>){
    loop {let Ok((socket,_))=listener.accept().await else{return;};let budget=budget.clone();
        tokio::spawn(async move {let _=tokio::time::timeout(Duration::from_secs(55),tunnel(socket,budget)).await;});}
}
// Controlled categories only: never print a signed URL, hostname, query or token.
fn media_url_class(raw:&str)->&'static str {
    if raw.is_empty(){return "empty";}
    let Ok(url)=reqwest::Url::parse(raw)else{return "unparseable";};
    if url.scheme()!="https"{return "non_https";}
    if !url.username().is_empty()||url.password().is_some(){return "credentials_rejected";}
    if url.port().is_some_and(|p|p!=443){return "port_rejected";}
    if !url.host_str().is_some_and(|h|h=="googlevideo.com"||h.ends_with(".googlevideo.com")){return "host_rejected";}
    "allowed"
}
fn allowed_media(raw:&str)->bool{media_url_class(raw)=="allowed"}
fn is_aac(f:&rusty_ytdl::VideoFormat)->bool {
    f.has_audio&&!f.has_video&&!f.is_live&&!f.is_hls&&!f.is_dash_mpd
        &&f.mime_type.mime.to_string().starts_with("audio/mp4")
        &&f.mime_type.codecs.iter().any(|c|c.starts_with("mp4a"))
}
fn select_aac(formats:&[rusty_ytdl::VideoFormat])->Option<&rusty_ytdl::VideoFormat>{
    // An unusable preferred/high-bitrate format must not hide a usable AAC URL.
    // This selects locally from ONE response, never probes additional profiles.
    formats.iter().filter(|f|is_aac(f)&&allowed_media(&f.url))
        .max_by_key(|f|(f.itag==140,f.bitrate))
}
async fn range(client:&reqwest::Client,url:&str,start:u64,end:u64,total:u64,report:&mut Value,key:&str)->Result<Vec<u8>,&'static str>{
    let response=client.get(url).header(reqwest::header::USER_AGENT,UA).header(reqwest::header::ACCEPT_ENCODING,"identity")
        .header(reqwest::header::RANGE,format!("bytes={start}-{end}")).send().await.map_err(|_|"media_transport_failed")?;
    let status=response.status().as_u16();report[key]=json!({"http":status,"start":start,"end":end});
    if status!=206{return Err("media_http_rejected");}
    let expected=format!("bytes {start}-{end}/{total}");
    if response.headers().get(reqwest::header::CONTENT_RANGE).and_then(|h|h.to_str().ok())!=Some(expected.as_str()) {return Err("media_range_mismatch");}
    let mime=response.headers().get(reqwest::header::CONTENT_TYPE).and_then(|h|h.to_str().ok()).unwrap_or("").split(';').next().unwrap_or("");
    if !["audio/mp4","video/mp4","application/octet-stream"].contains(&mime){return Err("media_type_rejected");}
    if response.headers().get(reqwest::header::CONTENT_ENCODING).is_some_and(|v|v!="identity"){return Err("media_encoding_rejected");}
    let mut response=response;let mut bytes=Vec::new();
    while let Some(chunk)=response.chunk().await.map_err(|_|"media_read_failed")?{
        if bytes.len()+chunk.len()>65536{return Err("media_range_too_large");}bytes.extend_from_slice(&chunk);
    }
    if bytes.len() as u64!=end-start+1{return Err("media_range_length_mismatch");}
    report[key]["bytes"]=json!(bytes.len());Ok(bytes)
}
async fn experiment(budget:Arc<Budget>,report:&mut Value)->Result<(),&'static str>{
    let listener=TcpListener::bind("127.0.0.1:0").await.map_err(|_|"guard_start_failed")?;
    let address=listener.local_addr().map_err(|_|"guard_address_failed")?;
    tokio::spawn(proxy(listener,budget));
    // Strict HTTP/1 + Connection:close + zero idle pool ensures each guarded
    // CONNECT corresponds to at most one HTTP request. No redirects or retries.
    let mut headers=reqwest::header::HeaderMap::new();
    headers.insert(reqwest::header::CONNECTION,reqwest::header::HeaderValue::from_static("close"));
    let client=reqwest::Client::builder().no_proxy().proxy(reqwest::Proxy::https(format!("http://{address}")).map_err(|_|"guard_proxy_failed")?)
        .http1_only().pool_max_idle_per_host(0).default_headers(headers).redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(8)).timeout(Duration::from_secs(20)).build().map_err(|_|"http_client_failed")?;
    let options=VideoOptions{request_options:RequestOptions{client:Some(client.clone()),max_retries:Some(0),cookies:None,po_token:None,..Default::default()},..Default::default()};
    report["stage"]=json!("candidate_metadata");
    let video=Video::new_with_options(VIDEO,options).map_err(|_|"candidate_initialization_failed")?;
    // This calls only the Rust extractor's info path. No stream/download/ffmpeg
    // method, login, caller-supplied token, process or format fallback is invoked.
    let info=video.get_basic_info().await.map_err(|error|match error{
        rusty_ytdl::VideoError::VideoPlayerResponseError(ref reason) if reason.to_ascii_lowercase().contains("not a bot")=>"provider_sign_in_or_bot_challenge",
        _=>"candidate_extraction_failed",
    })?;
    if info.video_details.video_id!=VIDEO{return Err("candidate_video_id_mismatch");}
    let aac:Vec<_>=info.formats.iter().filter(|f|is_aac(f)).collect();
    report["format_count"]=json!(info.formats.len());
    report["aac_count"]=json!(aac.len());
    report["aac_url_categories"]=json!(aac.iter().take(16).map(|f|json!({
        "itag":f.itag,"url_class":media_url_class(&f.url),
        "has_content_length":f.content_length.is_some()
    })).collect::<Vec<_>>());
    if aac.is_empty(){return Err("candidate_no_aac");}
    let format=select_aac(&info.formats).ok_or("candidate_no_allowed_aac_url")?;
    let total=format.content_length.as_ref().and_then(|v|v.parse::<u64>().ok()).filter(|n|(131072..=512*1024*1024).contains(n)).ok_or("candidate_length_rejected")?;
    report["itag"]=json!(format.itag);report["content_length"]=json!(total);
    report["stage"]=json!("first_range");
    let first=range(&client,&format.url,0,65535,total,report,"first_range").await?;
    if !first.windows(4).take(32).any(|b|b==b"ftyp"){return Err("candidate_not_mp4");}
    report["stage"]=json!("seek_range");
    let _=range(&client,&format.url,total-65536,total-1,total,report,"seek_range").await?;
    report["stage"]=json!("range_gate_passed");Ok(())
}
#[tokio::main(flavor="current_thread")]
async fn main(){
    debug_assert!(!LIVE_APPROVAL_AVAILABLE);
    let renewed = std::env::var("AYSHA_SOURCE_RECHECK_APPROVAL").as_deref()==Ok("image9-one-public-video")
        &&std::env::var("GITHUB_RUN_ATTEMPT").as_deref()==Ok("1")
        &&std::env::args().skip(1).collect::<Vec<_>>()==["--approved-recheck-once"];
    if !renewed {
        println!("{}",json!({"status":"offline","code":"approval_consumed"}));return;
    }
    // Third-party error messages/panic payloads can include signed URLs. Never
    // print them. Only controlled classifications/counts reach the CI artifact.
    std::panic::set_hook(Box::new(|_|{}));
    let budget=Arc::new(Budget::new());
    let mut report=json!({"status":"blocked","video_id":VIDEO,"candidate":"rusty_ytdl_PR62_research_only","revision":REV,"limits":{"http_requests":REQUESTS,"transport_bytes":LIMIT,"seconds":60,"media_ranges":2},"phone_acceptance":false,"apk_integration":false});
    let result=std::panic::AssertUnwindSafe(tokio::time::timeout(Duration::from_secs(55),experiment(budget.clone(),&mut report))).catch_unwind().await;
    let code=match result{Ok(Ok(Ok(())))=>{report["status"]=json!("range_gate_passed");"ok"},Ok(Ok(Err(code)))=>code,Ok(Err(_))=>"time_budget_exhausted",Err(_)=>"candidate_panicked"};
    report["code"]=json!(code);report["requests"]=json!(budget.requests.load(Ordering::Acquire));
    report["transport_bytes_reserved_or_used"]=json!(LIMIT-budget.remaining.load(Ordering::Acquire));
    report["elapsed_ms"]=json!(budget.start.elapsed().as_millis());
    println!("{report}");
    if code!="ok"{std::process::exit(3);}
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn guard_refuses_untrusted_hosts_accounts_non443_and_credentials(){
        for raw in ["localhost:443","youtube.com.evil.test:443","127.0.0.1:443","accounts.google.com:443","www.youtube.com:80","user@www.youtube.com:443"]{assert!(authority(raw).is_none());}
        assert_eq!(authority("www.youtube.com:443"),Some("www.youtube.com"));assert!(authority("r1---sn.googlevideo.com:443").is_some());
        assert!(allowed_media("https://r1---sn.googlevideo.com/videoplayback?expire=123&itag=140"));
        assert!(!LIVE_APPROVAL_AVAILABLE,"the consumed one-off permission must stay disabled");
        assert_eq!(media_url_class(""),"empty");
        assert_eq!(media_url_class("url=https%3A%2F%2Fr.googlevideo.com"),"unparseable");
        assert_eq!(media_url_class("http://r.googlevideo.com"),"non_https");
        assert_eq!(media_url_class("https://r.googlevideo.com:444/a"),"port_rejected");
        assert!(!allowed_media("https://www.youtube.com/watch"));assert!(!allowed_media("https://u:p@r.googlevideo.com/a"));
    }
    #[test]fn budgets_are_hard_bounds(){let budget=Budget::new();for _ in 0..REQUESTS{budget.request().unwrap();}assert!(budget.request().is_err());let n=budget.reserve(LIMIT+1).unwrap();assert_eq!(n,LIMIT);assert!(budget.reserve(1).is_err());}
}
