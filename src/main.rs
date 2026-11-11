use axum::{
    extract::{Query, State},
    response::{Html, Json, Response},
    body::{Body, Bytes},
    routing::{get, post},
    Router,
};
use axum::http::header;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, ReadBuf};
use tower_http::cors::CorsLayer;
use futures_util::stream::Stream;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Track {
    id: String,
    title: String,
    artist: String,
    thumbnail: String,
    url: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct PlayerState {
    current_track: Option<Track>,
    is_playing: bool,
    volume: f32, // 0.0 to 1.0
    status_msg: String,
    history: Vec<Track>,
    queue: Vec<Track>,
    liked: Vec<Track>,
}

struct AppStateInner {
    player: PlayerState,
    http_client: reqwest::Client,
}

type AppState = Arc<Mutex<AppStateInner>>;

#[derive(Deserialize)]
struct PlayQuery {
    url: String,
    title: Option<String>,
    artist: Option<String>,
    thumbnail: Option<String>,
    id: Option<String>,
}

#[derive(Deserialize)]
struct VolumeQuery {
    step: Option<f32>,
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

#[derive(Deserialize)]
struct StreamQuery {
    url: String,
}

#[tokio::main]
async fn main() {
    println!("Starting Aysha's Music Player Server...");

    // Spawn non-blocking background Bluetooth auto-reconnect loop
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
            tokio::task::spawn_blocking(|| {
                let _ = run_bt_connect_script();
            }).await.ok();
        }
    });

    let http_client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .build()
        .unwrap_or_default();

    let state = Arc::new(Mutex::new(AppStateInner {
        player: PlayerState {
            current_track: None,
            is_playing: false,
            volume: 0.8,
            status_msg: "Ready".to_string(),
            history: Vec::new(),
            queue: Vec::new(),
            liked: Vec::new(),
        },
        http_client,
    }));

    // Start autoplay watcher with access to state
    let watcher_state = Arc::clone(&state);
    tokio::spawn(async move {
        autoplay_watcher(watcher_state).await;
    });

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/search", get(search_handler))
        .route("/api/stream", get(api_stream))
        .route("/api/play", post(play_handler))
        .route("/api/pause", post(pause_handler))
        .route("/api/resume", post(resume_handler))
        .route("/api/stop", post(stop_handler))
        .route("/api/volume/up", post(volume_up_handler))
        .route("/api/volume/down", post(volume_down_handler))
        .route("/api/next", post(next_handler))
        .route("/api/prev", post(prev_handler))
        .route("/api/status", get(status_handler))
        .route("/api/queue", get(queue_handler))
        .route("/api/queue_add", post(queue_add_handler))
        .route("/api/queue_clear", post(queue_clear_handler))
        .route("/api/like", post(like_handler))
        .route("/api/unlike", post(unlike_handler))
        .route("/api/bt/status", get(bt_status_handler))
        .route("/api/bt/connect", post(bt_connect_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:5050").await.unwrap();
    println!("Server running on http://0.0.0.0:5050");
    axum::serve(listener, app).await.unwrap();
}

/// Autoplay watcher: when a track ends and queue is empty, fetch related tracks
/// and auto-play them (Spotify Radio style).
async fn autoplay_watcher(state: AppState) {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        let (playing, queue_empty, current) = {
            let lock = match state.lock() {
                Ok(l) => l,
                Err(_) => continue,
            };
            let playing = lock.player.is_playing;
            let queue_empty = lock.player.queue.is_empty();
            let current = lock.player.current_track.clone();
            (playing, queue_empty, current)
        };

        if !playing || !queue_empty {
            continue;
        }

        let Some(t) = current else { continue };
        if t.id.is_empty() { continue; }

        // Fetch related/similar tracks
        let related = fetch_related_tracks(&t.id, &t.title, &t.artist);
        if related.is_empty() {
            continue;
        }

        // Add related tracks to queue
        {
            let mut lock = match state.lock() {
                Ok(l) => l,
                Err(_) => continue,
            };
            for r in &related {
                if lock.player.queue.len() < 200
                    && !lock.player.queue.iter().any(|q| q.id == r.id)
                    && r.id != t.id
                {
                    lock.player.queue.push(r.clone());
                }
            }
        }

        // Play the first related track
        if let Some(first) = related.first() {
            let pq = PlayQuery {
                url: first.url.clone(),
                title: Some(first.title.clone()),
                artist: Some(first.artist.clone()),
                thumbnail: Some(first.thumbnail.clone()),
                id: Some(first.id.clone()),
            };
            let _ = play_handler(State(Arc::clone(&state)), Query(pq)).await;
        }
    }
}

