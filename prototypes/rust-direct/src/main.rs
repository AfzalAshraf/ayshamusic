//! Bounded public-player feasibility probe. No downloader library, JS engine or subprocess.
//! Does not solve signatures/tokens or bypass playability restrictions. Never production.
mod bootstrap;
mod literal_data;
use reqwest::{Client, Url};
use serde::Deserialize;
use serde_json::json;
use std::{fs, time::{Duration, Instant}};
const IDENTITY: &str = "aysha-direct-v2-visionos";
const STANDARD_IDENTITY: &str = "aysha-direct-v3-standard-context";
const BOOTSTRAP_IDENTITY: &str = "aysha-direct-v5-data-literals";
const INSPECT_IDENTITY: &str = "aysha-direct-v5-watch-inspect";
const VERSION: &str = "1.02";
const AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 15_7_3) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15";
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
struct Playability {
    status: String,
    #[serde(default)] reason: String,
    #[serde(default)] messages: Vec<String>,
}
// Return fixed classifications only, never upstream reason/status/message strings.
fn playability(player: &Player) -> Result<(), &'static str> {
    let status = player.playability_status.as_ref().ok_or("player_status_missing")?;
    if status.status == "OK" { return Ok(()); }
    let bot_signal = |text: &str| {
        let text = text.to_ascii_lowercase();
        text.contains("not a bot") || text.contains("confirm you're not a bot")
    };
    if bot_signal(&status.reason) || status.messages.iter().any(|s| bot_signal(s)) {
        return Err("player_bot_check");
    }
    Err(match status.status.as_str() {
        "LOGIN_REQUIRED" => "player_login_required",
        "AGE_CHECK_REQUIRED" | "AGE_VERIFICATION_REQUIRED" => "player_age_check_required",
        "CONTENT_CHECK_REQUIRED" => "player_content_check_required",
        "UNPLAYABLE" => "player_unplayable",
        "LIVE_STREAM_OFFLINE" => "player_live_offline",
        "ERROR" => "player_error",
        _ => "player_status_unknown",
    })
}
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
fn identity(player: &Player, id: &str) -> Result<(), &'static str> {
    let details = player.video_details.as_ref().ok_or("video_identity_missing")?;
    if details.video_id != id { return Err("video_identity_mismatch"); }
    if details.is_live || details.is_live_content { return Err("live_unsupported"); }
    Ok(())
}
fn select<'a>(player: &'a Player, id: &str) -> Result<&'a Format, &'static str> {
    playability(player)?;
    identity(player,id)?;
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
fn validate_sample(count: usize, prefix: &[u8]) -> Result<(), &'static str> {
    if count != SAMPLE || !mp4(prefix) { return Err("short_or_invalid_probe"); }
    Ok(())
}
fn peak_kib() -> Option<u64> {
    fs::read_to_string("/proc/self/status").ok()?.lines().find_map(|line| {
        line.strip_prefix("VmHWM:")?.split_whitespace().next()?.parse().ok()
    })
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RequestProfile { V2, StandardContext, WatchBootstrap, WatchInspect }
impl RequestProfile {
    fn identity(self) -> &'static str {
        match self { Self::V2 => IDENTITY, Self::StandardContext => STANDARD_IDENTITY, Self::WatchBootstrap => BOOTSTRAP_IDENTITY, Self::WatchInspect => INSPECT_IDENTITY }
    }
}
fn parse_args(args: &[String]) -> Result<(&str, RequestProfile), &'static str> {
    let profile = match args {
        [flag, id] if flag == "--video-id" && valid_id(id) => RequestProfile::V2,
        [flag, id, opt] if flag == "--video-id" && valid_id(id) && opt == "--standard-context" => RequestProfile::StandardContext,
        [flag, id, opt] if flag == "--video-id" && valid_id(id) && opt == "--watch-bootstrap" => RequestProfile::WatchBootstrap,
        [flag, id, opt] if flag == "--video-id" && valid_id(id) && opt == "--watch-inspect" => RequestProfile::WatchInspect,
        _ => return Err("invalid_arguments"),
    };
    Ok((&args[1], profile))
}
fn player_body(id: &str, profile: RequestProfile) -> serde_json::Value {
    let mut body = json!({"context":{"client":{
        "clientName":"VISIONOS", "clientVersion":VERSION, "deviceMake":"Apple",
        "deviceModel":"RealityDevice17,1", "userAgent":AGENT,
        "osName":"visionOS", "osVersion":"26.5.23O471", "hl":"en"}}, "videoId":id});
    if profile != RequestProfile::V2 {
        body["context"]["client"]["timeZone"] = json!("UTC");
        body["context"]["client"]["utcOffsetMinutes"] = json!(0);
    }
    body
}
fn player_request(client: &Client, id: &str, profile: RequestProfile, visitor: Option<&bootstrap::Visitor>) -> reqwest::RequestBuilder {
    let request = client.post("https://www.youtube.com/youtubei/v1/player?prettyPrint=false")
        .header("User-Agent", AGENT).header("X-Youtube-Client-Name", "101")
        .header("X-Youtube-Client-Version", VERSION);
    let request = if profile != RequestProfile::V2 {
        request.header("Origin", "https://www.youtube.com")
    } else { request };
    let request = if profile == RequestProfile::WatchBootstrap {
        if let Some(visitor) = visitor { request.header("X-Goog-Visitor-Id", visitor.as_header()) }
        else { request }
    } else { request };
    request.json(&player_body(id, profile))
}
async fn watch_visitor(client: &Client, id: &str) -> Result<bootstrap::Visitor, &'static str> {
    let mut response = client.get(format!("https://www.youtube.com/watch?v={id}&hl=en"))
        .header("User-Agent",AGENT).send().await.map_err(|_| "watch_request_failed")?;
    if !response.status().is_success() { return Err("watch_http_failed"); }
    let html = response.headers().get("content-type").and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next()).is_some_and(|v| v.trim().eq_ignore_ascii_case("text/html"));
    if !html { return Err("watch_content_type_rejected"); }
    if response.content_length().is_some_and(|n| n > bootstrap::PAGE_MAX as u64) {
        return Err("watch_size_limit");
    }
    let mut bytes = Vec::with_capacity(32768);
    while let Some(chunk) = response.chunk().await.map_err(|_| "watch_read_failed")? {
        if chunk.len() > bootstrap::PAGE_MAX.saturating_sub(bytes.len()) { return Err("watch_size_limit"); }
        bytes.extend_from_slice(&chunk);
    }
    bootstrap::parse(&bytes,id)
}
fn http_client() -> Result<Client, &'static str> {
    Client::builder().https_only(true).no_proxy()
        .redirect(reqwest::redirect::Policy::none()).connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(20)).build().map_err(|_| "client_init_failed")
}
fn watch_summary(visitor: bootstrap::Visitor, started: Instant) -> serde_json::Value {
    let metadata_nonfinite_ignored = visitor.metadata_nonfinite_ignored();
    let literal_syntax_normalized = visitor.literal_syntax_normalized();
    drop(visitor);
    json!({"status":"diagnostic_pass", "candidate":INSPECT_IDENTITY, "stage":"watch",
        "watch_parser_only":true, "player_requested":false, "media_requested":false,
        "metadata_nonfinite_ignored":metadata_nonfinite_ignored, "literal_syntax_normalized":literal_syntax_normalized,
        "native_acquisition_demonstrated":false, "production_ready":false, "external_tools":false,
        "total_ms":started.elapsed().as_millis(), "peak_rss_kib":peak_kib()})
}
// Separate watch-only branch: it has no call to player_request/probe/media logic.
async fn inspect_watch(id: &str, stage: &mut &'static str) -> Result<serde_json::Value, &'static str> {
    let started = Instant::now();
    let client = http_client()?;
    *stage = "watch";
    let visitor = watch_visitor(&client,id).await?;
    Ok(watch_summary(visitor,started))
}
async fn probe(id: &str, profile: RequestProfile, stage: &mut &'static str) -> Result<serde_json::Value, &'static str> {
    let started = Instant::now();
    let client = http_client()?;
    // V4 alone reads anonymous context. Never loads or stores account cookies,
    // creates PO tokens, follows redirects or responds to provider restrictions.
    let visitor = if profile == RequestProfile::WatchBootstrap {
        *stage = "watch";
        Some(watch_visitor(&client,id).await?)
    } else { None };
    let metadata_nonfinite_ignored = visitor.as_ref().is_some_and(|v| v.metadata_nonfinite_ignored());
    let literal_syntax_normalized = visitor.as_ref().is_some_and(|v| v.literal_syntax_normalized());
    *stage = "player";
    let mut response = player_request(&client, id, profile, visitor.as_ref()).send().await.map_err(|_| "player_request_failed")?;
    if !response.status().is_success() { return Err("player_http_failed"); }
    if response.content_length().is_some_and(|len| len > METADATA_MAX as u64) { return Err("metadata_size_limit"); }
    let mut bytes = Vec::with_capacity(32768);
    while let Some(chunk) = response.chunk().await.map_err(|_| "player_read_failed")? {
        append_bounded(&mut bytes, &chunk, METADATA_MAX)?;
    }
    drop(visitor);
    let player: Player = serde_json::from_slice(&bytes).map_err(|_| "player_json_invalid")?;
    drop(bytes);
    let format = select(&player, id)?;
    let resolve_ms = started.elapsed().as_millis();
    // No redirects; selected URL is origin-validated and never printed.
    *stage = "media";
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
    validate_sample(count,&prefix)?;
    Ok(json!({"status":"pass", "candidate":profile.identity(), "bytes":count, "itag":format.itag,
        "resolve_ms":resolve_ms, "first_byte_ms":first_byte_ms, "total_ms":started.elapsed().as_millis(),
        "peak_rss_kib":peak_kib(), "external_tools":false, "production_ready":false,
        "literal_syntax_normalized":literal_syntax_normalized,"metadata_nonfinite_ignored":metadata_nonfinite_ignored}))
}
async fn within_deadline<T>(future: impl std::future::Future<Output=Result<T, &'static str>>, limit: Duration) -> Result<T, &'static str> {
    tokio::time::timeout(limit,future).await.map_err(|_| "deadline_exceeded")?
}
#[tokio::main(flavor="current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).take(4).collect();
    if args == ["--help"] {
        println!("aysha-direct-prototype --video-id ID [--standard-context | --watch-bootstrap | --watch-inspect]\n--watch-inspect: v5 data-literal check ONLY; never requests player or media.\n--watch-bootstrap: opt-in v4 anonymous watch context, no cookies/persistence/JS; one attempt.\n--standard-context: isolated v3 request adding only Origin and UTC fields; no fallback.\nExperimental public AAC 64KiB probe only. No JS/signature/PO-token support, credentials, fallback, playback or production integration. Fixed 60s deadline; external timeout recommended.");
        return;
    }
    let (id, profile) = match parse_args(&args) {
        Ok(value) => value,
        Err(code) => {
            println!("{}", json!({"status":"fail", "candidate":"invalid-cli", "code":code}));
            std::process::exit(2);
        }
    };
    let mut stage = "init";
    let result = if profile == RequestProfile::WatchInspect {
        within_deadline(inspect_watch(id,&mut stage),Duration::from_secs(60)).await
    } else {
        within_deadline(probe(id, profile, &mut stage),Duration::from_secs(60)).await
    };
    match result {
        Ok(metrics) => println!("{metrics}"),
        Err(code) => {
            println!("{}",json!({"status":"fail", "candidate":profile.identity(), "code":code, "stage":stage,
                "external_tools":false, "watch_parser_only":profile == RequestProfile::WatchInspect,
                "player_requested":stage == "player" || stage == "media", "media_requested":stage == "media",
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
    #[test] fn standard_context_body_changes_only_two_fields() {
        let old = player_body("Jm7Satp7Zqk", RequestProfile::V2);
        let mut changed = player_body("Jm7Satp7Zqk", RequestProfile::StandardContext);
        assert_eq!(changed["context"]["client"]["timeZone"], "UTC");
        assert_eq!(changed["context"]["client"]["utcOffsetMinutes"], 0);
        let client = changed["context"]["client"].as_object_mut().unwrap();
        client.remove("timeZone"); client.remove("utcOffsetMinutes");
        assert_eq!(old, changed);
        assert_eq!(old.as_object().unwrap().len(), 2); // no auth/integrity/restriction fields
        assert!(old["context"]["client"].get("visitorData").is_none());
    }
    #[tokio::test(flavor="current_thread")] async fn standard_context_headers_change_only_origin() {
        // Build offline: never send or open a connection.
        let client = Client::builder().no_proxy().build().unwrap();
        let old = player_request(&client,"Jm7Satp7Zqk",RequestProfile::V2,None).build().unwrap();
        let mut changed = player_request(&client,"Jm7Satp7Zqk",RequestProfile::StandardContext,None).build().unwrap();
        assert_eq!(changed.headers_mut().remove("origin").unwrap(), "https://www.youtube.com");
        assert_eq!(old.headers(), changed.headers());
        assert_eq!(old.url(), changed.url());
        assert_eq!(old.method(), changed.method());
        assert!(old.headers().get("cookie").is_none());
        assert!(changed.headers().get("authorization").is_none());
    }
    #[test] fn profiles_are_explicit_and_invalid_options_never_trigger_a_request() {
        let args = |items: &[&str]| items.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let legacy = args(&["--video-id", "Jm7Satp7Zqk"]);
        let standard = args(&["--video-id", "Jm7Satp7Zqk", "--standard-context"]);
        assert_eq!(parse_args(&legacy).unwrap().1.identity(), IDENTITY);
        assert_eq!(parse_args(&standard).unwrap().1.identity(), STANDARD_IDENTITY);
        for value in [args(&[]), args(&["--standard-context"]),
            args(&["--video-id", "invalid", "--standard-context"]),
            args(&["--video-id", "Jm7Satp7Zqk", "--other"]),
            args(&["--video-id", "Jm7Satp7Zqk", "--standard-context", "--extra"])] {
            assert_eq!(parse_args(&value).err(), Some("invalid_arguments"));
        }
    }
    #[tokio::test(flavor="current_thread")] async fn bootstrap_only_adds_allowlisted_header_and_never_media_headers() {
        let client = Client::builder().no_proxy().build().unwrap();
        let page = br#"<script>ytcfg.set({"VISITOR_DATA":"fixture_visitor"});</script><script>var ytInitialPlayerResponse = {"playabilityStatus":{"status":"OK"},"videoDetails":{"videoId":"Jm7Satp7Zqk"}};</script>"#;
        let visitor = bootstrap::parse(page,"Jm7Satp7Zqk").unwrap();
        let v3 = player_request(&client,"Jm7Satp7Zqk",RequestProfile::StandardContext,None).build().unwrap();
        let mut v4 = player_request(&client,"Jm7Satp7Zqk",RequestProfile::WatchBootstrap,Some(&visitor)).build().unwrap();
        assert_eq!(v4.headers_mut().remove("X-Goog-Visitor-Id").unwrap(), "fixture_visitor");
        assert_eq!(v3.headers(),v4.headers());
        assert_eq!(v3.url(),v4.url());
        assert_eq!(v3.body().unwrap().as_bytes(),v4.body().unwrap().as_bytes());
        let legacy = player_request(&client,"Jm7Satp7Zqk",RequestProfile::V2,Some(&visitor)).build().unwrap();
        assert!(legacy.headers().get("X-Goog-Visitor-Id").is_none());
        // Media is built independently and must not inherit provider context/auth.
        let media = client.get("https://x.googlevideo.com/a").header("Range","bytes=0-65535").build().unwrap();
        for name in ["cookie","authorization","X-Goog-Visitor-Id","origin"] {
            assert!(media.headers().get(name).is_none());
        }
    }
    #[test] fn bootstrap_mode_cannot_be_combined_with_other_modes_or_values() {
        let args = |items: &[&str]| items.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let valid = args(&["--video-id","Jm7Satp7Zqk","--watch-bootstrap"]);
        assert_eq!(parse_args(&valid).unwrap().1.identity(), BOOTSTRAP_IDENTITY);
        for items in [args(&["--video-id","Jm7Satp7Zqk","--standard-context","--watch-bootstrap"]),
            args(&["--video-id","Jm7Satp7Zqk","--watch-bootstrap=private"]),
            args(&["--video-id","invalid","--watch-bootstrap"])] {
            assert_eq!(parse_args(&items).err(),Some("invalid_arguments"));
        }
    }
    #[tokio::test(flavor="current_thread")] async fn deadline_cancels_and_drops_owned_inflight_state() {
        use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
        struct Owned(Arc<AtomicBool>);
        impl Drop for Owned { fn drop(&mut self) { self.0.store(true,Ordering::SeqCst); } }
        let dropped = Arc::new(AtomicBool::new(false));
        let owned = Owned(dropped.clone());
        let future = async move {
            let _owned = owned;
            std::future::pending::<Result<(), &'static str>>().await
        };
        assert_eq!(within_deadline(future,Duration::from_millis(1)).await, Err("deadline_exceeded"));
        assert!(dropped.load(Ordering::SeqCst));
    }
    #[test] fn watch_inspect_is_separate_strict_and_never_acquisition_success() {
        let args: Vec<String> = ["--video-id","Jm7Satp7Zqk","--watch-inspect"].iter().map(|s| s.to_string()).collect();
        assert_eq!(parse_args(&args).unwrap().1, RequestProfile::WatchInspect);
        let mut combined = args.clone(); combined.push("--watch-bootstrap".into());
        assert_eq!(parse_args(&combined).err(),Some("invalid_arguments"));
        let page = br#"<script>ytcfg.set({"VISITOR_DATA":"PRIVATE_CONTEXT"});</script><script>var ytInitialPlayerResponse = {"playabilityStatus":{"status":"OK"},"videoDetails":{"videoId":"Jm7Satp7Zqk"}};</script>"#;
        let visitor = bootstrap::parse(page,"Jm7Satp7Zqk").unwrap();
        let report = watch_summary(visitor,Instant::now());
        assert_eq!(report["status"],"diagnostic_pass");
        assert_eq!(report["player_requested"],false);
        assert_eq!(report["media_requested"],false);
        assert_eq!(report["native_acquisition_demonstrated"],false);
        assert!(!report.to_string().contains("PRIVATE_CONTEXT"));
    }
    #[tokio::test(flavor="current_thread")] async fn fixture_pipeline_watch_request_selection_and_prefix_validation() {
        // Entire pipeline is synthetic/offline: no send(), audio device or media decode.
        let html=br#"<!doctype html><html><textarea><script>ytcfg.set({VISITOR_DATA:'fake'});</script></textarea><script nonce="x">/* data only */ ytcfg . set ({VISITOR_DATA:'opaque_fixture',LOGGED_IN:false,INNERTUBE_CONTEXT:{client:{visitorData:'opaque_fixture'}},metadata:{quoted:'I\'m } [ not code',values:[NaN,Infinity,-Infinity,],},});</script><script>const ytInitialPlayerResponse={playabilityStatus:{status:'OK'},videoDetails:{videoId:'Jm7Satp7Zqk'},};</script></html>"#;
        let visitor=bootstrap::parse(html,"Jm7Satp7Zqk").unwrap();
        assert!(visitor.literal_syntax_normalized()); assert!(visitor.metadata_nonfinite_ignored());
        let client=http_client().unwrap();
        let request=player_request(&client,"Jm7Satp7Zqk",RequestProfile::WatchBootstrap,Some(&visitor)).build().unwrap();
        assert_eq!(request.url().host_str(),Some("www.youtube.com"));
        assert_eq!(request.headers()["X-Goog-Visitor-Id"],"opaque_fixture");
        assert!(request.headers().get("cookie").is_none());
        assert!(request.headers().get("authorization").is_none());
        let body:serde_json::Value=serde_json::from_slice(request.body().unwrap().as_bytes().unwrap()).unwrap();
        assert!(body["context"]["client"].get("visitorData").is_none());
        assert!(body.get("serviceIntegrityDimensions").is_none());
        let payload=fixture(json!([format("",256000),format("https://x.googlevideo.com/audio",128000)]));
        let selected=select(&payload,"Jm7Satp7Zqk").unwrap(); assert_eq!(selected.itag,140);
        assert!(validate_sample(65536,b"\0\0\0\x20ftyp").is_ok());
        assert!(validate_sample(65535,b"\0\0\0\x20ftyp").is_err());
        assert!(validate_sample(65536,b"<html>bad").is_err());
        let report=watch_summary(visitor,Instant::now());
        assert_eq!(report["player_requested"],false);
        assert!(!report.to_string().contains("opaque_fixture"));
    }
    #[test] fn single_quoted_initial_player_restrictions_stop_the_pipeline() {
        for (status,expected) in [("LOGIN_REQUIRED","player_login_required"),
            ("AGE_CHECK_REQUIRED","player_age_check_required"),("UNPLAYABLE","player_unplayable")] {
            let html=format!("<script>ytcfg.set({{VISITOR_DATA:'opaque',LOGGED_IN:false}});</script><script>let ytInitialPlayerResponse={{playabilityStatus:{{status:'{status}',reason:'PRIVATE'}},videoDetails:{{videoId:'Jm7Satp7Zqk'}}}};</script>");
            let error=bootstrap::parse(html.as_bytes(),"Jm7Satp7Zqk").err().unwrap();
            assert_eq!(error,expected); assert!(!error.contains("PRIVATE"));
        }
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
        assert_eq!(select(&player,"Jm7Satp7Zqk").err(),Some("player_login_required"));
    }
    #[test] fn metadata_limits_and_container() {
        let mut bytes = Vec::new();
        assert!(append_bounded(&mut bytes,b"1234",4).is_ok());
        assert_eq!(append_bounded(&mut bytes,b"5",4),Err("metadata_size_limit"));
        assert_eq!(bytes,b"1234");
        assert!(mp4(b"\0\0\0\x20ftyp")); assert!(!mp4(b"<html>bad"));
    }
    #[test] fn rejection_categories_are_fixed_and_never_echo_upstream_text() {
        for (status, code) in [
            ("LOGIN_REQUIRED", "player_login_required"),
            ("UNPLAYABLE", "player_unplayable"), ("ERROR", "player_error"),
            ("LIVE_STREAM_OFFLINE", "player_live_offline"),
            ("AGE_CHECK_REQUIRED", "player_age_check_required"),
            ("AGE_VERIFICATION_REQUIRED", "player_age_check_required"),
            ("CONTENT_CHECK_REQUIRED", "player_content_check_required"),
            ("private-arbitrary-status", "player_status_unknown"),
        ] {
            let player: Player = serde_json::from_value(json!({"playabilityStatus":{
                "status":status, "reason":"https://private.example/?signed=secret"}})).unwrap();
            assert_eq!(playability(&player), Err(code));
        }
        assert_eq!(playability(&Player::default()), Err("player_status_missing"));
    }
    #[test] fn bot_signal_is_distinguished_without_bypassing_login() {
        let player: Player = serde_json::from_value(json!({"playabilityStatus":{
            "status":"LOGIN_REQUIRED", "reason":"Sign in to confirm you’re not a bot"}})).unwrap();
        assert_eq!(playability(&player), Err("player_bot_check"));
        let player: Player = serde_json::from_value(json!({"playabilityStatus":{
            "status":"ERROR", "messages":["Confirm you're not a bot", "private-content"]}})).unwrap();
        assert_eq!(playability(&player), Err("player_bot_check"));
    }
    #[test] fn unrelated_or_too_many_formats_fail() {
        let mut f = format("https://x.googlevideo.com/a",128000); f["width"]=json!(1920);
        assert_eq!(select(&fixture(json!([f])),"Jm7Satp7Zqk").err(),Some("aac_unavailable"));
        let f = format("https://x.googlevideo.com/a",128000);
        assert_eq!(select(&fixture(json!(vec![f;257])),"Jm7Satp7Zqk").err(),Some("format_limit"));
    }
}
