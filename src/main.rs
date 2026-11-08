//! Aysha's Music Player — Pure Rust standard library server.
//! No external crates. Designed to compile with `rustc`/`cargo` and run
//! under ~20MB RAM.
//!
//! Endpoints:
//!   GET  /                 — SPA UI (PWA, MediaSession, liked songs)
//!   GET  /manifest.json    — PWA manifest
//!   GET  /sw.js            — Service worker
//!   GET  /api/status       — Player state
//!   GET  /api/search?q=    — YouTube search via yt-dlp subprocess
//!   POST /api/play         — Play track (JSON body)
//!   POST /api/pause        — Pause
//!   POST /api/stop         — Stop
//!   POST /api/volume/up    — Volume up
//!   POST /api/volume/down  — Volume down
//!   POST /api/next         — Next track
//!   POST /api/prev         — Previous track
//!   GET  /api/bt/status    — Bluetooth status
//!   POST /api/bt/connect   — Bluetooth connect
//!   GET  /api/liked        — Liked songs
//!   POST /api/like         — Like a track
//!   POST /api/unlike       — Unlike a track

use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write, BufReader};
use std::thread;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::process::Command;
use std::os::unix::net::UnixStream;
use std::fs;
use std::time::Duration;

const PORT: u16 = 5050;
const SOCK_PATH: &str = "/tmp/aysha-mpv.sock";
const HISTORY_FILE: &str = "/tmp/aysha-history.json";
const LIKED_FILE: &str = "/tmp/aysha-liked.json";

const INDEX_HTML: &str = include_str!("www/index.html");
const MANIFEST_JSON: &str = include_str!("www/manifest.json");
const SW_JS: &str = include_str!("www/sw.js");

// ============================================================================
// JSON helpers (no serde)
// ============================================================================

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn json_str(s: &str) -> String {
    format!("\"{}\"", json_escape(s))
}

fn json_obj(pairs: &[(&str, String)]) -> String {
    let mut out = String::from("{");
    for (i, (k, v)) in pairs.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&json_str(k));
        out.push(':');
        out.push_str(v);
    }
    out.push('}');
    out
}

fn json_arr(items: &[String]) -> String {
    let mut out = String::from("[");
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(item);
    }
    out.push(']');
    out
}

/// Extract a top-level string field from a JSON line.
/// Handles `"field": "value"` and `"field": value`.
fn extract_json_field(s: &str, field: &str) -> String {
    let pattern = format!("\"{}\"", field);
    match s.find(&pattern) {
        Some(pos) => {
            let after = &s[pos + pattern.len()..];
            let after = after.trim_start();
            if after.starts_with(':') {
                let val = &after[1..].trim_start();
                if val.starts_with('"') {
                    let rest = &val[1..];
                    let mut end = 0;
                    let mut escaped = false;
                    for (i, ch) in rest.char_indices() {
                        if escaped {
                            escaped = false;
                        } else if ch == '\\' {
                            escaped = true;
                        } else if ch == '"' {
                            end = i;
                            break;
                        }
                    }
                    rest[..end].to_string()
                } else {
                    let end = val
                        .find(|c: char| c == ',' || c == '}' || c.is_whitespace())
                        .unwrap_or(val.len());
                    val[..end].to_string()
                }
            } else {
                String::new()
            }
        }
        None => String::new(),
    }
}

fn parse_query(qs: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for pair in qs.split('&') {
        if let Some(eq) = pair.find('=') {
            let k = url_decode(&pair[..eq]);
            let v = url_decode(&pair[eq + 1..]);
            map.insert(k, v);
        }
    }
    map
}