/// Fetch related/similar tracks via yt-dlp search.
fn fetch_related_tracks(id: &str, title: &str, artist: &str) -> Vec<Track> {
    let query = format!("{} {} similar", title, artist);
    let output = std::process::Command::new("yt-dlp")
        .args(&[
            "--flat-playlist",
            "--dump-json",
            &format!("ytsearch10:{}", query),
        ])
        .output();

    let mut tracks = Vec::new();
    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            if let Ok(item) = serde_json::from_str::<Value>(line) {
                let tid = item["id"].as_str().unwrap_or("").to_string();
                if tid.is_empty() || tid == id {
                    continue;
                }
                tracks.push(Track {
                    id: tid.clone(),
                    title: item["title"].as_str().unwrap_or("Unknown Title").to_string(),
                    artist: item["uploader"]
                        .as_str()
                        .or_else(|| item["channel"].as_str())
                        .unwrap_or("YouTube")
                        .to_string(),
                    thumbnail: format!("https://img.youtube.com/vi/{}/hqdefault.jpg", tid),
                    url: format!("https://www.youtube.com/watch?v={}", tid),
                });
            }
        }
    }
    tracks
}

/// Proxy YouTube audio through the server using yt-dlp to download chunks
/// and pipe them to the browser HTML5 <audio> element with CORS headers.
/// The audio never goes directly from YouTube to the browser — it always
/// passes through this local server first.
async fn api_stream(
    State(_state): State<AppState>,
    Query(sq): Query<StreamQuery>,
) -> Result<Response, axum::http::StatusCode> {
    let url = sq.url.clone();

    let mut child = match tokio::process::Command::new("yt-dlp")
        .args(&[
            "-f", "bestaudio",
            "-o", "-",
            "--no-check-certificate",
            "--no-warnings",
        ])
        .arg(&url)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
    };

    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    };

    let stream = YtDlpStream {
        stdout,
        buf: [0u8; 8192],
        eof: false,
    };
    let body = Body::from_stream(stream);

    Response::builder()
        .status(200)
        .header(header::CONTENT_TYPE, "audio/mpeg")
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Headers", "Range")
        .header("Accept-Ranges", "bytes")
        .header("Cache-Control", "no-cache")
        .body(body)
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)
}

/// Wraps a tokio::process::ChildStdout as a byte stream for axum response body.
/// yt-dlp downloads audio chunks and this stream pipes them to the browser.
struct YtDlpStream {
    stdout: tokio::process::ChildStdout,
    buf: [u8; 8192],
    eof: bool,
}

impl Stream for YtDlpStream {
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.eof {
            return Poll::Ready(None);
        }
        // Split self into fields to allow independent mutable borrows of buf and stdout
        let Self { stdout, buf, eof } = self.get_mut();
        let mut rb = ReadBuf::new(buf);
        match Pin::new(stdout).poll_read(cx, &mut rb) {
            Poll::Ready(Ok(())) => {
                let n = rb.filled().len();
                if n == 0 {
                    *eof = true;
                    return Poll::Ready(None);
                }
                Poll::Ready(Some(Ok(Bytes::copy_from_slice(&buf[..n]))))
            }
            Poll::Ready(Err(e)) => return Poll::Ready(Some(Err(e))),
            Poll::Pending => return Poll::Pending,
        }
    }
}

