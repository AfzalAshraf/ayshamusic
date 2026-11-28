//! Explicitly requested, packaged yt-dlp subprocess. No config, cookies,
//! plugins, remote components, retries, or client rotation. A pinned local
//! Deno runtime is used by yt-dlp's bundled JavaScript component.
use std::{path::PathBuf, process::Stdio, sync::{Arc,atomic::{AtomicU64,Ordering}}, time::Duration};
use tokio::io::AsyncReadExt;

const MAX_BYTES:usize=64*1024*1024;
pub const VERSION:&str="2026.10.26";
pub const DENO_VERSION:&str="2.9.8";
// Prefer an available direct Opus/WebM stream; fall back to compatible AAC.
const AUDIO_SELECTOR:&str="bestaudio[acodec^=opus][ext=webm]/bestaudio[acodec^=mp4a][ext=m4a]/bestaudio[acodec^=mp4a][ext=mp4]";
fn valid_id(id:&str)->bool{id.len()==11 && id.bytes().all(|c|c.is_ascii_alphanumeric()||c==b'_'||c==b'-')}
// Return only fixed codes; stderr may contain private provider data and is never reported.
fn classify(stderr:&str)->String {
    let text=stderr.to_ascii_lowercase();
    let mut codes=Vec::new();
    if ["sign in", "sign-in", "not a bot", "captcha", "login required"].iter().any(|s|text.contains(s)) {codes.push("ytdlp_sign_in_or_bot_challenge");}
    if text.contains("http error 403") || text.contains("forbidden") {codes.push("ytdlp_http_403");}
    if text.contains("requested format is not available") {codes.push("ytdlp_format_unavailable");}
    if text.contains("signature solving failed") || text.contains("n challenge solving failed") {codes.push("ytdlp_js_challenge_failed");}
    if text.contains("no supported javascript runtime") || text.contains("no javascript runtime") || text.contains("no js runtime") {codes.push("ytdlp_js_runtime_unavailable");}
    if text.contains("only images are available for download") {codes.push("ytdlp_images_only");}
    if text.contains("formats require a gvs po token") {codes.push("ytdlp_formats_require_token");}
    if text.contains("formats have been skipped as they are missing a url") {codes.push("ytdlp_formats_missing_url");}
    if text.contains("formats have been skipped as they are drm protected") {codes.push("ytdlp_formats_drm_protected");}
    if codes.is_empty(){codes.push("ytdlp_fetch_failed");}
    codes.join(", ")
}
fn bundled_executable(name:&str,error:&'static str)->Result<PathBuf,&'static str> {
    let exe=std::env::current_exe().map_err(|_|error)?;
    let path=exe.parent().ok_or(error)?.join(name);
    if !path.is_file(){return Err(error);} Ok(path)
}

pub async fn fetch(id:&str,generation:Arc<AtomicU64>,expected:u64)->Result<Vec<u8>,String> {
    if !valid_id(id){return Err("invalid_video_id".into());}
    let exe=bundled_executable("yt-dlp.exe","ytdlp_missing").map_err(str::to_owned)?;
    let runtime=bundled_executable("deno.exe","ytdlp_js_runtime_missing").map_err(str::to_owned)?;
    let runtime_arg=format!("deno:{}",runtime.to_str().ok_or("ytdlp_js_runtime_missing")?);
    let mut command=tokio::process::Command::new(exe);
    command.args(["--ignore-config","--no-plugin-dirs","--no-cache-dir","--no-update","--no-playlist","--no-progress","--proxy","","--no-remote-components","--no-js-runtimes","--js-runtimes",&runtime_arg,"--no-check-formats","--retries","0","--fragment-retries","0","--extractor-retries","0","--socket-timeout","15","--max-filesize","64M","--extractor-args","youtube:player_client=web","--extractor-args","youtube-ejs:jitless=true","-f",AUDIO_SELECTOR,"-o","-","--",&format!("https://www.youtube.com/watch?v={id}")]);
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
        if !status.success(){return Err(classify(&err));}
        let mp4=bytes.len()>=12 && &bytes[4..8]==b"ftyp";
        let webm=bytes.len()>=4 && bytes[..4]==[0x1a,0x45,0xdf,0xa3];
        if !mp4 && !webm {return Err("downloaded_audio_invalid".into());}
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
        let result=classify("WARNING: Signature solving failed: Some formats may be missing. https://private.test/secret\nERROR: Requested format is not available");
        assert_eq!(result,"ytdlp_format_unavailable, ytdlp_js_challenge_failed");
        assert!(!result.contains("private"));
        assert_eq!(classify("WARNING: Only images are available for download. Check --list-formats\nERROR: Requested format is not available"),"ytdlp_format_unavailable, ytdlp_images_only");
        assert!(classify("WARNING: web client https formats require a GVS PO Token which was not provided.").contains("ytdlp_formats_require_token"));
        assert!(classify("WARNING: Some web client https formats have been skipped as they are missing a URL.").contains("ytdlp_formats_missing_url"));
        assert!(classify("WARNING: Some web client https formats have been skipped as they are DRM protected.").contains("ytdlp_formats_drm_protected"));
    }
}
