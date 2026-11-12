//! Bounded public-player feasibility probe. No downloader library, JS engine or subprocess.
//! Does not solve signatures/tokens or bypass playability restrictions. Never production.
use reqwest::{Client, Url};
use serde::Deserialize;
use serde_json::json;
use std::{fs, time::{Duration, Instant}};
const IDENTITY: &str = "aysha-direct-v1-android-vr";
const VERSION: &str = "1.65.10";
const AGENT: &str = "com.google.android.apps.youtube.vr.oculus/1.65.10 (Linux; U; Android 12L; eureka-user Build/SQ3A.220605.009.A1) gzip";
const METADATA_MAX: usize = 1024 * 1024;
const SAMPLE: usize = 65536;
#[derive(Default, Deserialize)]
#[serde(rename_all="camelCase")]
struct Player {
    playability_status: Option<Playability>,
    streaming_data: Option<Streams>,
    video_details: Option<Details>,
}
#[derive(Deserialize)]
struct Playability { status: String }
#[derive(Default, Deserialize)]
#[serde(rename_all="camelCase")]
struct Streams {
    #[serde(default)] adaptive_formats: Vec<Format>,
}
#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
struct Details {
    video_id: String,
    #[serde(default)] is_live_content: bool,
    #[serde(default)] is_live: bool,
}
#[derive(Default, Deserialize)]
#[serde(rename_all="camelCase")]
struct Format {
    #[serde(default)] itag: u64,
    #[serde(default)] bitrate: u64,
    #[serde(default)] mime_type: String,
    #[serde(default)] url: String,
    #[serde(default)] signature_cipher: String,
    #[serde(default)] cipher: String,
    width: Option<u64>,
    height: Option<u64>,
}
fn valid_id(id: &str) -> bool {
    id.len() == 11 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
fn media_url(value: &str) -> bool {
    let Ok(url) = Url::parse(value) else { return false };
    url.scheme() == "https" && url.username().is_empty() && url.password().is_none()
        && url.port().is_none() && url.host_str().is_some_and(|h| h.ends_with(".googlevideo.com"))
}
fn aac(f: &Format) -> bool {
    f.width.is_none() && f.height.is_none()
        && f.mime_type.split(';').next().is_some_and(|m| m.trim() == "audio/mp4")
        && f.mime_type.split(';').skip(1).any(|p| p.trim() == "codecs=\"mp4a.40.2\"")
}
fn select<'a>(player: &'a Player, id: &str) -> Result<&'a Format, &'static str> {
    if player.playability_status.as_ref().map(|s| s.status.as_str()) != Some("OK") {
        return Err("player_not_playable");
    }
    let details = player.video_details.as_ref().ok_or("video_identity_missing")?;
    if details.video_id != id { return Err("video_identity_mismatch"); }
    if details.is_live || details.is_live_content { return Err("live_unsupported"); }
    let formats = &player.streaming_data.as_ref().ok_or("streams_missing")?.adaptive_formats;
    if formats.len() > 256 { return Err("format_limit"); }
    let candidates: Vec<_> = formats.iter().filter(|f| aac(f)).collect();
    if candidates.is_empty() { return Err("aac_unavailable"); }
    if let Some(best) = candidates.iter().filter(|f| media_url(&f.url)).max_by_key(|f| f.bitrate) {
        return Ok(best);
    }
    if candidates.iter().any(|f| !f.signature_cipher.is_empty() || !f.cipher.is_empty()) {
        return Err("signature_required_unsupported");
    }
    if candidates.iter().all(|f| f.url.is_empty()) { return Err("all_aac_urls_missing"); }
    Err("media_origin_rejected")
}
fn append_bounded(buffer: &mut Vec<u8>, chunk: &[u8], max: usize) -> Result<(), &'static str> {
    if chunk.len() > max.saturating_sub(buffer.len()) { return Err("metadata_size_limit"); }
    buffer.extend_from_slice(chunk);
    Ok(())
}
fn mp4(bytes: &[u8]) -> bool { bytes.len() >= 8 && &bytes[4..8] == b"ftyp" }
fn peak_kib() -> Option<u64> {
    fs::read_to_string("/proc/self/status").ok()?.lines().find_map(|line| {
        line.strip_prefix("VmHWM:")?.split_whitespace().next()?.parse().ok()
    })
}
async fn probe(id: &str) -> Result<serde_json::Value, &'static str> {
    let started = Instant::now();
    let client = Client::builder().https_only(true).no_proxy()
        .redirect(reqwest::redirect::Policy::none()).connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(20)).build().map_err(|_| "client_init_failed")?;
    // Public player request only; no auth, visitor tokens, cookies or restriction flags.
    let body = json!({"context":{"client":{
        "clientName":"ANDROID_VR", "clientVersion":VERSION, "deviceMake":"Oculus",
        "deviceModel":"Quest 3", "androidSdkVersion":32, "userAgent":AGENT,
        "osName":"Android", "osVersion":"12L", "hl":"en"}}, "videoId":id});
    let mut response = client.post("https://www.youtube.com/youtubei/v1/player?prettyPrint=false")
        .header("User-Agent", AGENT).header("X-Youtube-Client-Name", "28")
        .header("X-Youtube-Client-Version", VERSION).json(&body).send().await.map_err(|_| "player_request_failed")?;
    if !response.status().is_success() { return Err("player_http_failed"); }
    if response.content_length().is_some_and(|len| len > METADATA_MAX as u64) { return Err("metadata_size_limit"); }
    let mut bytes = Vec::with_capacity(32768);
    while let Some(chunk) = response.chunk().await.map_err(|_| "player_read_failed")? {
        append_bounded(&mut bytes, &chunk, METADATA_MAX)?;
    }
    let player: Player = serde_json::from_slice(&bytes).map_err(|_| "player_json_invalid")?;
    drop(bytes);
    let format = select(&player, id)?;
    let resolve_ms = started.elapsed().as_millis();
    // No redirects; selected URL is origin-validated and never printed.
    let mut media = client.get(&format.url).header("Range", "bytes=0-65535").send().await.map_err(|_| "media_request_failed")?;
    if !media.status().is_success() { return Err("media_http_failed"); }
    let mut count = 0;
    let mut prefix = Vec::with_capacity(8);
    let mut first_byte_ms = None;
    while let Some(chunk) = media.chunk().await.map_err(|_| "media_read_failed")? {
        if chunk.is_empty() { continue; }
        first_byte_ms.get_or_insert_with(|| started.elapsed().as_millis());
        let take = (8 - prefix.len()).min(chunk.len());
        prefix.extend_from_slice(&chunk[..take]);
        if prefix.len() == 8 && !mp4(&prefix) { return Err("invalid_mp4_header"); }
        count += chunk.len().min(SAMPLE - count);
        if count == SAMPLE { break; }
    }
    if count != SAMPLE || !mp4(&prefix) { return Err("short_or_invalid_probe"); }
    Ok(json!({"status":"pass", "candidate":IDENTITY, "bytes":count, "itag":format.itag,
        "resolve_ms":resolve_ms, "first_byte_ms":first_byte_ms, "total_ms":started.elapsed().as_millis(),
        "peak_rss_kib":peak_kib(), "external_tools":false, "production_ready":false}))
}
#[tokio::main(flavor="current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).take(4).collect();
    if args == ["--help"] {
        println!("aysha-direct-prototype --video-id ID\nExperimental public AAC 64KiB probe only. No JS/signature/token support, credentials, fallback, playback or production integration. Fixed 60s deadline; external timeout recommended.");
        return;
    }
    if args.len() != 2 || args[0] != "--video-id" || !valid_id(&args[1]) {
        println!("{}", json!({"status":"fail", "candidate":IDENTITY, "code":"invalid_arguments"}));
        std::process::exit(2);
    }
    let result = tokio::time::timeout(Duration::from_secs(60), probe(&args[1])).await;
    match result {
        Ok(Ok(metrics)) => println!("{metrics}"),
        failure => {
            let code = match failure { Ok(Err(code)) => code, _ => "deadline_exceeded" };
            println!("{}",json!({"status":"fail", "candidate":IDENTITY, "code":code,
                "peak_rss_kib":peak_kib(), "production_ready":false}));
            std::process::exit(1);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(formats: serde_json::Value) -> Player {
        serde_json::from_value(json!({"playabilityStatus":{"status":"OK"},
            "videoDetails":{"videoId":"Jm7Satp7Zqk"}, "streamingData":{"adaptiveFormats":formats}})).unwrap()
    }
    fn format(url: &str, bitrate: u64) -> serde_json::Value {
        json!({"itag":140, "mimeType":"audio/mp4; codecs=\"mp4a.40.2\"", "bitrate":bitrate,"url":url})
    }
    #[test] fn ids_and_origins_fail_closed() {
        assert!(valid_id("Jm7Satp7Zqk")); assert!(!valid_id("--exec=evil"));
        assert!(!media_url("https://googlevideo.com.evil.test/a"));
        assert!(!media_url("http://x.googlevideo.com/a"));
        assert!(!media_url("https://u@x.googlevideo.com/a"));
        assert!(!media_url("https://x.googlevideo.com:444/a"));
        assert!(!media_url("https://127.0.0.1/a"));
        assert!(media_url("https://x.googlevideo.com/a"));
    }
    #[test] fn choose_valid_aac_not_empty_best() {
        let player = fixture(json!([format("",256000),format("https://x.googlevideo.com/a",128000)]));
        assert_eq!(select(&player,"Jm7Satp7Zqk").unwrap().bitrate,128000);
    }
    #[test] fn missing_and_cipher_are_not_success() {
        let player = fixture(json!([format("",128000)]));
        assert_eq!(select(&player,"Jm7Satp7Zqk").err(),Some("all_aac_urls_missing"));
        let mut f = format("",128000); f["signatureCipher"] = json!("private-not-logged");
        let player = fixture(json!([f]));
        assert_eq!(select(&player,"Jm7Satp7Zqk").err(),Some("signature_required_unsupported"));
    }
    #[test] fn restrictions_identity_and_live_are_not_bypassed() {
        let mut player = fixture(json!([format("https://x.googlevideo.com/a",128000)]));
        assert_eq!(select(&player,"aaaaaaaaaaa").err(),Some("video_identity_mismatch"));
        player.video_details.as_mut().unwrap().is_live_content = true;
        assert_eq!(select(&player,"Jm7Satp7Zqk").err(),Some("live_unsupported"));
        player.playability_status.as_mut().unwrap().status = "LOGIN_REQUIRED".into();
        assert_eq!(select(&player,"Jm7Satp7Zqk").err(),Some("player_not_playable"));
    }
    #[test] fn metadata_limits_and_container() {
        let mut bytes = Vec::new();
        assert!(append_bounded(&mut bytes,b"1234",4).is_ok());
        assert_eq!(append_bounded(&mut bytes,b"5",4),Err("metadata_size_limit"));
        assert_eq!(bytes,b"1234");
        assert!(mp4(b"\0\0\0\x20ftyp")); assert!(!mp4(b"<html>bad"));
    }
    #[test] fn unrelated_or_too_many_formats_fail() {
        let mut f = format("https://x.googlevideo.com/a",128000); f["width"]=json!(1920);
        assert_eq!(select(&fixture(json!([f])),"Jm7Satp7Zqk").err(),Some("aac_unavailable"));
        let f = format("https://x.googlevideo.com/a",128000);
        assert_eq!(select(&fixture(json!(vec![f;257])),"Jm7Satp7Zqk").err(),Some("format_limit"));
    }
}