async fn index_handler() -> Html<&'static str> {
    Html(include_str!("index.html"))
}

async fn search_handler(
    State(state): State<AppState>,
    Query(sq): Query<SearchQuery>,
) -> Json<Vec<Track>> {
    let client = {
        let lock = state.lock().unwrap();
        lock.http_client.clone()
    };

    let query_str = sq.q.trim().to_string();
    let mut results = Vec::new();

    // 1. Try Fast InnerTube API (Android Client)
    let url = "https://www.youtube.com/youtubei/v1/search";
    let body = serde_json::json!({
        "context": {
            "client": {
                "clientName": "ANDROID",
                "clientVersion": "19.02.39",
                "hl": "en",
                "gl": "US"
            }
        },
        "query": query_str
    });

    if let Ok(res) = client.post(url).json(&body).send().await {
        if let Ok(v) = res.json::<serde_json::Value>().await {
            results = parse_yt_json(&v);
        }
    }

    // 2. Fallback to flat playlist search if InnerTube returned empty
    if results.is_empty() {
        let q_clone = query_str.clone();
        if let Ok(out) = tokio::task::spawn_blocking(move || {
            std::process::Command::new("yt-dlp")
                .args(&["--flat-playlist", "--dump-json", &format!("ytsearch16:{}", q_clone)])
                .output()
        }).await {
            if let Ok(out) = out {
                let text = String::from_utf8_lossy(&out.stdout);
                for line in text.lines() {
                    if let Ok(item) = serde_json::from_str::<serde_json::Value>(line) {
                        let id = item["id"].as_str().unwrap_or("").to_string();
                        if !id.is_empty() {
                            let title = item["title"].as_str().unwrap_or("Unknown Title").to_string();
                            let artist = item["uploader"].as_str().or_else(|| item["channel"].as_str()).unwrap_or("YouTube Artist").to_string();
                            results.push(Track {
                                id: id.clone(),
                                title,
                                artist,
                                thumbnail: format!("https://img.youtube.com/vi/{}/hqdefault.jpg", id),
                                url: format!("https://www.youtube.com/watch?v={}", id),
                            });
                        }
                    }
                }
            }
        }
    }

    Json(results)
}

fn parse_yt_json(v: &serde_json::Value) -> Vec<Track> {
    let mut results = Vec::new();
    let sections = v["contents"]["sectionListRenderer"]["contents"].as_array()
        .or_else(|| v["contents"]["twoColumnSearchResultsRenderer"]["primaryContents"]["sectionListRenderer"]["contents"].as_array());

    if let Some(secs) = sections {
        for section in secs {
            if let Some(items) = section["itemSectionRenderer"]["contents"].as_array() {
                for item in items {
                    if let Some(video) = item.get("videoRenderer") {
                        let id = video["videoId"].as_str().unwrap_or("").to_string();
                        if id.is_empty() { continue; }

                        let title = video["title"]["runs"][0]["text"].as_str()
                            .unwrap_or("Unknown Title").to_string();

                        let artist = video["ownerText"]["runs"][0]["text"].as_str()
                            .or_else(|| video["shortBylineText"]["runs"][0]["text"].as_str())
                            .unwrap_or("YouTube Artist").to_string();

                        results.push(Track {
                            id: id.clone(),
                            title,
                            artist,
                            thumbnail: format!("https://img.youtube.com/vi/{}/hqdefault.jpg", id),
                            url: format!("https://www.youtube.com/watch?v={}", id),
                        });
                        if results.len() >= 16 { break; }
                    }
                }
            }
            if results.len() >= 16 { break; }
        }
    }
    results
}