fn url_decode(s: &str) -> String {
    let mut out = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = &bytes[i + 1..i + 3];
            if let Ok(val) = u8::from_str_radix(
                std::str::from_utf8(hex).unwrap_or("00"),
                16,
            ) {
                out.push(val as char);
                i += 3;
            } else {
                out.push(bytes[i] as char);
                i += 1;
            }
        } else if bytes[i] == b'+' {
            out.push(' ');
            i += 1;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

fn query_param(qs: Option<&str>, key: &str) -> Option<String> {
    qs.and_then(|q| parse_query(q).get(key).cloned())
}

/// Parse a flat JSON object like {"key1":"value1","key2":"value2"} into a HashMap.
/// Handles escaped quotes inside string values.
fn parse_flat_json(s: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let s = s.trim();
    if !(s.starts_with('{') && s.ends_with('}')) {
        return map;
    }
    let inner = &s[1..s.len() - 1];
    let mut key = String::new();
    let mut val = String::new();
    let mut state = ParseState::ExpectKey;
    let mut in_string = false;
    let mut escaped = false;
    let mut collecting_key = false;
    let mut collecting_val = false;

    for ch in inner.chars() {
        if escaped {
            if collecting_key {
                key.push(ch);
            } else if collecting_val {
                val.push(ch);
            }
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            if collecting_key {
                key.push(ch);
            } else if collecting_val {
                val.push(ch);
            }
            continue;
        }
        match state {
            ParseState::ExpectKey => {
                if ch == '"' {
                    in_string = true;
                    collecting_key = true;
                    state = ParseState::InKey;
                }
            }
            ParseState::InKey => {
                if ch == '"' {
                    in_string = false;
                    collecting_key = false;
                    state = ParseState::ExpectColon;
                } else {
                    key.push(ch);
                }
            }
            ParseState::ExpectColon => {
                if ch == ':' {
                    state = ParseState::ExpectValue;
                }
            }
            ParseState::ExpectValue => {
                if ch == '"' {
                    in_string = true;
                    collecting_val = true;
                    state = ParseState::InValue;
                } else if ch.is_whitespace() {
                    // skip
                } else {
                    // non-string value (number, true, false, null)
                    collecting_val = true;
                    val.push(ch);
                    state = ParseState::InValueNonString;
                }
            }
            ParseState::InValue => {
                if ch == '"' {
                    in_string = false;
                    collecting_val = false;
                    map.insert(key.clone(), val.clone());
                    key.clear();
                    val.clear();
                    state = ParseState::ExpectComma;
                } else {
                    val.push(ch);
                }
            }
            ParseState::InValueNonString => {
                if ch == ',' {
                    map.insert(key.clone(), val.clone());
                    key.clear();
                    val.clear();
                    state = ParseState::ExpectKey;
                } else if !ch.is_whitespace() {
                    val.push(ch);
                }
            }
            ParseState::ExpectComma => {
                if ch == ',' {
                    state = ParseState::ExpectKey;
                }
                // skip whitespace
            }
        }
    }
    // Handle last key-value pair (no trailing comma)
    if !key.is_empty() && !val.is_empty() {
        map.insert(key, val);
    }
    map
}

#[derive(PartialEq)]
enum ParseState {
    ExpectKey,
    InKey,
    ExpectColon,
    ExpectValue,
    InValue,
    InValueNonString,
    ExpectComma,
}

// ============================================================================
// State
// ============================================================================

#[derive(Clone, Debug)]
struct Track {
    id: String,
    title: String,
    artist: String,
    thumbnail: String,
    url: String,
}

impl Track {
    fn to_json(&self) -> String {
        json_obj(&[
            ("id", json_str(&self.id)),
            ("title", json_str(&self.title)),
            ("artist", json_str(&self.artist)),
            ("thumbnail", json_str(&self.thumbnail)),
            ("url", json_str(&self.url)),
        ])
    }
}

struct PlayerState {
    current_track: Option<Track>,
    is_playing: bool,
    volume: u32,
    history: Vec<Track>,
    queue: Vec<Track>,
    liked: Vec<Track>,
}

struct AppState {
    player: Mutex<PlayerState>,
}

impl AppState {
    fn new() -> Self {
        Self {
            player: Mutex::new(PlayerState {
                current_track: None,
                is_playing: false,
                volume: 80,
                history: Vec::new(),
                queue: Vec::new(),
                liked: Vec::new(),
            }),
        }
    }
}

// ============================================================================
// File persistence (tiny JSON)
// ============================================================================

fn load_tracks(path: &str) -> Vec<Track> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut tracks = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let id = extract_json_field(line, "id");
        if id.is_empty() {
            continue;
        }
        tracks.push(Track {
            id,
            title: extract_json_field(line, "title"),
            artist: extract_json_field(line, "artist"),
            thumbnail: extract_json_field(line, "thumbnail"),
            url: extract_json_field(line, "url"),
        });
    }
    tracks
}

