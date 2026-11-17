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

// Research-only anonymous VISIONOS profile reviewed from yt-dlp upstream
// e6275c2b72fe178d54c490c9d775097aa8184457. NOT an APK/client-list change.
const TARGET:&str="Suno Aisha Best Song - Aisha|Sonam Kapoor|Abhay Deol|Javed Akhtar|Amit Trivedi|Ash King";
const VISION_UA:&str="Mozilla/5.0 (Macintosh; Intel Mac OS X 15_7_3) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15";
fn normalized(s:&str)->String{s.chars().filter(|c|c.is_ascii_alphanumeric()).flat_map(char::to_lowercase).collect()}
fn classify(error:&str)->&'static str {
    let s=error.to_ascii_lowercase();
    if s.contains("not a bot")||s.contains("sign in")||s.contains("login_required"){"provider_sign_in_or_bot_challenge"}
    else if s.contains("403"){"http_403"}
    else if s.contains("no longer supported"){"client_unsupported"}
    else if s.contains("no audio"){"no_audio_source"}
    else {"metadata_or_transport_rejected"}
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
    let hits=youtube::search(&client,TARGET,visitor.attached().then_some(visitor.value.as_str()),10).await.map_err(|_|"target_search_failed")?;
    let exact:Vec<_>=hits.iter().filter(|h|normalized(&h.channel)=="sonymusicindiavevo"&&normalized(&h.title)==normalized(TARGET)).collect();
    report["exact_target_matches"]=json!(exact.len());
    if exact.len()!=1{return Err("target_not_uniquely_identified");}
    let target=exact[0];let video_id=target.video_id.clone();
    report["video_id"]=json!(video_id);report["matched_title"]=json!(target.title);report["matched_channel"]=json!(target.channel);
    report["attempts"]=json!([]);
    // One unchanged-production baseline and ONE researched candidate. No loops
    // over guessed profiles, bot challenges, account tokens, or full downloads.
    for candidate in [false,true] {
        let name=if candidate{"VISIONOS_RESEARCH"}else{"ANDROID_BASELINE"};
        let result=if candidate {
            let mut context=json!({"clientName":"VISIONOS","clientVersion":"1.02","deviceMake":"Apple","deviceModel":"RealityDevice17,1",
                "osName":"visionOS","osVersion":"26.5.23O471","userAgent":VISION_UA,"hl":"en","gl":"US"});
            if visitor.attached(){context["visitorData"]=json!(visitor.value);}
            let response=client.post("https://www.youtube.com/youtubei/v1/player?prettyPrint=false")
                .header(reqwest::header::USER_AGENT,VISION_UA).header("X-YouTube-Client-Name","101").header("X-YouTube-Client-Version","1.02")
                .json(&json!({"context":{"client":context},"videoId":video_id})).send().await.map_err(|_|"candidate_api_transport_failed")?;
            if !response.status().is_success(){return Err("candidate_api_http_rejected");}
            let body:Value=response.json().await.map_err(|_|"candidate_json_rejected")?;
            youtube::parse_player_json(&body,&video_id,"VISIONOS")
        }else{youtube::resolve_forced(&client,&video_id,youtube::ANDROID,visitor.attached().then_some(visitor.value.as_str())).await};
        let audio=match result {Ok(audio)=>audio,Err(error)=>{
            let code=classify(&error.to_string());report["attempts"].as_array_mut().unwrap().push(json!({"client":name,"code":code}));
            if code=="provider_sign_in_or_bot_challenge"{return Err("provider_sign_in_or_bot_challenge");}
            continue;
        }};
        let mut shape=json!({});
        if let Ok(url)=reqwest::Url::parse(&audio.url){for key in ["n","pot","range","sig","lsig","signature"]{shape[key]=json!(url.query_pairs().any(|(k,_)|k==key));}}
        let ua=if candidate{VISION_UA}else{youtube::ANDROID.user_agent};let itag=audio.itag;let headers=headers.clone();
        let outcome=tokio::task::spawn_blocking(move||->Result<Value,String>{
            let client=reqwest::blocking::Client::builder().no_proxy().proxy(reqwest::Proxy::https(format!("http://{address}")).map_err(|_|"guard_proxy_failed")?)
                .http1_only().pool_max_idle_per_host(0).default_headers(headers).redirect(reqwest::redirect::Policy::none())
                .connect_timeout(Duration::from_secs(8)).timeout(Duration::from_secs(12)).build().map_err(|_|"media_client_failed")?;
            let expected=audio.content_length;
            let fetch=range_media::HttpFetch::with_client(audio.url,ua,client).map_err(|e|e.to_string())?;
            let first=match fetch.fetch(0,65535){Ok(b)=>b,Err(e)=>return Ok(json!({"client":name,"itag":itag,"url_shape":shape,"phase":"head","code":classify(&e.to_string())}))};
            if expected.is_some_and(|n|n!=first.total)||first.total<131072||first.bytes.len()!=65536{return Err("media_length_rejected".into());}
            if !first.bytes.windows(4).take(32).any(|b|b==b"ftyp"){return Err("media_not_mp4".into());}
            let total=first.total;
            if let Err(error)=fetch.fetch(total-65536,total-1){return Ok(json!({"client":name,"itag":itag,"url_shape":shape,"head_http":206,"phase":"tail","code":classify(&error.to_string())}));}
            let middle=(total/2/65536)*65536;
            if let Err(error)=fetch.fetch(middle,(middle+65535).min(total-1)){return Ok(json!({"client":name,"itag":itag,"url_shape":shape,"head_http":206,"tail_http":206,"phase":"middle","code":classify(&error.to_string())}));}
            Ok(json!({"client":name,"itag":itag,"url_shape":shape,"content_length":total,"head_http":206,"tail_http":206,"middle_http":206,"code":"range_gate_passed"}))
        }).await.map_err(|_|"media_worker_failed")?.map_err(|_|"media_validation_rejected")?;
        let passed=outcome["code"]=="range_gate_passed";
        report["attempts"].as_array_mut().unwrap().push(outcome);
        if candidate&&passed{return Ok(());}
    }
    Err("candidate_did_not_pass_target_ranges")
}
#[tokio::main]
async fn main(){
    // Old public-video allowance stays sealed. A separate exact-parent workflow
    // gate authorizes this screenshot-specific one-off research check only.
    const LIVE_APPROVAL_AVAILABLE:bool=false;
    debug_assert!(!LIVE_APPROVAL_AVAILABLE);
    if std::env::var("AYSHA_TARGET_SOURCE_APPROVAL").as_deref()!=Ok("suno-aisha-one-target")
        ||std::env::var("GITHUB_RUN_ATTEMPT").as_deref()!=Ok("1")
        ||std::env::args().skip(1).collect::<Vec<_>>()!=["--approved-target-once"] {
        println!("{}",json!({"status":"offline","code":"approval_consumed"}));return;
    }
    std::panic::set_hook(Box::new(|_|{}));
    let budget=Arc::new(Budget::new());
    let mut report=json!({"status":"blocked","requested_title":TARGET,"phone_acceptance":false,"complete_playback_test":false,"production_baseline":true,"candidate_in_apk":false,
        "limits":{"http_requests":REQUESTS,"transport_bytes":LIMIT,"seconds":60,"range_operations":6}});
    let result=std::panic::AssertUnwindSafe(tokio::time::timeout(Duration::from_secs(55),check(budget.clone(),&mut report))).catch_unwind().await;
    let code=match result {Ok(Ok(Ok(())))=>{report["status"]=json!("range_gate_passed");"ok"},Ok(Ok(Err(code)))=>code,Ok(Err(_))=>"time_budget_exhausted",Err(_)=>"source_check_panicked"};
    report["code"]=json!(code);report["requests"]=json!(budget.requests.load(Ordering::Acquire));
    report["transport_bytes_reserved_or_used"]=json!(LIMIT-budget.remaining.load(Ordering::Acquire));report["elapsed_ms"]=json!(budget.start.elapsed().as_millis());
    println!("{report}");if code!="ok"{std::process::exit(3);}
}

#[cfg(test)] mod target_tests {
    use super::*;
    #[test] fn target_match_is_exact_except_spacing_and_punctuation(){
        assert_eq!(normalized("SonyMusicIndiaVEVO"),"sonymusicindiavevo");
        assert_ne!(normalized("Sony Music India"),"sonymusicindiavevo");
        assert_eq!(normalized("Suno Aisha | Best Song"),normalized("Suno Aisha Best Song"));
        assert_ne!(normalized("Suno Aisha Lyric Video"),normalized(TARGET));
    }
    #[test] fn provider_challenges_stop_without_body_or_token_output(){
        assert_eq!(classify("Sign in to confirm you're not a bot"),"provider_sign_in_or_bot_challenge");
        assert_eq!(classify("LOGIN_REQUIRED"),"provider_sign_in_or_bot_challenge");
        assert_eq!(classify("media_prepare_tail_failed: media_range_http_403"),"http_403");
    }
}
