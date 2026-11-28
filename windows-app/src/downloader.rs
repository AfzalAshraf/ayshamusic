//! Explicitly requested, packaged yt-dlp subprocess. No config, cookies,
//! plugins, remote components, JS runtimes, retries, or client rotation.
use std::{path::PathBuf, process::Stdio, sync::{Arc,atomic::{AtomicU64,Ordering}}, time::Duration};
use tokio::io::AsyncReadExt;

const MAX_BYTES:usize=64*1024*1024;
pub const VERSION:&str="2026.10.26";
// AAC without assuming yt-dlp's extension label is always m4a.
const AUDIO_SELECTOR:&str="bestaudio[acodec^=mp4a][ext=m4a]/bestaudio[acodec^=mp4a][ext=mp4]";
fn valid_id(id:&str)->bool{id.len()==11 && id.bytes().all(|c|c.is_ascii_alphanumeric()||c==b'_'||c==b'-')}
fn classify(stderr:&str)->&'static str {
    let text=stderr.to_ascii_lowercase();
    if ["sign in", "sign-in", "not a bot", "captcha", "login required"].iter().any(|s|text.contains(s)) {"ytdlp_sign_in_or_bot_challenge"}
    else if text.contains("http error 403") || text.contains("forbidden") {"ytdlp_http_403"}
    else if text.contains("requested format is not available") {"ytdlp_format_unavailable"}
    else if text.contains("javascript runtime") || text.contains("js runtime") {"ytdlp_js_runtime_unavailable"}
    else {"ytdlp_fetch_failed"}
}
fn executable()->Result<PathBuf,&'static str> {
    let exe=std::env::current_exe().map_err(|_|"ytdlp_missing")?;
    let path=exe.parent().ok_or("ytdlp_missing")?.join("yt-dlp.exe");
    if !path.is_file(){return Err("ytdlp_missing");} Ok(path)
}

pub async fn fetch(id:&str,generation:Arc<AtomicU64>,expected:u64)->Result<Vec<u8>,String> {
    if !valid_id(id){return Err("invalid_video_id".into());}
    let exe=executable().map_err(str::to_owned)?;
    let mut command=tokio::process::Command::new(exe);
    command.args(["--ignore-config","--no-plugin-dirs","--no-cache-dir","--no-update","--no-playlist","--no-progress","--no-warnings","--proxy","","--no-remote-components","--no-js-runtimes","--no-check-formats","--retries","0","--fragment-retries","0","--extractor-retries","0","--socket-timeout","15","--max-filesize","64M","--extractor-args","youtube:player_client=web","-f",AUDIO_SELECTOR,"-o","-","--",&format!("https://www.youtube.com/watch?v={id}")]);
    command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    #[cfg(windows)] {command.creation_flags(0x08000000);}
    let mut child=command.spawn().map_err(|_|"ytdlp_start_failed".to_string())?;
    let mut stdout=child.stdout.take().ok_or("ytdlp_start_failed")?;
    let mut stderr=child.stderr.take().ok_or("ytdlp_start_failed")?;
    // Drain all stderr to avoid a blocked child, but retain only 32 KB.
    let err_task=tokio::spawn(async move {let mut saved=Vec::new();let mut buf=[0u8;4096];while let Ok(n)=stderr.read(&mut buf).await {if n==0{break;}let keep=n.min(32768usize.saturating_sub(saved.len()));saved.extend_from_slice(&buf[..keep]);}String::from_utf8_lossy(&saved).into_owned()});
    let result=tokio::time::timeout(Duration::from_secs(180),async {
        let mut bytes=Vec::new();let mut buf=[0u8;32*1024];
        loop {
            if generation.load(Ordering::SeqCst)!=expected{return Err("play_cancelled".to_owned());}
            let n=tokio::select! {
                r=stdout.read(&mut buf)=>r.map_err(|_|"ytdlp_read_failed".to_string())?,
                _=tokio::time::sleep(Duration::from_millis(200))=>continue,
            };
            if n==0 {break;}
            if bytes.len()+n>MAX_BYTES{return Err("ytdlp_size_limit".into());}
            bytes.extend_from_slice(&buf[..n]);
        }
        let status=child.wait().await.map_err(|_|"ytdlp_fetch_failed".to_string())?;
        let err=err_task.await.unwrap_or_default();
        if !status.success(){return Err(classify(&err).into());}
        if bytes.len()<12 || &bytes[4..8]!=b"ftyp" {return Err("downloaded_audio_invalid".into());}
        Ok(bytes)
    }).await;
    match result {Ok(value)=>value,Err(_)=>Err("ytdlp_timeout".into())}
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn validated_ids_and_classification_do_not_expose_messages() {
        assert!(valid_id("dQw4w9WgXcQ"));assert!(!valid_id("../bad"));assert!(!valid_id("dQw4w9WgXc/"));
        assert_eq!(classify("ERROR: Sign in to confirm you’re not a bot"),"ytdlp_sign_in_or_bot_challenge");
        assert_eq!(classify("ERROR: HTTP Error 403: Forbidden"),"ytdlp_http_403");
        assert_eq!(classify("secret https://private.example/token"),"ytdlp_fetch_failed");
    }
}