fn save_tracks(path: &str, tracks: &[Track]) {
    let mut json = String::from("[");
    for (i, t) in tracks.iter().enumerate() {
        if i > 0 {
            json.push(',');
        }
        json.push_str(&t.to_json());
    }
    json.push(']');
    let _ = fs::write(path, json);
}

// ============================================================================
// MPV IPC helpers
// ============================================================================

fn mpv_ipc(cmd: &str) {
    if let Ok(mut stream) = UnixStream::connect(SOCK_PATH) {
        let _ = stream.write_all(format!("{}\n", cmd).as_bytes());
    }
}

fn mpv_get_prop(prop: &str) -> Option<String> {
    if let Ok(mut stream) = UnixStream::connect(SOCK_PATH) {
        let cmd = format!(
            "{{\"command\":[\"get_property\",\"{}\"]}}\n",
            prop
        );
        let _ = stream.write_all(cmd.as_bytes());
        let mut reader = BufReader::new(stream.try_clone().unwrap_or(stream));
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? > 0 {
            let data_field = "\"data\":";
            if let Some(pos) = line.find(data_field) {
                let after = &line[pos + data_field.len()..];
                let trimmed = after.trim_start();
                if trimmed.starts_with('"') {
                    let rest = &trimmed[1..];
                    let mut end = 0;
                    let mut escaped = false;
                    for (i, ch) in rest.char_indices() {
                        if escaped {
                            escaped = false;
                        } else if ch == '\\' {
                            escaped = true;
                        } else if ch == '"' {
                            end = i;
                            break;
                        }
                    }
                    return Some(rest[..end].to_string());
                } else {
                    let end = trimmed
                        .find(|c: char| c == ',' || c == '}' || c.is_whitespace())
                        .unwrap_or(trimmed.len());
                    return Some(trimmed[..end].to_string());
                }
            }
        }
    }
    None
}

fn get_mpv_pos_duration() -> (f64, f64) {
    let pos = mpv_get_prop("time-pos")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    let dur = mpv_get_prop("duration")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    (pos, dur)
}

// ============================================================================
// Bluetooth helpers
// ============================================================================

fn is_bt_connected(mac: &str) -> bool {
    if let Ok(out) = Command::new("bluetoothctl").args(&["info", mac]).output() {
        let text = String::from_utf8_lossy(&out.stdout);
        text.contains("Connected: yes")
    } else {
        false
    }
}

fn run_bt_connect(mac: &str) -> bool {
    let _ = Command::new("bluetoothctl").args(&["power", "on"]).output();
    let _ = Command::new("bluetoothctl").args(&["connect", mac]).output();
    thread::sleep(Duration::from_secs(2));
    is_bt_connected(mac)
}

// ============================================================================
// API route handlers
// ============================================================================

fn api_status(state: &Arc<AppState>) -> (String, String) {
    let p = state.player.lock().unwrap();
    let (pos, duration) = get_mpv_pos_duration();
    let track_json = match &p.current_track {
        Some(t) => t.to_json(),
        None => String::from("null"),
    };
    let body = json_obj(&[
        ("current_track", track_json),
        ("is_playing", if p.is_playing { "true" } else { "false" }.to_string()),
        ("volume", p.volume.to_string()),
        ("pos", pos.to_string()),
        ("duration", duration.to_string()),
        ("liked_count", p.liked.len().to_string()),
    ]);
    (String::from("200 OK"), body)
}

fn api_search(q: &str) -> (String, String) {
    let output = Command::new("yt-dlp")
        .args(&[
            "--flat-playlist",
            "--dump-json",
            &format!("ytsearch16:{}", q),
        ])
        .output();

    let mut tracks: Vec<String> = Vec::new();
    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let id = extract_json_field(line, "id");
            if id.is_empty() {
                continue;
            }
            let title = extract_json_field(line, "title");
            let artist = extract_json_field(line, "uploader");
            let artist = if artist.is_empty() {
                "YouTube".to_string()
            } else {
                artist
            };
            let thumbnail = format!("https://img.youtube.com/vi/{}/hqdefault.jpg", id);
            let url = format!("https://www.youtube.com/watch?v={}", id);
            tracks.push(Track {
                id,
                title,
                artist,
                thumbnail,
                url,
            }.to_json());
        }
    }
    (String::from("200 OK"), json_arr(&tracks))
}

