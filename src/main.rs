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
mod metadata_budget;
mod playback;
mod runtime_user;
mod resolved_media;
mod speaker_process;

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
    speaker: Option<speaker_process::SpeakerProcess>,
    playback_epoch: u64,
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
struct SetVolumeQuery {
    val: f32,
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
        speaker: None,
        playback_epoch: 0,
    }));

    // Start autoplay watcher with access to state
    let watcher_state = Arc::clone(&state);
    tokio::spawn(async move {
        autoplay_watcher(watcher_state).await;
    });

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/icons.svg", get(icons_handler))
        .route("/api/search", get(search_handler))
        .route("/api/stream", get(api_stream))
        .route("/api/play", post(play_handler))
        .route("/api/pause", post(pause_handler))
        .route("/api/resume", post(resume_handler))
        .route("/api/stop", post(stop_handler))
        .route("/api/volume", post(volume_handler))
        .route("/api/volume/up", post(volume_up_handler))
        .route("/api/volume/down", post(volume_down_handler))
        .route("/api/next", post(next_handler))
        .route("/api/prev", post(prev_handler))
        .route("/api/status", get(status_handler))
        .route("/api/queue", get(queue_handler))
        .route("/api/queue_add", post(queue_add_handler))
        .route("/api/queue_clear", post(queue_clear_handler))
        .route("/api/liked", get(liked_handler))
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

/// Prefetch related tracks without interrupting the current song.
/// Browser advancement is driven by the media element's ended event.
async fn autoplay_watcher(state: AppState) {
    let mut attempts = metadata_budget::PrefetchAttempts::default();
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        // The opt-in transfer trial must not immediately restart a speculative
        // Python resolver during steady audio. Explicit Next remains on demand.
        if resolved_media::is_active() { continue; }
        let current = {
            let lock = state.lock().unwrap();
            if !attempts.reserve(lock.playback_epoch, lock.player.is_playing, lock.player.queue.is_empty()) { continue; }
            lock.player.current_track.clone().map(|track| (track, lock.playback_epoch))
        };
        if let Some((current, epoch)) = current {
            let related = fetch_related_tracks(&current.id, &current.title, &current.artist).await;
            let mut lock = state.lock().unwrap();
            if lock.playback_epoch == epoch && lock.player.current_track.as_ref().map(|t| &t.url) == Some(&current.url) {
                lock.player.enqueue_related(related);
            }
        }
    }
}

