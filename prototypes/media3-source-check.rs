//! Bounded CI check of the SAME resolver/range/preflight code used by Media3.
//! Fixed public video, no credentials, no raw URLs/provider bodies, no retry loop.
#![allow(dead_code)]
#[path="../src/youtube.rs"] mod youtube;
#[path="../src/range_media.rs"] mod range_media;
use range_media::Fetch;
use serde_json::{json,Value};
use futures_util::FutureExt;
use std::{io,sync::{Arc,atomic::{AtomicUsize,Ordering}},time::{Duration,Instant}};
use tokio::{io::{AsyncRead,AsyncReadExt,AsyncWrite,AsyncWriteExt},net::{TcpListener,TcpStream}};
const LIMIT:usize=8*1024*1024;
const REQUESTS:usize=16;
const VIDEO:&str="dQw4w9WgXcQ";
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

async fn check(budget:Arc<Budget>,report:&mut Value)->Result<(),&'static str>{
    let listener=TcpListener::bind("127.0.0.1:0").await.map_err(|_|"guard_start_failed")?;
    let address=listener.local_addr().map_err(|_|"guard_address_failed")?;
    tokio::spawn(proxy(listener,budget));
    let mut headers=reqwest::header::HeaderMap::new();
    headers.insert(reqwest::header::CONNECTION,reqwest::header::HeaderValue::from_static("close"));
    let client=reqwest::Client::builder().no_proxy().proxy(reqwest::Proxy::https(format!("http://{address}")).map_err(|_|"guard_proxy_failed")?)
        .http1_only().pool_max_idle_per_host(0).default_headers(headers.clone()).redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(8)).timeout(Duration::from_secs(12)).build().map_err(|_|"api_client_failed")?;
    let visitor=youtube::ensure_visitor(&client,None).await;
    report["visitor_attached"]=json!(visitor.attached());
    report["attempts"]=json!([]);
    for profile in youtube::CLIENTS {
        let resolved=youtube::resolve_forced(&client,VIDEO,profile,visitor.attached().then_some(visitor.value.as_str())).await;
        let audio=match resolved {Ok(audio)=>audio,Err(error)=>{
            let text=error.to_string().to_ascii_lowercase();
            let code=if text.contains("not a bot")||text.contains("sign in"){"provider_sign_in_or_bot_challenge"}
                else if text.contains("no longer supported"){"provider_client_unsupported"}
                else if text.contains("no audio"){"no_audio_source"}else{"metadata_rejected"};
            report["attempts"].as_array_mut().unwrap().push(json!({"client":profile.name,"code":code}));continue;
        }};
        let itag=audio.itag;let mime_aac=audio.mime.starts_with("audio/mp4");
        let headers=headers.clone();
        let outcome=tokio::task::spawn_blocking(move||->Result<Value,String>{
            let client=reqwest::blocking::Client::builder().no_proxy().proxy(reqwest::Proxy::https(format!("http://{address}")).map_err(|_|"guard_proxy_failed")?)
                .http1_only().pool_max_idle_per_host(0).default_headers(headers).redirect(reqwest::redirect::Policy::none())
                .connect_timeout(Duration::from_secs(8)).timeout(Duration::from_secs(12)).build().map_err(|_|"media_client_failed")?;
            let user_agent=audio.user_agent();
            let fetch=range_media::HttpFetch::with_client(audio.url,user_agent,client).map_err(|e|e.to_string())?;
            let prepared=range_media::PreparedFetch::new(Box::new(fetch),audio.content_length).map_err(|e|e.to_string())?;
            let total=prepared.len();let block=range_media::BLOCK as u64;
            let first=prepared.fetch(0,total.min(block)-1).map_err(|e|e.to_string())?;
            if !first.bytes.windows(4).take(32).any(|b|b==b"ftyp"){return Err("media_not_mp4".into());}
            // Two additional spaced blocks exercise the same later-read path
            // as the source that stopped after1–2seconds, not just metadata.
            let mut later=0;
            for start in [block,(total/2/block)*block] {
                if start<total {prepared.fetch(start,(start+block-1).min(total-1)).map_err(|e|e.to_string())?;later+=1;}
            }
            Ok(json!({"client":profile.name,"itag":itag,"aac":mime_aac,"content_length":total,"later_reads":later,"code":"range_gate_passed"}))
        }).await.map_err(|_|"media_worker_failed")?;
        match outcome {
            Ok(result)=>{report["attempts"].as_array_mut().unwrap().push(result);return Ok(());}
            Err(code)=>{
                let safe=if code.len()<80&&code.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'_'){code}else{"media_read_failed".into()};
                report["attempts"].as_array_mut().unwrap().push(json!({"client":profile.name,"itag":itag,"code":safe}));
            }
        }
    }
    Err("all_source_profiles_rejected")
}
#[tokio::main]
async fn main(){
    // Consumed, successful source proof43743729439. All old flags/environment
    // approvals are ignored. Re-enabling requires a reviewed explicit change.
    const LIVE_APPROVAL_AVAILABLE:bool=false;
    if !LIVE_APPROVAL_AVAILABLE {
        println!("{}",json!({"status":"offline","code":"approval_consumed"}));return;
    }
    std::panic::set_hook(Box::new(|_|{}));
    let budget=Arc::new(Budget::new());
    let mut report=json!({"status":"blocked","video_id":VIDEO,"phone_acceptance":false,"complete_playback_test":false,"production_source_code":true,
        "limits":{"http_requests":REQUESTS,"transport_bytes":LIMIT,"seconds":60,"range_operations":12}});
    let result=std::panic::AssertUnwindSafe(tokio::time::timeout(Duration::from_secs(55),check(budget.clone(),&mut report))).catch_unwind().await;
    let code=match result {Ok(Ok(Ok(())))=>{report["status"]=json!("range_gate_passed");"ok"},Ok(Ok(Err(code)))=>code,Ok(Err(_))=>"time_budget_exhausted",Err(_)=>"source_check_panicked"};
    report["code"]=json!(code);report["requests"]=json!(budget.requests.load(Ordering::Acquire));
    report["transport_bytes_reserved_or_used"]=json!(LIMIT-budget.remaining.load(Ordering::Acquire));report["elapsed_ms"]=json!(budget.start.elapsed().as_millis());
    println!("{report}");if code!="ok"{std::process::exit(3);}
}