fn api_play(body: &str, state: &Arc<AppState>) -> (String, String) {
    let params = parse_flat_json(body);
    let url = params.get("url").cloned().unwrap_or_default();
    let title = params
        .get("title")
        .cloned()
        .unwrap_or_else(|| "Unknown Track".to_string());
    let artist = params
        .get("artist")
        .cloned()
        .unwrap_or_else(|| "YouTube".to_string());
    let thumbnail = params.get("thumbnail").cloned().unwrap_or_default();
    let id = params.get("id").cloned().unwrap_or_default();

    let track = Track {
        id: id.clone(),
        title: title.clone(),
        artist: artist.clone(),
        thumbnail: thumbnail.clone(),
        url: url.clone(),
    };

    {
        let mut p = state.player.lock().unwrap();
        if let Some(ref current) = p.current_track {
            p.history.push(current.clone());
        }
        p.current_track = Some(track.clone());
        p.is_playing = true;
    }

    // Kill existing mpv
    let _ = Command::new("pkill")
        .args(&["-f", "mpv.*aysha-mpv.sock"])
        .output();

    // Spawn new mpv in a thread
    {
        let vol = { let p = state.player.lock().unwrap(); p.volume };
        let url_clone = url.clone();
        thread::spawn(move || {
            let _ = Command::new("mpv")
                .args(&[
                    "--no-video",
                    "--no-terminal",
                    "--idle=yes",
                    "--keep-open=no",
                    &format!("--volume={}", vol),
                    "--ao=pulse",
                    &format!("--input-ipc-server={}", SOCK_PATH),
                    &url_clone,
                ])
                .env("XDG_RUNTIME_DIR", "/run/user/1000")
                .env("PULSE_SERVER", "unix:/run/user/1000/pulse/native")
                .spawn();
        });
    }

    (String::from("200 OK"), json_obj(&[("status", json_str("playing"))]))
}

fn api_pause(state: &Arc<AppState>) -> (String, String) {
    mpv_ipc(r#"{"command":["set_property","pause",true]}"#);
    {
        let mut p = state.player.lock().unwrap();
        p.is_playing = false;
    }
    (String::from("200 OK"), json_obj(&[("status", json_str("paused"))]))
}

fn api_stop(state: &Arc<AppState>) -> (String, String) {
    let _ = Command::new("pkill")
        .args(&["-f", "mpv.*aysha-mpv.sock"])
        .output();
    {
        let mut p = state.player.lock().unwrap();
        p.is_playing = false;
        p.current_track = None;
    }
    (String::from("200 OK"), json_obj(&[("status", json_str("stopped"))]))
}

fn api_volume(state: &Arc<AppState>, up: bool, query: Option<&str>) -> (String, String) {
    let step: u32 = query
        .and_then(|q| parse_query(q).get("step").cloned())
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);
    {
        let mut p = state.player.lock().unwrap();
        if up {
            p.volume = (p.volume + step).min(100);
        } else {
            p.volume = p.volume.saturating_sub(step);
        }
        let vol = p.volume;
        mpv_ipc(&format!("{{\"command\":[\"set_property\",\"volume\",{}]}}", vol));
    }
    let vol = { let p = state.player.lock().unwrap(); p.volume };
    (String::from("200 OK"), json_obj(&[("volume", vol.to_string())]))
}

fn api_next(state: &Arc<AppState>) -> (String, String) {
    let next_track = {
        let mut p = state.player.lock().unwrap();
        p.queue.remove(0)
    };
    match next_track {
        Some(t) => {
            let body = t.to_json();
            let _ = api_play(&body, state);
            (String::from("200 OK"), json_obj(&[("status", json_str("next"))]))
        }
        None => {
            let _ = api_stop(state);
            (String::from("200 OK"), json_obj(&[("status", json_str("stopped"))]))
        }
    }
}