/// Fetch related/similar tracks via yt-dlp search.
async fn fetch_related_tracks(id: &str, title: &str, artist: &str) -> Vec<Track> {
    let query = format!("{} {} similar", title, artist);
    let mut command = tokio::process::Command::new("yt-dlp");
    command.args(["--flat-playlist", "--dump-json", "--socket-timeout", "10", "--retries", "1",
        &format!("ytsearch10:{}", query)]).kill_on_drop(true);
    let output = metadata_budget::helper_output(command).await;

    let mut tracks = Vec::new();
    if let Ok(bytes) = output {
        let text = String::from_utf8_lossy(&bytes);
        for line in text.lines() {
            if let Ok(item) = serde_json::from_str::<Value>(line) {
                let tid = item["id"].as_str().unwrap_or("").to_string();
                if tid.is_empty() || tid == id {
                    continue;
                }
                if tracks.len() >= 10 { break; }
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

async fn icons_handler() -> Response {
    Response::builder()
        .header("Content-Type", "image/svg+xml")
        .header("Cache-Control", "no-cache")
        .body(axum::body::Body::from(include_str!("icons.svg")))
        .expect("static SVG response")
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
    if query_str.is_empty() || query_str.len() > 512 { return Json(Vec::new()); }
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

    // Bound the entire native HTTP path, not just its individual socket reads.
    let native = tokio::time::timeout(std::time::Duration::from_secs(12), async {
        let response = client.post(url).json(&body).send().await.ok()?;
        metadata_budget::bounded_json(response).await
    }).await;
    if let Ok(Some(value)) = native { results = parse_yt_json(&value); }

    // Existing compatibility fallback remains explicit; it is NOT native
    // extraction or a proof that the helper's own RSS fits the total target.
    if results.is_empty() {
        let mut command = tokio::process::Command::new("yt-dlp");
        command.args(["--flat-playlist", "--dump-json", "--socket-timeout", "10", "--retries", "1",
                      &format!("ytsearch16:{}", query_str)]);
        match metadata_budget::helper_output(command).await {
            Ok(bytes) => {
                for line in String::from_utf8_lossy(&bytes).lines() {
                    if let Ok(item) = serde_json::from_str::<serde_json::Value>(line) {
                        let id = item["id"].as_str().unwrap_or("").to_string();
                        if !id.is_empty() {
                            let title = item["title"].as_str().unwrap_or("Unknown Title").to_string();
                            let artist = item["uploader"].as_str().or_else(|| item["channel"].as_str()).unwrap_or("YouTube Artist").to_string();
                            results.push(Track {
                                id: id.clone(), title, artist,
                                thumbnail: format!("https://img.youtube.com/vi/{}/hqdefault.jpg", id),
                                url: format!("https://www.youtube.com/watch?v={}", id),
                            });
                            if results.len() >= 16 { break; }
                        }
                    }
                }
            }
            Err(code) => eprintln!("Search compatibility fallback: {code}"),
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
    let mode = pq.mode.unwrap_or_else(|| "bt".to_string());
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

    start_track(state, track, mode, true).await
}

async fn start_track(state: AppState, track: Track, mode: String, remember: bool) -> Json<Value> {
    let browser_mode = mode == "browser";
    let video_url = track.url.clone();
    let response_track = track.clone();
    let (vol_percent, epoch, previous) = {
        let mut lock = state.lock().unwrap();
        lock.playback_epoch = lock.playback_epoch.wrapping_add(1);
        let epoch = lock.playback_epoch;
        let previous = lock.speaker.take();
        if remember { lock.player.remember_current(&track); }
        lock.player.current_track = Some(track);
        lock.player.is_playing = true;
        lock.player.play_mode = if browser_mode { "browser" } else { "bt" }.to_string();
        lock.player.status_msg = "Playing".to_string();
        ((lock.player.volume * 100.0) as u32, epoch, previous)
    };

    if let Some(previous) = previous {
        if let Err(code) = previous.stop().await { return failed_start(&state,epoch,code); }
    }
    if browser_mode {
        if state.lock().unwrap().playback_epoch != epoch {
            return Json(serde_json::json!({"status":"failed","error":"Playback request superseded"}));
        }
        return Json(serde_json::json!({"status":"playing","mode":"browser","track":response_track}));
    }

    let prepared = tokio::task::spawn_blocking(move || {
        setup_pulse_bt_audio("04:CB:88:92:43:60");
        let ipc = speaker_process::PrivateIpc::create()?;
        let mut command = audio_command("mpv");
        command.args([
            "--no-video", "--no-terminal", "--demuxer-max-bytes=2MiB",
            "--demuxer-readahead-secs=5", &format!("--volume={}",vol_percent),
            "--ao=pulse", &format!("--input-ipc-server={}",ipc.socket().display()), &video_url,
        ]);
        if let Ok(runtime) = std::env::var("MUSIC_YTDLP_JS_RUNTIME") {
            if !runtime.is_empty() { command.arg(format!("--ytdl-raw-options=js-runtimes={runtime}")); }
        }
        Ok::<_, std::io::Error>((command,ipc))
    }).await;
    match prepared {
        Ok(Ok((command,ipc))) => match install_speaker(&state,epoch,command,ipc) {
            Ok(()) => Json(serde_json::json!({"status":"playing","mode":"bt","track":response_track})),
            Err(code) => failed_start(&state,epoch,code),
        },
        _ => failed_start(&state,epoch,"speaker_prepare_failed"),
    }
}

fn failed_start(state: &AppState, epoch: u64, code: &'static str) -> Json<Value> {
    let mut lock = state.lock().unwrap();
    if lock.playback_epoch == epoch {
        lock.player.is_playing = false;
        lock.player.status_msg = code.into();
    }
    Json(serde_json::json!({"status":"failed","error":code}))
}
fn install_speaker(state: &AppState, epoch: u64, command: std::process::Command,
                   ipc: speaker_process::PrivateIpc) -> Result<(), &'static str> {
    let mut lock = state.lock().unwrap();
    if lock.playback_epoch != epoch { return Err("speaker_request_superseded"); }
    lock.speaker = Some(speaker_process::SpeakerProcess::spawn(command,ipc).map_err(|_| "speaker_spawn_failed")?);
    Ok(())
}

async fn pause_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let mut lock = state.lock().unwrap();
    send_mpv_ipc(&lock,r#"{"command": ["set_property", "pause", true]}"#);
    lock.player.is_playing = false;
    lock.player.status_msg = "Paused".to_string();
    Json(serde_json::json!({"status": "paused"}))
}

async fn resume_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let mut lock = state.lock().unwrap();
    send_mpv_ipc(&lock,r#"{"command": ["set_property", "pause", false]}"#);
    lock.player.is_playing = true;
    lock.player.status_msg = "Playing".to_string();
    Json(serde_json::json!({"status": "resumed"}))
}

async fn stop_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let previous = {
        let mut lock = state.lock().unwrap();
        lock.playback_epoch = lock.playback_epoch.wrapping_add(1);
        let previous = lock.speaker.take();
        lock.player.is_playing = false;
        lock.player.current_track = None;
        lock.player.status_msg = "Stopped".to_string();
        previous
    };
    if let Some(previous) = previous {
        if let Err(code) = previous.stop().await {
            return Json(serde_json::json!({"status":"failed","error":code}));
        }
    }
    Json(serde_json::json!({"status":"stopped"}))
}

async fn volume_up_handler(
    State(state): State<AppState>,
    Query(vq): Query<VolumeQuery>,
) -> Json<serde_json::Value> {
    let step = vq.step.unwrap_or(0.05);
    let mut lock = state.lock().unwrap();
    lock.player.volume = (lock.player.volume + step).min(1.0);
    let vol_percent = (lock.player.volume * 100.0) as u32;
    send_mpv_ipc(&lock,&format!(
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
    send_mpv_ipc(&lock,&format!(
        r#"{{"command": ["set_property", "volume", {}]}}"#,
        vol_percent
    ));
    Json(serde_json::json!({"volume": lock.player.volume}))
}

async fn next_handler(State(state): State<AppState>) -> Json<Value> {
    let seed = {
        let lock = state.lock().unwrap();
        if lock.player.queue.is_empty() { lock.player.current_track.clone() } else { None }
    };
    if let Some(seed) = seed {
        let related = fetch_related_tracks(&seed.id, &seed.title, &seed.artist).await;
        let mut lock = state.lock().unwrap();
        if lock.player.current_track.as_ref().map(|t| &t.url) != Some(&seed.url) {
            return Json(serde_json::json!({"status": "failed", "error": "Track changed; retry Next"}));
        }
        lock.player.enqueue_related(related);
    }
    let selection = {
        let mut lock = state.lock().unwrap();
        if lock.player.queue.is_empty() { None } else {
            Some((lock.player.queue.remove(0), lock.player.play_mode.clone()))
        }
    };
    match selection {
        Some((track, mode)) => start_track(state, track, mode, true).await,
        None => Json(serde_json::json!({"status": "failed", "error": "No next track available. Add songs to the queue or retry."})),
    }
}

async fn prev_handler(State(state): State<AppState>) -> Json<Value> {
    let selection = {
        let mut lock = state.lock().unwrap();
        lock.player.previous_track().map(|track| (track, lock.player.play_mode.clone()))
    };
    match selection {
        Some((track, mode)) => start_track(state, track, mode, false).await,
        None => Json(serde_json::json!({"status": "failed", "error": "No previous track in this session"})),
    }
}

async fn volume_handler(State(state): State<AppState>, Query(vq): Query<SetVolumeQuery>) -> Json<Value> {
    if !vq.val.is_finite() || !(0.0..=100.0).contains(&vq.val) {
        return Json(serde_json::json!({"status": "failed", "error": "Volume must be between 0 and 100"}));
    }
    let mut lock = state.lock().unwrap();
    lock.player.volume = vq.val / 100.0;
    if lock.player.play_mode == "bt" {
        send_mpv_ipc(&lock,&format!(r#"{{"command":["set_property","volume",{}]}}"#, vq.val));
    }
    Json(serde_json::json!({"volume": lock.player.volume}))
}

async fn status_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let lock = state.lock().unwrap();
    let (pos, duration) = if lock.player.play_mode == "bt" { get_mpv_pos_duration(&lock) } else { (0.0, 0.0) };
    Json(serde_json::json!({
        "current_track": lock.player.current_track,
        "is_playing": lock.player.is_playing,
        "volume": lock.player.volume,
        "status_msg": lock.player.status_msg,
        "pos": pos,
        "duration": duration,
        "queue_len": lock.player.queue.len(),
        "liked_count": lock.player.liked.len(),
        "stream_transport": media::configured_transport_name(),
        "rust_stream_active": resolved_media::is_active(),
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

async fn liked_handler(State(state): State<AppState>) -> Json<Vec<Track>> {
    Json(state.lock().unwrap().player.liked.clone())
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

fn send_mpv_ipc(state: &AppStateInner, cmd: &str) {
    let Some(path) = state.speaker.as_ref().and_then(|speaker| speaker.socket()) else { return };
    if let Ok(mut stream) = UnixStream::connect(path) {
        let mut full_cmd = cmd.to_string();
        full_cmd.push('\n');
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(300)));
        let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(300)));
        let _ = stream.write_all(full_cmd.as_bytes());
    }
}

fn get_mpv_pos_duration(state: &AppStateInner) -> (f64, f64) {
    let Some(path) = state.speaker.as_ref().and_then(|speaker| speaker.socket()) else { return (0.0,0.0) };
    let mut pos = 0.0;
    let mut duration = 0.0;

    if let Ok(mut stream) = UnixStream::connect(path) {
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(300)));
        let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(300)));
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

    if let Ok(mut stream) = UnixStream::connect(path) {
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(300)));
        let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(300)));
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
    let runtime = runtime_user::runtime_directory();
    if let Some(runtime) = runtime {
        command.env("XDG_RUNTIME_DIR", &runtime);
        if std::env::var_os("PULSE_SERVER").is_none() {
            command.env("PULSE_SERVER", format!("unix:{}/pulse/native",runtime.display()));
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


#[cfg(test)]
mod api_tests {
    use super::*;
    fn state() -> AppState {
        Arc::new(Mutex::new(AppStateInner {
            player: PlayerState { current_track: None, is_playing: false, volume: 0.8,
                status_msg: String::new(), history: vec![], queue: vec![], liked: vec![], play_mode: "browser".into() },
            http_client: reqwest::Client::new(),
            speaker: None,
            playback_epoch: 0,
        }))
    }

    #[tokio::test]
    async fn absolute_volume_updates_browser_state() {
        let state = state();
        let result = volume_handler(State(state.clone()), Query(SetVolumeQuery { val: 35.0 })).await;
        assert_eq!(result.0["volume"], serde_json::json!(0.35f32));
        assert_eq!(state.lock().unwrap().player.volume, 0.35);
    }

    #[tokio::test]
    async fn invalid_volume_does_not_mutate_state() {
        let state = state();
        for val in [-1.0, 101.0, f32::NAN, f32::INFINITY] {
            let result = volume_handler(State(state.clone()), Query(SetVolumeQuery { val })).await;
            assert_eq!(result.0["status"], "failed");
            assert_eq!(state.lock().unwrap().player.volume, 0.8);
        }
    }

    #[tokio::test]
    async fn missing_previous_does_not_stop_current_playback() {
        let state = state();
        state.lock().unwrap().player.is_playing = true;
        let result = prev_handler(State(state.clone())).await;
        assert_eq!(result.0["status"], "failed");
        assert!(state.lock().unwrap().player.is_playing);
    }

    #[tokio::test]
    async fn icons_are_served_from_embedded_sprite() {
        let response = icons_handler().await;
        assert_eq!(response.headers()["Content-Type"], "image/svg+xml");
        let bytes = axum::body::to_bytes(response.into_body(), 100_000).await.unwrap();
        let svg = std::str::from_utf8(&bytes).unwrap();
        assert!(svg.contains("id=\"previous\""));
        assert!(svg.contains("id=\"next\""));
    }
    #[tokio::test] async fn stopped_request_cannot_install_a_late_speaker() {
        let state=state(); let old_epoch=state.lock().unwrap().playback_epoch;
        stop_handler(State(state.clone())).await;
        let ipc=speaker_process::PrivateIpc::create().unwrap();
        let result=install_speaker(&state,old_epoch,std::process::Command::new("/no-such-test-program"),ipc);
        assert_eq!(result,Err("speaker_request_superseded"));
        assert!(state.lock().unwrap().speaker.is_none());
    }
    #[tokio::test] async fn stale_start_failure_cannot_mark_a_new_browser_track_stopped() {
        let state=state();let old_epoch=state.lock().unwrap().playback_epoch;
        let track=Track{id:"fixture".into(),title:"Fixture".into(),artist:"Fixture".into(),thumbnail:String::new(),url:"https://www.youtube.com/watch?v=Jm7Satp7Zqk".into()};
        let result=start_track(state.clone(),track,"browser".into(),true).await;
        assert_eq!(result.0["status"],"playing");
        failed_start(&state,old_epoch,"speaker_prepare_failed");
        assert!(state.lock().unwrap().player.is_playing);
    }
    #[tokio::test] async fn stop_clears_owned_speaker_and_does_not_touch_another_child() {
        let state=state();let mut other=tokio::process::Command::new("sleep").arg("30").kill_on_drop(true).spawn().unwrap();
        let mut command=std::process::Command::new("sleep");command.arg("30");
        install_speaker(&state,0,command,speaker_process::PrivateIpc::create().unwrap()).unwrap();
        let result=stop_handler(State(state.clone())).await;
        assert_eq!(result.0["status"],"stopped");
        assert!(state.lock().unwrap().speaker.is_none());
        assert!(other.try_wait().unwrap().is_none());other.kill().await.unwrap();
    }

    #[tokio::test]
    async fn oversized_or_empty_search_never_starts_an_upstream_request() {
        for q in [String::new(), " ".into(), "x".repeat(513)] {
            let result = search_handler(State(state()), Query(SearchQuery { q })).await;
            assert!(result.0.is_empty());
        }
    }

}