async fn play_handler(
    State(state): State<AppState>,
    Query(pq): Query<PlayQuery>,
) -> Json<serde_json::Value> {
    let video_url = pq.url.clone();
    let title = pq.title.unwrap_or_else(|| "Unknown Track".to_string());
    let artist = pq.artist.unwrap_or_else(|| "YouTube".to_string());
    let thumbnail = pq.thumbnail.unwrap_or_default();
    let id = pq.id.unwrap_or_default();

    let track = Track {
        id,
        title: title.clone(),
        artist,
        thumbnail,
        url: video_url.clone(),
    };

    let vol_percent = {
        let mut lock = state.lock().unwrap();
        if let Some(tr) = lock.player.current_track.as_ref().cloned() {
            lock.player.history.push(tr);
        }
        lock.player.current_track = Some(track);
        lock.player.is_playing = true;
        lock.player.status_msg = "Playing".to_string();
        (lock.player.volume * 100.0) as u32
    };

    // Kill any existing mpv stream instantly
    let _ = std::process::Command::new("pkill")
        .args(&["-f", "mpv.*mpv-music.sock"])
        .output();

    // Spawn tuned low-RAM mpv background process
    tokio::task::spawn_blocking(move || {
        let mut child = std::process::Command::new("mpv");
        child.args(&[
            "--no-video",
            "--no-terminal",
            "--demuxer-max-bytes=2MiB",
            "--demuxer-readahead-secs=5",
            &format!("--volume={}", vol_percent),
            "--ao=pulse",
            "--input-ipc-server=/tmp/mpv-music.sock",
            &video_url,
        ]);
        child.env("XDG_RUNTIME_DIR", "/run/user/1000");
        child.env("PULSE_SERVER", "unix:/run/user/1000/pulse/native");
        let _ = child.spawn();

        setup_pulse_bt_audio("04:CB:88:92:43:60");
    });

    Json(serde_json::json!({"status": "playing"}))
}

async fn pause_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let mut lock = state.lock().unwrap();
    send_mpv_ipc(r#"{"command": ["set_property", "pause", true]}"#);
    lock.player.is_playing = false;
    lock.player.status_msg = "Paused".to_string();
    Json(serde_json::json!({"status": "paused"}))
}

async fn resume_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let mut lock = state.lock().unwrap();
    send_mpv_ipc(r#"{"command": ["set_property", "pause", false]}"#);
    lock.player.is_playing = true;
    lock.player.status_msg = "Playing".to_string();
    Json(serde_json::json!({"status": "resumed"}))
}

async fn stop_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let mut lock = state.lock().unwrap();
    let _ = std::process::Command::new("pkill")
        .args(&["-f", "mpv.*mpv-music.sock"])
        .output();
    lock.player.is_playing = false;
    lock.player.current_track = None;
    lock.player.status_msg = "Stopped".to_string();
    Json(serde_json::json!({"status": "stopped"}))
}

async fn volume_up_handler(
    State(state): State<AppState>,
    Query(vq): Query<VolumeQuery>,
) -> Json<serde_json::Value> {
    let step = vq.step.unwrap_or(0.05);
    let mut lock = state.lock().unwrap();
    lock.player.volume = (lock.player.volume + step).min(1.0);
    let vol_percent = (lock.player.volume * 100.0) as u32;
    send_mpv_ipc(&format!(
        r#"{{"command": ["set_property", "volume", {}]}}"#,
        vol_percent
    ));
    Json(serde_json::json!({"volume": lock.player.volume}))
}

async fn volume_down_handler(
    State(state): State<AppState>,
    Query(vq): Query<VolumeQuery>,
) -> Json<serde_json::Value> {
    let step = vq.step.unwrap_or(0.05);
    let mut lock = state.lock().unwrap();
    lock.player.volume = (lock.player.volume - step).max(0.0);
    let vol_percent = (lock.player.volume * 100.0) as u32;
    send_mpv_ipc(&format!(
        r#"{{"command": ["set_property", "volume", {}]}}"#,
        vol_percent
    ));
    Json(serde_json::json!({"volume": lock.player.volume}))
}