fn api_prev(state: &Arc<AppState>) -> (String, String) {
    let prev_track = {
        let mut p = state.player.lock().unwrap();
        p.history.pop()
    };
    match prev_track {
        Some(t) => {
            let body = t.to_json();
            let _ = api_play(&body, state);
            (String::from("200 OK"), json_obj(&[("status", json_str("prev"))]))
        }
        None => (String::from("200 OK"), json_obj(&[("status", json_str("no_prev"))])),
    }
}

fn api_bt_status() -> (String, String) {
    let connected = is_bt_connected("04:CB:88:92:43:60");
    (
        String::from("200 OK"),
        json_obj(&[
            ("device", json_str("JBL Flip Essential")),
            ("mac", json_str("04:CB:88:92:43:60")),
            ("connected", if connected { "true" } else { "false" }.to_string()),
        ]),
    )
}

fn api_bt_connect(query: Option<&str>) -> (String, String) {
    let mac = query
        .and_then(|q| parse_query(q).get("mac").cloned())
        .unwrap_or_else(|| "04:CB:88:92:43:60".to_string());
    let success = run_bt_connect(&mac);
    (
        String::from("200 OK"),
        json_obj(&[("status", if success { "connected" } else { "failed" }.to_string())]),
    )
}

fn api_liked(state: &Arc<AppState>) -> (String, String) {
    let p = state.player.lock().unwrap();
    let mut json_tracks: Vec<String> = Vec::new();
    for t in &p.liked {
        json_tracks.push(t.to_json());
    }
    (String::from("200 OK"), json_arr(&json_tracks))
}

fn api_like(body: &str, state: &Arc<AppState>) -> (String, String) {
    let params = parse_flat_json(body);
    let id = params.get("id").cloned().unwrap_or_default();
    if id.is_empty() {
        return (
            String::from("400 Bad Request"),
            json_obj(&[("error", json_str("missing id"))]),
        );
    }
    let track = Track {
        id: id.clone(),
        title: params.get("title").cloned().unwrap_or_default(),
        artist: params.get("artist").cloned().unwrap_or_default(),
        thumbnail: params.get("thumbnail").cloned().unwrap_or_default(),
        url: params.get("url").cloned().unwrap_or_default(),
    };

    {
        let mut p = state.player.lock().unwrap();
        if !p.liked.iter().any(|t| t.id == id) {
            p.liked.push(track.clone());
            save_tracks(LIKED_FILE, &p.liked);
        }
    }
    (String::from("200 OK"), json_obj(&[("status", json_str("liked"))]))
}

fn api_unlike(body: &str, state: &Arc<AppState>) -> (String, String) {
    let params = parse_flat_json(body);
    let id = params.get("id").cloned().unwrap_or_default();
    {
        let mut p = state.player.lock().unwrap();
        p.liked.retain(|t| t.id != id);
        save_tracks(LIKED_FILE, &p.liked);
    }
    (String::from("200 OK"), json_obj(&[("status", json_str("unliked"))]))
}

// ============================================================================
// HTTP server
// ============================================================================

fn route(method: &str, path: &str, query: Option<&str>, body: &str, state: &Arc<AppState>) -> (String, String, String) {
    match (method, path) {
        ("GET", "/") => (String::from("200 OK"), String::from("text/html; charset=utf-8"), INDEX_HTML.to_string()),
        ("GET", "/manifest.json") => (String::from("200 OK"), String::from("application/manifest+json"), MANIFEST_JSON.to_string()),
        ("GET", "/sw.js") => (String::from("200 OK"), String::from("application/javascript"), SW_JS.to_string()),
        ("GET", "/api/status") => {
            let (s, b) = api_status(state);
            (s, String::from("application/json"), b)
        }
        ("GET", "/api/search") => {
            let (s, b) = api_search(&query_param(query, "q").unwrap_or_default());
            (s, String::from("application/json"), b)
        }
        ("POST", "/api/play") => {
            let (s, b) = api_play(body, state);
            (s, String::from("application/json"), b)
        }
        ("POST", "/api/pause") => {
            let (s, b) = api_pause(state);
            (s, String::from("application/json"), b)
        }
        ("POST", "/api/stop") => {
            let (s, b) = api_stop(state);
            (s, String::from("application/json"), b)
        }
        ("POST", "/api/volume/up") => {
            let (s, b) = api_volume(state, true, query);
            (s, String::from("application/json"), b)
        }
        ("POST", "/api/volume/down") => {
            let (s, b) = api_volume(state, false, query);
            (s, String::from("application/json"), b)
        }
        ("POST", "/api/next") => {
            let (s, b) = api_next(state);
            (s, String::from("application/json"), b)
        }
        ("POST", "/api/prev") => {
            let (s, b) = api_prev(state);
            (s, String::from("application/json"), b)
        }
        ("GET", "/api/bt/status") => {
            let (s, b) = api_bt_status();
            (s, String::from("application/json"), b)
        }
        ("POST", "/api/bt/connect") => {
            let (s, b) = api_bt_connect(query);
            (s, String::from("application/json"), b)
        }
        ("GET", "/api/liked") => {
            let (s, b) = api_liked(state);
            (s, String::from("application/json"), b)
        }
        ("POST", "/api/like") => {
            let (s, b) = api_like(body, state);
            (s, String::from("application/json"), b)
        }
        ("POST", "/api/unlike") => {
            let (s, b) = api_unlike(body, state);
            (s, String::from("application/json"), b)
        }
        _ => (
            String::from("404 Not Found"),
            String::from("application/json"),
            json_obj(&[("error", json_str("not found"))]),
        ),
    }
}

