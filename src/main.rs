use axum::{
    extract::{Query, State},
    response::{Html, Json, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;
mod media;

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
    play_mode: String,
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
    mode: Option<String>,
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
            play_mode: "bt".to_string(),
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
        .route("/api/bt/disconnect", post(bt_disconnect_handler))
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
            let playing = lock.player.is_playing && lock.player.play_mode == "bt";
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
                mode: Some("bt".to_string()),
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

async fn api_stream(
    State(_state): State<AppState>,
    Query(sq): Query<StreamQuery>,
) -> Result<Response, axum::http::StatusCode> {
    media::open_stream(&sq.url).await
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
    let browser_mode = pq.mode.as_deref() == Some("browser");
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
        lock.player.play_mode = if browser_mode { "browser" } else { "bt" }.to_string();
        lock.player.status_msg = "Playing".to_string();
        (lock.player.volume * 100.0) as u32
    };

    // Kill any existing mpv stream instantly
    let _ = std::process::Command::new("pkill")
        .args(&["-f", "mpv.*mpv-music.sock"])
        .output();

    if browser_mode {
        return Json(serde_json::json!({"status": "playing", "mode": "browser"}));
    }

    let result = tokio::task::spawn_blocking(move || {
        setup_pulse_bt_audio("04:CB:88:92:43:60");
        let mut command = audio_command("mpv");
        command.args([
            "--no-video", "--no-terminal", "--demuxer-max-bytes=2MiB",
            "--demuxer-readahead-secs=5", &format!("--volume={}", vol_percent),
            "--ao=pulse", "--input-ipc-server=/tmp/mpv-music.sock", &video_url,
        ]);
        command.spawn().map(|mut child| {
            // Reap mpv when it exits without blocking an async runtime worker.
            std::thread::spawn(move || { let _ = child.wait(); });
        })
    }).await;
    match result {
        Ok(Ok(())) => Json(serde_json::json!({"status": "playing"})),
        other => {
            let error = format!("Unable to start mpv: {other:?}");
            eprintln!("{error}");
            let mut lock = state.lock().unwrap();
            lock.player.is_playing = false;
            lock.player.status_msg = error.clone();
            Json(serde_json::json!({"status": "failed", "error": error}))
        }
    }
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
                mode: Some(state.lock().unwrap().player.play_mode.clone()),
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
                mode: Some(state.lock().unwrap().player.play_mode.clone()),
            };
            return play_handler(State(state), Query(pq)).await;
        }
        None => stop_handler(State(state)).await,
    }
}

async fn status_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let lock = state.lock().unwrap();
    let (pos, duration) = if lock.player.play_mode == "bt" { get_mpv_pos_duration() } else { (0.0, 0.0) };
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
    let info = bluetooth_info().await;
    Json(serde_json::json!({
        "device": "JBL Flip Essential",
        "mac": "04:CB:88:92:43:60",
        "connected": info.as_ref().map(|text| text.contains("Connected: yes")).unwrap_or(false),
        "error": info.err(),
    }))
}

async fn bluetooth_info() -> Result<String, String> {
    bluetooth_command(&["info", "04:CB:88:92:43:60"]).await
}

async fn bluetooth_command(args: &[&str]) -> Result<String, String> {
    let mut command = tokio::process::Command::new("bluetoothctl");
    command.args(args).kill_on_drop(true);
    match tokio::time::timeout(std::time::Duration::from_secs(20), command.output()).await {
        Ok(Ok(output)) => {
            let text = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
            if output.status.success() { Ok(text) } else { Err(text) }
        }
        Ok(Err(error)) => Err(format!("Cannot run bluetoothctl: {error}")),
        Err(_) => Err("bluetoothctl timed out after 20 seconds".to_string()),
    }
}

async fn bt_connect_handler() -> Json<serde_json::Value> {
    let script = std::env::var("MUSIC_BT_CONNECT_SCRIPT")
        .unwrap_or_else(|_| "/home/afine/connect_jbl.sh".to_string());
    let mut command = tokio::process::Command::new("bash");
    command.arg(&script).kill_on_drop(true);
    let result = tokio::time::timeout(std::time::Duration::from_secs(30), command.output()).await;
    let error = match result {
        Ok(Ok(output)) if output.status.success() => None,
        Ok(Ok(output)) => Some(format!("Connect script failed ({}): {}{}", output.status,
            String::from_utf8_lossy(&output.stderr), String::from_utf8_lossy(&output.stdout))),
        Ok(Err(error)) => Some(format!("Cannot run {script}: {error}")),
        Err(_) => Some("Connect script timed out after 30 seconds".to_string()),
    };
    let info = bluetooth_info().await;
    let connected = info.as_ref().map(|text| text.contains("Connected: yes")).unwrap_or(false);
    let error = if connected { None } else {
        error.or_else(|| info.err()).or_else(|| Some("Script finished but JBL is not connected. Check pairing, adapter power and service permissions.".to_string()))
    };
    Json(serde_json::json!({"status": if connected { "connected" } else { "failed" }, "error": error}))
}

async fn bt_disconnect_handler() -> Json<serde_json::Value> {
    match bluetooth_command(&["disconnect", "04:CB:88:92:43:60"]).await {
        Ok(_) => Json(serde_json::json!({"status": "disconnected"})),
        Err(error) => Json(serde_json::json!({"status": "failed", "error": error})),
    }
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

fn audio_command(program: &str) -> std::process::Command {
    let mut command = std::process::Command::new(program);
    let runtime = std::env::var("XDG_RUNTIME_DIR").ok().or_else(|| {
        let output = std::process::Command::new("id").arg("-u").output().ok()?;
        if !output.status.success() { return None; }
        let uid = String::from_utf8(output.stdout).ok()?;
        Some(format!("/run/user/{}", uid.trim()))
    });
    if let Some(runtime) = runtime {
        command.env("XDG_RUNTIME_DIR", &runtime);
        if std::env::var_os("PULSE_SERVER").is_none() {
            command.env("PULSE_SERVER", format!("unix:{runtime}/pulse/native"));
        }
    }
    command
}

fn setup_pulse_bt_audio(mac: &str) {
    let mac_underscore = mac.replace(":", "_");
    let card_name = format!("bluez_card.{}", mac_underscore);

    let _ = audio_command("pactl")
        .args(&["set-card-profile", &card_name, "a2dp_sink"])
        .output();

    if let Ok(out) = audio_command("pactl")
        .args(&["list", "sinks", "short"])
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let sink_name = parts[1];
                if sink_name.contains(&mac_underscore) {
                    let _ = audio_command("pactl")
                        .args(&["set-default-sink", sink_name])
                        .output();

                    let _ = audio_command("pactl")
                        .args(&["set-sink-mute", sink_name, "0"])
                        .output();

                    let _ = audio_command("pactl")
                        .args(&["set-sink-volume", sink_name, "100%"])
                        .output();

                    if let Ok(inputs) = audio_command("pactl")
                        .args(&["list", "sink-inputs", "short"])
                        .output()
                    {
                        let input_text = String::from_utf8_lossy(&inputs.stdout);
                        for in_line in input_text.lines() {
                            let in_parts: Vec<&str> = in_line.split_whitespace().collect();
                            if !in_parts.is_empty() {
                                let input_id = in_parts[0];
                                let _ = audio_command("pactl")
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