async fn next_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let next_track = {
        let mut lock = state.lock().unwrap();
        if lock.player.queue.is_empty() {
            None
        } else {
            Some(lock.player.queue.remove(0))
        }
    };
    match next_track {
        Some(t) => {
            let pq = PlayQuery {
                url: t.url,
                title: Some(t.title),
                artist: Some(t.artist),
                thumbnail: Some(t.thumbnail),
                id: Some(t.id),
            };
            return play_handler(State(state), Query(pq)).await;
        }
        None => stop_handler(State(state)).await,
    }
}

async fn prev_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let prev_track = {
        let mut lock = state.lock().unwrap();
        lock.player.history.pop()
    };
    match prev_track {
        Some(t) => {
            let pq = PlayQuery {
                url: t.url,
                title: Some(t.title),
                artist: Some(t.artist),
                thumbnail: Some(t.thumbnail),
                id: Some(t.id),
            };
            return play_handler(State(state), Query(pq)).await;
        }
        None => stop_handler(State(state)).await,
    }
}

async fn status_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let lock = state.lock().unwrap();
    let (pos, duration) = get_mpv_pos_duration();
    Json(serde_json::json!({
        "current_track": lock.player.current_track,
        "is_playing": lock.player.is_playing,
        "volume": lock.player.volume,
        "status_msg": lock.player.status_msg,
        "pos": pos,
        "duration": duration,
        "queue_len": lock.player.queue.len(),
        "liked_count": lock.player.liked.len(),
    }))
}

async fn queue_handler(State(state): State<AppState>) -> Json<Value> {
    let lock = state.lock().unwrap();
    let q: Vec<Value> = lock
        .player
        .queue
        .iter()
        .map(|t| serde_json::to_value(t).unwrap())
        .collect();
    Json(serde_json::json!(q))
}

async fn queue_add_handler(
    State(state): State<AppState>,
    Json(track): Json<Track>,
) -> Json<Value> {
    let mut lock = state.lock().unwrap();
    if lock.player.queue.len() < 200 && !lock.player.queue.iter().any(|q| q.id == track.id) {
        lock.player.queue.push(track);
    }
    Json(serde_json::json!({"status": "queued"}))
}

async fn queue_clear_handler(State(state): State<AppState>) -> Json<Value> {
    let mut lock = state.lock().unwrap();
    lock.player.queue.clear();
    Json(serde_json::json!({"status": "cleared"}))
}

async fn like_handler(
    State(state): State<AppState>,
    Json(track): Json<Track>,
) -> Json<Value> {
    let mut lock = state.lock().unwrap();
    if !lock.player.liked.iter().any(|t| t.id == track.id) {
        lock.player.liked.push(track);
    }
    Json(serde_json::json!({"status": "liked"}))
}

async fn unlike_handler(
    State(state): State<AppState>,
    Json(track): Json<Track>,
) -> Json<Value> {
    let mut lock = state.lock().unwrap();
    lock.player.liked.retain(|t| t.id != track.id);
    Json(serde_json::json!({"status": "unliked"}))
}

async fn bt_status_handler() -> Json<serde_json::Value> {
    let connected = is_bt_connected("04:CB:88:92:43:60").await;
    Json(serde_json::json!({
        "device": "JBL Flip Essential",
        "mac": "04:CB:88:92:43:60",
        "connected": connected,
        "sink_name": if connected { "JBL Flip Essential (Active)" } else { "Internal Audio" }
    }))
}

async fn bt_connect_handler() -> Json<serde_json::Value> {
    let success = run_bt_connect_script();
    Json(serde_json::json!({
        "status": if success { "connected" } else { "failed" }
    }))
}

fn send_mpv_ipc(cmd: &str) {
    if let Ok(mut stream) = UnixStream::connect("/tmp/mpv-music.sock") {
        let mut full_cmd = cmd.to_string();
        full_cmd.push('\n');
        let _ = stream.write_all(full_cmd.as_bytes());
    }
}