fn handle_client(mut stream: TcpStream, state: Arc<AppState>) {
    // Clone stream for reading so we can write to the original after
    let reader_stream = match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    };
    let mut reader = BufReader::new(reader_stream);

    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];

    // Read headers until \r\n\r\n
    loop {
        match reader.read(&mut tmp) {
            Ok(0) => return,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if buf.len() >= 4 && &buf[buf.len() - 4..] == b"\r\n\r\n" {
                    break;
                }
                if buf.len() > 8192 {
                    return;
                }
            }
            Err(_) => return,
        }
    }

    let request_str = String::from_utf8_lossy(&buf);
    let header_end = match request_str.find("\r\n\r\n") {
        Some(i) => i,
        None => return,
    };
    let headers_part = &request_str[..header_end];
    let body_start = header_end + 4;

    let lines: Vec<&str> = headers_part.lines().collect();
    if lines.is_empty() {
        return;
    }

    let request_line = lines[0];
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }

    let method = parts[0];
    let raw_path = parts[1];

    let (path, query) = if let Some(qpos) = raw_path.find('?') {
        (&raw_path[..qpos], Some(&raw_path[qpos + 1..]))
    } else {
        (raw_path, None)
    };

    let mut content_length: usize = 0;
    for line in &lines[1..] {
        if let Some(rest) = line.to_lowercase().strip_prefix("content-length:") {
            content_length = rest.trim().parse().unwrap_or(0);
        }
    }

    // Body bytes already read into buf
    let mut body = if body_start < buf.len() {
        String::from_utf8_lossy(&buf[body_start..]).to_string()
    } else {
        String::new()
    };

    while body.len() < content_length {
        let remaining = content_length - body.len();
        let mut tmp2 = [0u8; 1024];
        match reader.read(&mut tmp2[..remaining.min(1024)]) {
            Ok(0) => break,
            Ok(n) => {
                body.push_str(&String::from_utf8_lossy(&tmp2[..n]));
            }
            Err(_) => break,
        }
    }

    let (status, content_type, body_str) = route(method, path, query, &body, &state);
    let body_bytes = body_str.as_bytes();
    let _ = stream.write_all(
        format!(
            "HTTP/1.1 {}\r\nContent-Type: {}\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            status,
            content_type,
            body_bytes.len()
        )
        .as_bytes(),
    );
    let _ = stream.write_all(body_bytes);
}

// ============================================================================
// Main
// ============================================================================

fn main() {
    let listener = match TcpListener::bind(format!("0.0.0.0:{}", PORT)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind to port {}: {}", PORT, e);
            return;
        }
    };
    println!("Aysha's Music Player running on http://0.0.0.0:{}", PORT);

    let state = Arc::new(AppState::new());

    // Load liked songs from disk
    {
        let mut p = state.player.lock().unwrap();
        p.liked = load_tracks(LIKED_FILE);
    }

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state = state.clone();
                thread::spawn(move || {
                    handle_client(stream, state);
                });
            }
            Err(e) => {
                eprintln!("connection error: {}", e);
            }
        }
    }
}
