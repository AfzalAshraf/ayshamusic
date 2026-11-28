//! Shareable, allowlisted diagnostics. Never copy raw provider messages, paths,
//! track information, URLs, search text, or account data into the report.
use serde_json::Value;

#[derive(Clone)]
pub struct Event { pub operation: &'static str, pub detail: String }

fn codes(text: &str) -> Vec<&'static str> {
    let known = [
        ("ytdlp_sign_in_or_bot_challenge", "ytdlp_sign_in_or_bot_challenge"),
        ("ytdlp_http_403", "ytdlp_http_403"),
        ("ytdlp_format_unavailable", "ytdlp_format_unavailable"),
        ("ytdlp_js_runtime_unavailable", "ytdlp_js_runtime_unavailable"),
        ("ytdlp_fetch_failed", "ytdlp_fetch_failed"),
        ("ytdlp_missing", "ytdlp_missing"),
        ("ytdlp_start_failed", "ytdlp_start_failed"),
        ("ytdlp_read_failed", "ytdlp_read_failed"),
        ("ytdlp_size_limit", "ytdlp_size_limit"),
        ("ytdlp_timeout", "ytdlp_timeout"),
        ("downloaded_audio_invalid", "downloaded_audio_invalid"),
        ("media_range_http_403", "media_range_http_403"),
        ("media_range_http_404", "media_range_http_404"),
        ("media_range_http_429", "media_range_http_429"),
        ("media_range_http_500", "media_range_http_500"),
        ("media_range_connection_or_timeout", "media_range_connection_or_timeout"),
        ("media_prepare_head_failed", "media_prepare_head_failed"),
        ("media_prepare_tail_failed", "media_prepare_tail_failed"),
        ("media_range_header_missing", "media_range_header_missing"),
        ("media_range_bounds_rejected", "media_range_bounds_rejected"),
        ("media_range_length_mismatch", "media_range_length_mismatch"),
        ("media_range_content_type_rejected", "media_range_content_type_rejected"),
        ("media_url_rejected", "media_url_rejected"),
        ("audio_decode_failed", "audio_decode_failed"),
        ("audio_output_unavailable", "audio_output_unavailable"),
        ("audio_prepare_failed", "audio_prepare_failed"),
        ("audio_prepare_timeout", "audio_prepare_timeout"),
        ("play_cancelled", "play_cancelled"),
        ("source_prepare_deadline", "source_prepare_deadline"),
        ("provider_sign_in_or_bot_challenge", "provider_sign_in_or_bot_challenge"),
        ("LOGIN_REQUIRED", "LOGIN_REQUIRED"),
        ("Sign in to confirm you're not a bot", "provider_sign_in_or_bot_challenge"),
        ("Could not open the selected file", "local_file_open_failed"),
        ("Not a supported M4A file, or file is larger than 64 MB", "local_file_invalid_or_too_large"),
    ];
    let mut found=Vec::new();
    for (needle,code) in known {if text.contains(needle) && !found.contains(&code){found.push(code);}}
    found
}

pub fn event(operation: &'static str, raw: &str, status: Option<u16>) -> Event {
    let operation=match operation {"play"=>"online playback", "local-play"=>"local file playback", "search"=>"search", "control"=>"player control", "player"=>"player status", _=>"application"};
    let mut lines=Vec::new();
    if let Some(code)=status {lines.push(format!("Backend HTTP status: {code}"));}
    // Only known names and codes are retained, never provider-controlled text.
    let profiles=["ANDROID", "IOS", "TVHTML5_SIMPLY_EMBEDDED_PLAYER", "VISIONOS"];
    let positions: Vec<_>=profiles.iter().filter_map(|name|raw.find(&format!("{name}: ")).map(|pos|(pos,*name))).collect();
    let mut positions=positions;positions.sort_by_key(|v|v.0);
    for (i,(pos,name)) in positions.iter().enumerate() {
        let end=positions.get(i+1).map(|v|v.0).unwrap_or(raw.len());
        let found=codes(&raw[*pos..end]);
        lines.push(format!("Source {name}: {}",if found.is_empty(){"unclassified error".into()}else{found.join(", ")}));
    }
    if positions.is_empty() {let found=codes(raw);lines.push(format!("Error codes: {}",if found.is_empty(){"unclassified error (raw message omitted for privacy)".into()}else{found.join(", ")}));}
    Event{operation,detail:lines.join("\n")}
}

pub fn report(events:&[Event], player:&Value) -> String {
    let state=match player["state"].as_str() {Some("idle")=>"idle",Some("playing")=>"playing",Some("paused")=>"paused",Some("buffering")=>"buffering",Some("error")=>"error",Some("stopped")=>"stopped",_=>"unknown"};
    let mut out=format!("Aysha Music Windows diagnostic report\nApp version: {}\nPlatform: Windows ({})\nAudio engine: Rust / system audio\nOnline downloader: yt-dlp {}\nPlayer state: {}\n",env!("CARGO_PKG_VERSION"),std::env::consts::ARCH,crate::downloader::VERSION,state);
    if events.is_empty(){out.push_str("\nNo errors recorded in this session.\n");}
    else {out.push_str("\nRecent errors (oldest first):\n");for (i,event) in events.iter().enumerate(){out.push_str(&format!("\n{}. Operation: {}\n{}\n",i+1,event.operation,event.detail));}}
    out.push_str("\nPrivacy: no song names, search text, local paths, URLs, credentials or raw provider messages are intentionally included. Review before sharing.\n");
    out
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn reports_multiple_clients_without_leaking_untrusted_data() {
        let raw="all clients failed: ANDROID: audio_decode_failed; input: media_range_http_403 https://site/secret?token=private; IOS: audio_decode_failed; input: media_range_http_403 C:\\Users\\Secret\\song.m4a; TVHTML5_SIMPLY_EMBEDDED_PLAYER: LOGIN_REQUIRED password=abc";
        let e=event("play",raw,Some(502));let report=report(&[e],&serde_json::json!({"state":"error","title":"private song"}));
        assert!(report.contains("Source ANDROID: audio_decode_failed, media_range_http_403") || report.contains("Source ANDROID: media_range_http_403, audio_decode_failed"));
        assert!(report.contains("Source IOS:"));assert!(report.contains("TVHTML5_SIMPLY_EMBEDDED_PLAYER: LOGIN_REQUIRED"));assert!(report.contains("HTTP status: 502"));
        for secret in ["https://site", "token=private", "Users", "password=abc", "private song"] {assert!(!report.contains(secret));}
    }
    #[test] fn unknown_errors_and_player_values_are_never_copied() {
        let e=event("search","C:\\private https://example.com/secret",None);
        let report=report(&[e],&serde_json::json!({"state":"secret username"}));
        assert!(report.contains("unclassified"));assert!(report.contains("Player state: unknown"));assert!(!report.contains("private"));assert!(!report.contains("secret username"));
    }
}