fn get_mpv_pos_duration() -> (f64, f64) {
    let mut pos = 0.0;
    let mut duration = 0.0;

    if let Ok(mut stream) = UnixStream::connect("/tmp/mpv-music.sock") {
        let _ = stream.write_all(b"{\"command\": [\"get_property\", \"time-pos\"]}\n");
        let mut buf = [0u8; 256];
        if let Ok(n) = stream.read(&mut buf) {
            let s = String::from_utf8_lossy(&buf[..n]);
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                if let Some(p) = v["data"].as_f64() {
                    pos = p;
                }
            }
        }
    }

    if let Ok(mut stream) = UnixStream::connect("/tmp/mpv-music.sock") {
        let _ = stream.write_all(b"{\"command\": [\"get_property\", \"duration\"]}\n");
        let mut buf = [0u8; 256];
        if let Ok(n) = stream.read(&mut buf) {
            let s = String::from_utf8_lossy(&buf[..n]);
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                if let Some(d) = v["data"].as_f64() {
                    duration = d;
                }
            }
        }
    }

    (pos, duration)
}

async fn is_bt_connected(mac: &str) -> bool {
    if let Ok(out) = std::process::Command::new("bluetoothctl")
        .args(&["info", mac])
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        return text.contains("Connected: yes");
    }
    false
}

fn run_bt_connect_script() -> bool {
    let output = std::process::Command::new("bash")
        .arg("/home/afine/connect_jbl.sh")
        .output();
    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);
        return text.contains("SUCCESS");
    }
    false
}

fn setup_pulse_bt_audio(mac: &str) {
    let mac_underscore = mac.replace(":", "_");
    let card_name = format!("bluez_card.{}", mac_underscore);

    let _ = std::process::Command::new("pactl")
        .env("XDG_RUNTIME_DIR", "/run/user/1000")
        .env("PULSE_SERVER", "unix:/run/user/1000/pulse/native")
        .args(&["set-card-profile", &card_name, "a2dp_sink"])
        .output();

    if let Ok(out) = std::process::Command::new("pactl")
        .env("XDG_RUNTIME_DIR", "/run/user/1000")
        .env("PULSE_SERVER", "unix:/run/user/1000/pulse/native")
        .args(&["list", "sinks", "short"])
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let sink_name = parts[1];
                if sink_name.contains(&mac_underscore) || sink_name.contains("bluez") {
                    let _ = std::process::Command::new("pactl")
                        .env("XDG_RUNTIME_DIR", "/run/user/1000")
                        .env("PULSE_SERVER", "unix:/run/user/1000/pulse/native")
                        .args(&["set-default-sink", sink_name])
                        .output();

                    let _ = std::process::Command::new("pactl")
                        .env("XDG_RUNTIME_DIR", "/run/user/1000")
                        .env("PULSE_SERVER", "unix:/run/user/1000/pulse/native")
                        .args(&["set-sink-mute", sink_name, "0"])
                        .output();

                    let _ = std::process::Command::new("pactl")
                        .env("XDG_RUNTIME_DIR", "/run/user/1000")
                        .env("PULSE_SERVER", "unix:/run/user/1000/pulse/native")
                        .args(&["set-sink-volume", sink_name, "100%"])
                        .output();

                    if let Ok(inputs) = std::process::Command::new("pactl")
                        .env("XDG_RUNTIME_DIR", "/run/user/1000")
                        .env("PULSE_SERVER", "unix:/run/user/1000/pulse/native")
                        .args(&["list", "sink-inputs", "short"])
                        .output()
                    {
                        let input_text = String::from_utf8_lossy(&inputs.stdout);
                        for in_line in input_text.lines() {
                            let in_parts: Vec<&str> = in_line.split_whitespace().collect();
                            if !in_parts.is_empty() {
                                let input_id = in_parts[0];
                                let _ = std::process::Command::new("pactl")
                                    .env("XDG_RUNTIME_DIR", "/run/user/1000")
                                    .env("PULSE_SERVER", "unix:/run/user/1000/pulse/native")
                                    .args(&["move-sink-input", input_id, sink_name])
                                    .output();
                            }
                        }
                    }
                    break;
                }
            }
        }
    }
}
