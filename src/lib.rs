//! Shared library target for the Aysha Music Android APK.
//!
//! Phase 2 scope: JNI entry point, a local Axum server bound to
//! `127.0.0.1:5050`, the persistent Telegram queue/quota endpoints and the
//! Telegram login (`send_code` -> `verify_code` -> optional 2FA).
//!
//! Later phases wire in the pure-Rust YouTube extractor, the rodio player and
//! the `/api/tg/quota` sidebar gauge in the player page.

#![cfg(feature = "phone")]
mod android_catalog;
mod recommendations;
mod listening;
mod range_media;
mod pcm_stream;
mod tg_storage;

pub mod player;
pub mod tg_auth;
pub mod tg_cache;
pub mod tg_session;
pub mod youtube;

use axum::{
    extract::{Extension, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::{Arc, OnceLock};
use tg_auth::TgOutcome;
use tg_cache::TelegramCacheManager;

/// Embedded player page (same page the Linux server serves at `/`).
const INDEX_HTML: &str = include_str!("index.html");

/// `(files_dir, cache_dir)` supplied by Android on startup.
static PATHS: OnceLock<(String, String)> = OnceLock::new();
static SERVER_STARTED: OnceLock<()> = OnceLock::new();

/// Playback thread, started lazily on the first `/api/play`.
static PLAYER: OnceLock<player::Player> = OnceLock::new();
static TG_CACHE: OnceLock<Arc<TelegramCacheManager>> = OnceLock::new();
fn cache_manager() -> Option<Arc<TelegramCacheManager>> { TG_CACHE.get().cloned() }
/// Shared HTTP client for YouTube resolution and media streaming.
static HTTP_CLIENT: OnceLock<Option<reqwest::Client>> = OnceLock::new();
/// Visitor identity for InnerTube, cached for the process and on disk.
static VISITOR: OnceLock<youtube::Visitor> = OnceLock::new();

/// `filesDir/yt_visitor.json` - survives app restarts once fetched.
fn visitor_cache_path() -> Option<std::path::PathBuf> {
    paths().map(|(files, _)| std::path::Path::new(files).join("yt_visitor.json"))
}

async fn visitor(client: &reqwest::Client) -> &'static youtube::Visitor {
    if let Some(cached) = VISITOR.get() {
        return cached;
    }
    let fetched = youtube::ensure_visitor(client, visitor_cache_path().as_deref()).await;
    VISITOR.get_or_init(|| fetched)
}

/// Only `/api/play` may open the audio device. Read/control endpoints must
/// never acquire hardware just because the UI polls them.
fn started_player() -> Option<&'static player::Player> {
    PLAYER.get()
}

fn player() -> &'static player::Player {
    PLAYER.get_or_init(player::Player::spawn)
}

fn http_client() -> Option<&'static reqwest::Client> {
    HTTP_CLIENT
        .get_or_init(|| youtube::http_client().ok())
        .as_ref()
}

/// Streams media bytes. Unlike `http_client` it has no total request timeout,
/// because a track streams for minutes and a 12 s cap cut every song off.
fn media_client() -> Option<&'static reqwest::Client> {
    static MEDIA_CLIENT: OnceLock<Option<reqwest::Client>> = OnceLock::new();
    MEDIA_CLIENT
        .get_or_init(|| youtube::media_client().ok())
        .as_ref()
}

/// Shared cache handle. `None` means the cache/queue could not be opened, in
/// which case the Telegram endpoints report `unavailable` instead of failing.
pub type SharedCache = Arc<Option<Arc<TelegramCacheManager>>>;

/// Returns the Android-provided `(filesDir, cacheDir)` pair, if the server booted.
pub fn paths() -> Option<&'static (String, String)> {
    PATHS.get()
}

/// Boots the Tokio runtime and the local Axum server on a background thread.
///
/// Safe to call more than once; only the first call starts the server.
pub fn boot(files_dir: String, cache_dir: String) {
    if SERVER_STARTED.set(()).is_err() {
        return;
    }

    std::thread::Builder::new()
        .name("ayshamusic-server".into())
        .spawn(move || {
            // Two workers only: an 8-core phone must not pay for idle threads.
            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    eprintln!("ayshamusic: tokio runtime init failed: {error}");
                    return;
                }
            };
            runtime.block_on(serve(files_dir, cache_dir));
        })
        .expect("failed to spawn ayshamusic server thread");
}

/// Builds the HTTP router with the given cache handle.
pub fn router(state: SharedCache) -> Router {
    router_with_phone(state, android_catalog::new_state())
}
fn router_with_phone(state: SharedCache, phone: android_catalog::Phone) -> Router {
    Router::new()
        .merge(android_catalog::routes())
        .route("/", get(index))
        .route("/api/status", get(status))
        .route("/api/tg/quota", get(tg_quota))
        .route("/api/tg/send_code", post(tg_send_code))
        .route("/api/tg/verify_code", post(tg_verify_code))
        .route("/api/tg/status", get(tg_status))
        .route("/api/tg/configure", post(tg_configure))
        .route("/api/tg/disconnect", post(tg_disconnect))
        .route("/api/play", post(play))
        .route("/api/pause", post(pause))
        .route("/api/resume", post(resume))
        .route("/api/stop", post(stop))
        .route("/api/volume", post(volume))
        .route("/api/player", get(player_status))
        .route("/api/selftest", get(selftest))
        .layer(Extension(phone))
        .layer(axum::extract::DefaultBodyLimit::max(16 * 1024))
        .with_state(state)
}

async fn serve(files_dir: String, cache_dir: String) {
    // Open the persistent Telegram queue and keep the disk cache bounded.
    let manager = match TelegramCacheManager::open(&files_dir, &cache_dir) {
        Ok(manager) => {
            match manager.evict_lru() {
                Ok(report) if report.deleted_files > 0 => println!(
                    "ayshamusic: LRU evicted {} file(s), {} byte(s) freed",
                    report.deleted_files, report.freed_bytes
                ),
                Ok(_) => {}
                Err(error) => eprintln!("ayshamusic: cache eviction failed: {error}"),
            }
            Some(Arc::new(manager))
        }
        Err(error) => {
            eprintln!("ayshamusic: telegram cache manager unavailable: {error}");
            None
        }
    };

    let _ = PATHS.set((files_dir, cache_dir));
    if let Some(cache)=&manager {
        let _=TG_CACHE.set(cache.clone());
        tokio::spawn(tg_storage::worker(cache.clone()));
    }
    let phone = android_catalog::new_state();
    let sampling = phone.clone();
    tokio::spawn(async move {
        let mut timer = tokio::time::interval(std::time::Duration::from_secs(5));
        loop { timer.tick().await; let phone=sampling.clone(); let _=tokio::task::spawn_blocking(move || sample_listening(&phone)).await; }
    });
    let app = router_with_phone(Arc::new(manager), phone);

    // Loopback only: the WebView talks to 127.0.0.1, the phone's LAN cannot.
    match tokio::net::TcpListener::bind("127.0.0.1:5050").await {
        Ok(listener) => {
            println!("ayshamusic: listening on http://127.0.0.1:5050");
            if let Err(error) = axum::serve(listener, app).await {
                eprintln!("ayshamusic: server stopped: {error}");
            }
        }
        Err(error) => eprintln!("ayshamusic: bind 127.0.0.1:5050 failed: {error}"),
    }
}

async fn index() -> axum::response::Html<String> {
    axum::response::Html(INDEX_HTML.replace("nativePlayback: false", "nativePlayback: true"))
}

async fn status() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "phase": 2,
        "server": "ayshamusic",
        "backend": "rust",
        // The page uses this to switch from <audio>/proxy streaming to the
        // in-app Rust player. The Linux server does not send it.
        "native_playback": true,
        "build_id": option_env!("GITHUB_SHA").unwrap_or("local"),
        "playback_baseline": "7b09cde-1.2.1",
    }))
}

fn unavailable() -> Json<Value> {
    Json(json!({
        "status": "unavailable",
        "error": "telegram cache manager not initialized",
    }))
}

/// `GET /api/tg/quota` - flat snapshot for the sidebar quota gauge.
async fn tg_quota(State(state): State<SharedCache>) -> Json<Value> {
    let Some(manager) = state.as_ref() else {
        return unavailable();
    };

    match manager.quota_status() {
        Ok(quota) => Json(json!({
            "remaining_in_window": quota.remaining_in_window,
            "max_per_window": quota.max_per_window,
            "queued_tracks": quota.queued_tracks,
            "cooldown_seconds_left": quota.cooldown_seconds_left,
            "status_label": quota.status_label,
        })),
        Err(error) => Json(json!({
            "status": "error",
            "error": error.to_string(),
        })),
    }
}

#[derive(Deserialize)]
struct SendCodeBody {
    phone: String,
}

/// `POST /api/tg/send_code` - `{ "phone": "+123..." }`.
async fn tg_send_code(
    State(state): State<SharedCache>,
    Json(body): Json<SendCodeBody>,
) -> Json<Value> {
    let Some(manager) = state.as_ref() else {
        return unavailable();
    };
    let outcome=manager.telegram().send_code(&body.phone).await;
    if outcome==TgOutcome::Authorized {tg_storage::pause(false);}
    Json(outcome.to_json())
}

#[derive(Deserialize)]
struct VerifyCodeBody {
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    password: Option<String>,
}

/// `POST /api/tg/verify_code` - `{ "code": "12345" }`, then
/// `{ "password": "..." }` when Telegram reports that 2FA is enabled.
async fn tg_verify_code(
    State(state): State<SharedCache>,
    Json(body): Json<VerifyCodeBody>,
) -> Json<Value> {
    let Some(manager) = state.as_ref() else {
        return unavailable();
    };
    let telegram = manager.telegram();

    let outcome = match (body.password.as_deref(), body.code.as_deref()) {
        (Some(password), _) => telegram.verify_password(password).await,
        (None, Some(code)) => telegram.verify_code(code).await,
        (None, None) => TgOutcome::Error("code_or_password_required".to_string()),
    };
    if outcome==TgOutcome::Authorized {tg_storage::pause(false);}
    Json(outcome.to_json())
}

#[derive(Deserialize)]
struct TgConfigureBody {api_id:i32,api_hash:String}
async fn tg_configure(State(state):State<SharedCache>,Json(body):Json<TgConfigureBody>)->(StatusCode,Json<Value>) {
    let Some(manager)=state.as_ref() else {return (StatusCode::SERVICE_UNAVAILABLE,unavailable());};
    match manager.telegram().configure(body.api_id,&body.api_hash).await {
        Ok(())=>(StatusCode::OK,Json(json!({"status":"ok"}))),
        Err(error)=>api_error(StatusCode::BAD_REQUEST,"telegram_setup",error),
    }
}
async fn tg_disconnect(State(state):State<SharedCache>)->(StatusCode,Json<Value>) {
    let Some(manager)=state.as_ref() else {return (StatusCode::SERVICE_UNAVAILABLE,unavailable());};
    tg_storage::pause(true);
    match manager.telegram().disconnect().await {
        Ok(())=>{let _=manager.clear_pending();let _=started_player().map(player::Player::stop);(StatusCode::OK,Json(json!({"status":"ok"})))},
        Err(error)=>api_error(StatusCode::BAD_GATEWAY,"telegram_disconnect",error),
    }
}

/// `GET /api/tg/status` - login state for the login modal / sidebar.
async fn tg_status(State(state): State<SharedCache>) -> Json<Value> {
    let Some(manager) = state.as_ref() else {
        return unavailable();
    };

    let status = manager.telegram().status().await;
    Json(json!({
        "archive_error":tg_storage::last_error(),
        "failed_tracks":manager.failed_tracks().unwrap_or(0),
        "quota":manager.quota_status().ok(),
        "destination":"Saved Messages",
        "authorized": status.authorized,
        "connected": status.connected,
        "needs_password": status.needs_password,
        "pending_code": status.pending_code,
        "configured": status.configured,
    }))
}

#[derive(Deserialize)]
struct PlayBody {
    video_id: String,
    /// Optional: `android`, `ios` or `tv`. Omit to fall back across all three.
    #[serde(default)]
    client: Option<String>,
    #[serde(default)]
    track: Option<android_catalog::Track>,
}

fn api_error(status: StatusCode, code: &str, message: String) -> (StatusCode, Json<Value>) {
    (status, Json(json!({ "status": "error", "error": code, "detail": message })))
}

/// `POST /api/play` - `{ "video_id": "dQw4w9WgXcQ", "client": "android" }`.
///
/// `client` is optional (`android` | `ios` | `tv`); omit it to fall back across
/// all three. Failures return `attempts: [{client, error}]` containing
/// YouTube's own body per client. Resolves the audio URL, then streams the
/// media through a bounded ring buffer into the rodio/cpal playback thread;
/// returns once playback has been handed over, not when the track ends.
async fn play(Extension(phone): Extension<android_catalog::Phone>, Json(body): Json<PlayBody>) -> (StatusCode, Json<Value>) {
    let Ok(_permit)=phone.operation_slot.try_acquire() else {return api_error(StatusCode::TOO_MANY_REQUESTS,"play_busy","Track change already running".into());};
    let old=phone.library.lock().unwrap().current.clone();
    sample_listening(&phone);
    let supplied=body.track.clone();
    let id=body.video_id.trim().to_owned();
    let result=play_audio(body).await;
    if result.0.is_success() {
        if let Some(track)=old.filter(|t|t.id!=id) {phone.listening.lock().unwrap().skip(&track);}
        let title=result.1.0["title"].as_str().map(str::to_owned);
        let track=supplied.filter(|t|t.id==id).and_then(|t|t.normalized().ok()).unwrap_or_else(||android_catalog::Track::from_id(&id,title));
        phone.library.lock().unwrap().remember(track);
        phone.listening.lock().unwrap().flush();
    }
    result
}
async fn play_audio(body: PlayBody) -> (StatusCode, Json<Value>) {
    let Some(client)=http_client() else {return api_error(StatusCode::SERVICE_UNAVAILABLE,"http_client","HTTP client unavailable".into());};
    let forced=match forced_client(body.client.as_deref()) {Ok(value)=>value,Err(error)=>return error};
    match start_track(client,body.video_id.trim(),forced).await {Ok(payload)=>(StatusCode::OK,Json(payload)),Err(error)=>error}
}
fn sample_listening(phone: &android_catalog::Phone) {
    let Some(player)=started_player() else {return;};
    let snapshot=player.snapshot();
    let track=phone.library.lock().unwrap().current.clone();
    if let Some(track)=track {phone.listening.lock().unwrap().observe(&track,&snapshot);}
}

/// Resolves `android` / `ios` / `tv` for a request, or the API error to return.
fn forced_client(name: Option<&str>) -> Result<Option<youtube::InnertubeClient>, (StatusCode, Json<Value>)> {
    match name.map(str::trim).filter(|name| !name.is_empty()) {
        None => Ok(None),
        Some(name) => match youtube::client_by_name(name) {
            Some(profile) => Ok(Some(profile)),
            None => Err(api_error(
                StatusCode::BAD_REQUEST,
                "unknown_client",
                format!(
                    "unknown client '{name}'; expected one of {}",
                    youtube::client_alias_list()
                ),
            )),
        },
    }
}

/// Track length from the audio format: bytes / (bits per second / 8).
fn duration_from(audio: &youtube::ResolvedAudio) -> f64 {
    match (audio.content_length, audio.bitrate) {
        (Some(bytes), bitrate) if bitrate > 0 => bytes as f64 * 8.0 / bitrate as f64,
        _ => 0.0,
    }
}

/// Resolve + start one video. Shared by `/api/play`, `/api/next` and `/api/prev`.
async fn start_track(
    client: &reqwest::Client,
    video_id: &str,
    forced: Option<youtube::InnertubeClient>,
) -> Result<Value, (StatusCode, Json<Value>)> {
    use youtube::ResolveError;

    if !youtube::valid_video_id(video_id) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "invalid_video_id",
            "video_id must be 11 characters of [A-Za-z0-9_-]".into(),
        ));
    }

    if forced.is_none() {
        if let Some(cache)=cache_manager() {
            if let Ok(Some(media))=tg_storage::lookup(&cache,video_id).await {
                if start_native(media).await.is_ok() {
                    return Ok(json!({"status":"playing","video_id":video_id,"source":"telegram","telegram_archive":"saved"}));
                }
                // User approved YouTube fallback if Telegram lookup/read/decode fails.
            }
        }
    }
    let visitor = visitor(client).await;
    let visitor_value = visitor.attached().then(|| visitor.value.as_str());
    let resolved = match forced {
        Some(profile) => youtube::resolve_forced(client, video_id, profile, visitor_value).await,
        None => youtube::resolve_audio(client, video_id, visitor_value).await,
    };

    let audio = match resolved {
        Ok(audio) => audio,
        Err(error @ ResolveError::InvalidVideoId) => {
            return Err(api_error(
                StatusCode::BAD_REQUEST,
                "invalid_video_id",
                error.to_string(),
            ))
        }
        Err(ResolveError::AllClientsFailed(attempts)) => {
            // Surface YouTube's own body for every client that was tried.
            let attempts_json: Vec<Value> =
                attempts.iter().map(youtube::ClientAttempt::to_json).collect();
            let detail = ResolveError::AllClientsFailed(attempts).to_string();
            return Err((
                StatusCode::BAD_GATEWAY,
                Json(json!({
                    "status": "error",
                    "error": "unresolved",
                    "detail": detail,
                    "attempts": attempts_json,
                })),
            ));
        }
        Err(error) => {
            return Err(api_error(
                StatusCode::BAD_GATEWAY,
                "unresolved",
                error.to_string(),
            ))
        }
    };

    let duration_secs = duration_from(&audio);
    start_native(player::Media {
        video_id:audio.video_id.clone(), title:audio.title.clone(), client:audio.client,
        itag:audio.itag,url:audio.url.clone(),user_agent:youtube::ANDROID.user_agent,
        content_length:audio.content_length,fetch:None,
    }).await?;
    let mut archive = "not_connected";
    if let Some(cache)=cache_manager() {
        let path=cache.cached_path(video_id);
        archive=if cache.queued_tracks().unwrap_or(20)>=20 { "queue_full" }
          else if cache.enqueue(video_id,&path,audio.title.as_deref().unwrap_or("YouTube track"),"").is_ok() { "queued" }
          else { "queue_error" };
    }

    Ok(json!({
        "status": "playing",
        "video_id": audio.video_id,
        "title": audio.title,
        "itag": audio.itag,
        "mime": audio.mime,
        "bitrate": audio.bitrate,
        "content_length": audio.content_length,
        "duration_secs": duration_secs,
        "client": audio.client,
        "forced": forced.is_some(),
        "source":"youtube",
        "telegram_archive":archive,
    }))
}

async fn start_native(media: player::Media) -> Result<(), (StatusCode, Json<Value>)> {
    let ready=player().play(media).map_err(|e|api_error(StatusCode::SERVICE_UNAVAILABLE,"playback_unavailable",e))?;
    match tokio::time::timeout(std::time::Duration::from_secs(45),ready).await {
        Ok(Ok(Ok(())))=>Ok(()),
        Ok(Ok(Err(error)))=>Err(api_error(StatusCode::BAD_GATEWAY,"audio_prepare_failed",error)),
        _=>{let _=player().stop();Err(api_error(StatusCode::GATEWAY_TIMEOUT,"audio_prepare_timeout","Audio preparation did not complete".into()))}
    }
}

/// Streams the media URL into the playback ring.
///
/// A 403 (or a transport error) **before any audio reaches the decoder** triggers
/// exactly one re-resolve; googlevideo URLs expire, and the player has not
/// consumed a byte yet, so retrying is safe. Once bytes have flowed, restarting
/// would corrupt the AAC stream, so that case is reported instead of faked.
async fn pump_media(
    client: reqwest::Client,
    video_id: String,
    forced: Option<&'static str>,
    visitor: Option<String>,
    initial_url: String,
    tx: std::sync::mpsc::SyncSender<std::io::Result<Vec<u8>>>,
) {
    use futures_util::StreamExt;

    let Some(media) = media_client() else {
        send_stream_error(&tx, "could not build media HTTP client".into());
        return;
    };
    let mut url = initial_url;
    let mut refreshed = false;
    let mut delivered: usize = 0;

    loop {
        // Media client, not the API client: the API client's 12 s total timeout
        // truncated playback roughly twelve seconds into every track.
        let response = match media
            .get(&url)
            .header(reqwest::header::USER_AGENT, youtube::ANDROID.user_agent)
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                send_stream_error(&tx, format!("media request failed: {error}"));
                return;
            }
        };

        let status = response.status();
        if status == StatusCode::FORBIDDEN && !refreshed {
            refreshed = true;
            match re_resolve(&client, &video_id, forced, visitor.as_deref()).await {
                Ok(fresh) => {
                    url = fresh.url;
                    continue;
                }
                Err(error) => {
                    send_stream_error(
                        &tx,
                        format!("media URL 403 and re-resolve failed: {error}"),
                    );
                    return;
                }
            }
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            send_stream_error(
                &tx,
                format!(
                    "media HTTP {status}: {}",
                    youtube::summarize_error_body(&body)
                ),
            );
            return;
        }

        let mut stream = response.bytes_stream();
        let mut retry = false;
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) if bytes.is_empty() => {}
                Ok(bytes) => {
                    delivered += bytes.len();
                    // Blocks when the ring is full: backpressure, not buffering.
                    if tx.send(Ok(bytes.to_vec())).is_err() {
                        return;
                    }
                }
                Err(error) => {
                    if delivered == 0 && !refreshed {
                        refreshed = true;
                        match re_resolve(&client, &video_id, forced, visitor.as_deref()).await {
                            Ok(fresh) => {
                                url = fresh.url;
                                retry = true;
                                break;
                            }
                            Err(refresh_error) => {
                                send_stream_error(
                                    &tx,
                                    format!(
                                        "media stream failed ({error}) and re-resolve failed: {refresh_error}"
                                    ),
                                );
                                return;
                            }
                        }
                    }
                    send_stream_error(
                        &tx,
                        format!("media stream failed after {delivered} bytes: {error}"),
                    );
                    return;
                }
            }
        }
        if retry {
            continue;
        }
        return;
    }
}

async fn re_resolve(
    client: &reqwest::Client,
    video_id: &str,
    forced: Option<&'static str>,
    visitor: Option<&str>,
) -> Result<youtube::ResolvedAudio, String> {
    let resolved = match forced.and_then(youtube::client_by_name) {
        Some(profile) => youtube::resolve_forced(client, video_id, profile, visitor).await,
        None => youtube::resolve_audio(client, video_id, visitor).await,
    };
    resolved.map_err(|error| error.to_string())
}

fn send_stream_error(
    tx: &std::sync::mpsc::SyncSender<std::io::Result<Vec<u8>>>,
    message: String,
) {
    let _ = tx.send(Err(std::io::Error::other(message)));
}

async fn pause() -> Json<Value> {
    match started_player().map(player::Player::pause) {
        Some(Ok(())) => Json(json!({ "status": "paused" })),
        Some(Err(error)) => Json(json!({ "status": "error", "detail": error })),
        None => Json(json!({ "status": "idle" })),
    }
}

async fn resume() -> Json<Value> {
    match started_player().map(player::Player::resume) {
        Some(Ok(())) => Json(json!({ "status": "playing" })),
        Some(Err(error)) => Json(json!({ "status": "error", "detail": error })),
        None => Json(json!({ "status": "idle" })),
    }
}

async fn stop(Extension(phone): Extension<android_catalog::Phone>) -> Json<Value> {
    sample_listening(&phone);phone.listening.lock().unwrap().flush();
    match started_player().map(player::Player::stop) {
        Some(Ok(())) => Json(json!({ "status": "stopped" })),
        Some(Err(error)) => Json(json!({ "status": "error", "detail": error })),
        None => Json(json!({ "status": "idle" })),
    }
}

#[derive(Deserialize)]
struct VolumeBody {
    volume: f32,
}

/// `POST /api/volume` - `{ "volume": 0.8 }` (0.0 - 1.0, clamped).
async fn volume(Json(body): Json<VolumeBody>) -> Json<Value> {
    if started_player().is_none() {
        // Nothing has opened the device yet; report the level without doing so.
        return Json(json!({ "status": "idle", "volume": body.volume.clamp(0.0, 1.0) }));
    }
    match player().set_volume(body.volume) {
        Ok(()) => Json(json!({ "status": "ok", "volume": body.volume.clamp(0.0, 1.0) })),
        Err(error) => Json(json!({ "status": "error", "detail": error })),
    }
}

#[derive(Deserialize)]
struct SelftestQuery {
    #[serde(default)]
    video_id: Option<String>,
    #[serde(default)]
    client: Option<String>,
}

/// `GET /api/selftest[?video_id=...&client=android]`
///
/// Proves the pipeline without opening an audio device: resolve the video, then
/// range-fetch the first 64 KiB of the media URL. Returns timing, the chosen
/// client/itag and, on failure, every client's own YouTube error body.
async fn selftest(Query(query): Query<SelftestQuery>) -> (StatusCode, Json<Value>) {
    use std::time::Instant;
    use youtube::ResolveError;

    let video_id = query.video_id.as_deref().unwrap_or("dQw4w9WgXcQ").trim();
    let forced = match query
        .client
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        None => None,
        Some(name) => match youtube::client_by_name(name) {
            Some(profile) => Some(profile),
            None => {
                return api_error(
                    StatusCode::BAD_REQUEST,
                    "unknown_client",
                    format!(
                        "unknown client '{name}'; expected one of {}",
                        youtube::client_alias_list()
                    ),
                )
            }
        },
    };

    let Some(client) = http_client() else {
        return api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "http_client",
            "could not build HTTP client".into(),
        );
    };

    if !youtube::valid_video_id(video_id) {
        return api_error(
            StatusCode::BAD_REQUEST,
            "invalid_video_id",
            "video_id must be 11 characters of [A-Za-z0-9_-]".into(),
        );
    }

    let started = Instant::now();
    let visitor = visitor(client).await;
    let visitor_value = visitor.attached().then(|| visitor.value.as_str());
    let resolved = match forced {
        Some(profile) => youtube::resolve_forced(client, video_id, profile, visitor_value).await,
        None => youtube::resolve_audio(client, video_id, visitor_value).await,
    };
    let audio = match resolved {
        Ok(audio) => audio,
        Err(error @ ResolveError::InvalidVideoId) => {
            return api_error(StatusCode::BAD_REQUEST, "invalid_video_id", error.to_string())
        }
        Err(ResolveError::AllClientsFailed(attempts)) => {
            let attempts_json: Vec<Value> =
                attempts.iter().map(youtube::ClientAttempt::to_json).collect();
            let detail = ResolveError::AllClientsFailed(attempts).to_string();
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({
                    "ok": false,
                    "stage": "resolve",
                    "error": "unresolved",
                    "detail": detail,
                    "attempts": attempts_json,
                })),
            );
        }
        Err(error) => {
            return api_error(StatusCode::BAD_GATEWAY, "unresolved", error.to_string())
        }
    };
    let resolve_ms = started.elapsed().as_millis() as u64;

    let Some(media) = media_client() else {
        return api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "http_client",
            "could not build media HTTP client".into(),
        );
    };
    let response = match media
        .get(&audio.url)
        .header(reqwest::header::USER_AGENT, youtube::ANDROID.user_agent)
        .header(reqwest::header::RANGE, "bytes=0-65535")
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({
                    "ok": false,
                    "stage": "media",
                    "error": error.to_string(),
                    "video_id": audio.video_id,
                    "client": audio.client,
                    "itag": audio.itag,
                    "resolve_ms": resolve_ms,
                })),
            )
        }
    };

    let status = response.status();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let media_host = reqwest::Url::parse(&audio.url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string));

    use futures_util::StreamExt;
    let mut fetched: usize = 0;
    let mut stream = response.bytes_stream();
    let mut stream_error: Option<String> = None;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(bytes) => {
                fetched += bytes.len();
                if fetched >= 64 * 1024 {
                    break;
                }
            }
            Err(error) => {
                stream_error = Some(error.to_string());
                break;
            }
        }
    }

    let ok = status.is_success() && fetched > 0 && stream_error.is_none();
    let payload = json!({
        "ok": ok,
        "video_id": audio.video_id,
        "title": audio.title,
        "client": audio.client,
        "itag": audio.itag,
        "mime": audio.mime,
        "bitrate": audio.bitrate,
        "content_length": audio.content_length,
        "media_host": media_host,
        "media_status": status.as_u16(),
        "media_content_type": content_type,
        "bytes_fetched": fetched,
        "stream_error": stream_error,
        "visitor": {
            "source": visitor.source.as_str(),
            "attached": visitor.attached(),
        },
        "resolve_ms": resolve_ms,
        "elapsed_ms": started.elapsed().as_millis() as u64,
    });
    (
        if ok {
            StatusCode::OK
        } else {
            StatusCode::BAD_GATEWAY
        },
        Json(payload),
    )
}

/// `GET /api/player` - playback state for the now-playing UI.
///
/// Does not start the audio thread: polling the UI must not open the device.
async fn player_status(Extension(phone): Extension<android_catalog::Phone>) -> Json<Value> {
    let mut result = match started_player() {
        Some(player) => Json(player.snapshot()),
        None => Json(json!({
            "state": "idle",
            "video_id": null,
            "title": null,
            "volume": 1.0,
            "position_secs": 0.0,
            "error": null,
        })),
    };
    result.0["track"]=json!(phone.library.lock().unwrap().current);
    result
}

#[cfg(target_os = "android")]
mod android {
    use super::boot;
    use jni::{
        objects::{GlobalRef, JObject, JString},
        JNIEnv, JavaVM,
    };
    use std::{ffi::c_void, sync::Mutex};

    // Pin the APPLICATION context, not a JNI local ref or a short-lived Activity.
    // Its lifetime is the process lifetime, just like the native server/player.
    // This mutex serializes initialization (ndk-context must be set exactly once).
    static AUDIO_CONTEXT: Mutex<Option<GlobalRef>> = Mutex::new(None);

    pub(super) fn audio_context_ready() -> bool {
        AUDIO_CONTEXT.lock().map(|context| context.is_some()).unwrap_or(false)
    }

    fn initialize_audio_context(env: &mut JNIEnv<'_>, context: &JObject<'_>) -> Result<(), String> {
        let mut stored = AUDIO_CONTEXT.lock().map_err(|_| "Android audio context lock unavailable")?;
        if stored.is_some() {
            return Ok(());
        }
        if context.is_null() {
            return Err("Android application context is missing".into());
        }
        let vm = env.get_java_vm().map_err(|_| "Could not get Android JavaVM")?;
        let global = env.new_global_ref(context).map_err(|_| "Could not retain Android application context")?;
        // SAFETY: called once under AUDIO_CONTEXT before boot starts workers;
        // JVM lives for this process, and the GlobalRef is retained for that same
        // lifetime. Never release while a CPAL thread could still use the pointers.
        unsafe {
            ndk_context::initialize_android_context(
                vm.get_java_vm_pointer().cast(),
                global.as_obj().as_raw().cast(),
            );
        }
        *stored = Some(global);
        Ok(())
    }

    /// JNI version negotiation only. Context is supplied explicitly at startup.
    #[no_mangle]
    pub extern "system" fn JNI_OnLoad(_vm: JavaVM, _reserved: *mut c_void) -> jni::sys::jint {
        jni::sys::JNI_VERSION_1_6
    }

    /// Kotlin: `RustBridge.startServer(filesDir, cacheDir, applicationContext)`.
    ///
    /// `RustBridge` is a Kotlin `object`, so the receiver is a `jobject`
    /// (not a `jclass`).
    #[no_mangle]
    pub extern "system" fn Java_com_afzal_ayshamusic_RustBridge_startServer<'local>(
        mut env: JNIEnv<'local>,
        _receiver: JObject<'local>,
        files_dir: JString<'local>,
        cache_dir: JString<'local>,
        context: JObject<'local>,
    ) {
        let files_dir: String = match env.get_string(&files_dir) {
            Ok(value) => value.into(),
            Err(_) => return,
        };
        let cache_dir: String = match env.get_string(&cache_dir) {
            Ok(value) => value.into(),
            Err(_) => return,
        };

        if let Err(error) = initialize_audio_context(&mut env, &context) {
            let _ = env.throw_new("java/lang/IllegalStateException", error);
            return;
        }
        boot(files_dir, cache_dir);
    }
}

#[cfg(test)]
mod api_tests {
    use super::*;
    use axum::body::Body;
    use http_body_util::BodyExt;
    use std::path::PathBuf;
    use tower::ServiceExt;

    struct TempDirs {
        root: PathBuf,
    }

    impl TempDirs {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "aysha-api-test-{}-{}-{}",
                std::process::id(),
                name,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|elapsed| elapsed.as_nanos())
                    .unwrap_or(0)
            ));
            std::fs::create_dir_all(root.join("files")).unwrap();
            std::fs::create_dir_all(root.join("cache")).unwrap();
            Self { root }
        }

        fn manager(&self) -> TelegramCacheManager {
            TelegramCacheManager::open(self.root.join("files"), self.root.join("cache")).unwrap()
        }
    }

    impl Drop for TempDirs {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    async fn json_body(response: axum::response::Response) -> Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    fn post(uri: &str, body: &str) -> axum::http::Request<Body> {
        axum::http::Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    #[tokio::test]
    async fn telegram_endpoints_are_wired_and_report_missing_credentials() {
        let dirs = TempDirs::new("endpoints");
        let app = router(Arc::new(Some(Arc::new(dirs.manager()))));

        // send_code without TG_API_ID/TG_API_HASH must not touch the network.
        let response = app
            .clone()
            .oneshot(post("/api/tg/send_code", r#"{"phone":"+10000000000"}"#))
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(json_body(response).await["status"], "missing_credentials");

        // verify_code before send_code reports not_started.
        let response = app
            .clone()
            .oneshot(post("/api/tg/verify_code", r#"{"code":"12345"}"#))
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(json_body(response).await["status"], "not_started");

        // Status reflects an unconfigured, unauthorized account.
        let response = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/tg/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = json_body(response).await;
        assert_eq!(body["authorized"], false);
        assert_eq!(body["configured"], false);

        // Quota is always available when the manager opened.
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/tg/quota")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = json_body(response).await;
        assert_eq!(body["max_per_window"], 18);
        assert_eq!(body["queued_tracks"], 0);
        assert_eq!(body["cooldown_seconds_left"], 0);
    }

    #[tokio::test]
    async fn endpoints_report_unavailable_without_a_cache_manager() {
        let app = router(Arc::new(None));

        let response = app
            .clone()
            .oneshot(post("/api/tg/send_code", r#"{"phone":"+1"}"#))
            .await
            .unwrap();
        assert_eq!(json_body(response).await["status"], "unavailable");

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/tg/quota")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(json_body(response).await["status"], "unavailable");
    }

    #[tokio::test]
    async fn play_rejects_malformed_video_ids_without_touching_youtube() {
        let app = router(Arc::new(None));
        let response = app
            .oneshot(post("/api/play", r#"{"video_id":"../../etc/passwd"}"#))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = json_body(response).await;
        assert_eq!(body["error"], "invalid_video_id");
    }

    #[tokio::test]
    async fn phone_learning_reset_preserves_queue_and_likes_without_opening_audio() {
        let app=router(Arc::new(None));
        let track=r#"{"id":"abcdefghijk","title":"Romantic song","artist":"A"}"#;
        for route in ["/api/queue_add","/api/like"] {
            assert_eq!(app.clone().oneshot(post(route,track)).await.unwrap().status(),StatusCode::OK);
        }
        let get=|uri|axum::http::Request::builder().uri(uri).body(Body::empty()).unwrap();
        let profile=json_body(app.clone().oneshot(get("/api/recommendation/profile")).await.unwrap()).await;
        assert_eq!(profile["learned_tracks"],1);assert_eq!(profile["local_only"],true);
        assert_eq!(app.clone().oneshot(post("/api/recommendation/reset","{}")).await.unwrap().status(),StatusCode::OK);
        let profile=json_body(app.clone().oneshot(get("/api/recommendation/profile")).await.unwrap()).await;
        assert_eq!(profile["learned_tracks"],0);
        for uri in ["/api/liked","/api/queue"] {
            let items=json_body(app.clone().oneshot(get(uri)).await.unwrap()).await;
            assert_eq!(items.as_array().unwrap().len(),1);
        }
        let page=app.oneshot(get("/")).await.unwrap();
        let body=page.into_body().collect().await.unwrap().to_bytes();
        let html=std::str::from_utf8(&body).unwrap();
        assert!(html.contains("nativePlayback: true"));assert!(!html.contains("id=\"next-mood\""));
    }

    /// Live diagnostic (not in the default suite): shows exactly what each
    /// client said. `cargo test --lib live_play -- --ignored --nocapture`.
    #[tokio::test]
    #[ignore = "hits youtubei"]
    async fn live_play_reports_youtube_body_per_client() {
        let app = router(Arc::new(None));
        let response = app
            .oneshot(post("/api/play", r#"{"video_id":"dQw4w9WgXcQ"}"#))
            .await
            .unwrap();
        let status = response.status();
        let body = json_body(response).await;
        eprintln!(
            "status={status} body={}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );
        if status == StatusCode::OK {
            return;
        }
        assert_eq!(body["error"], "unresolved");
        let attempts = body["attempts"].as_array().expect("attempts array");
        assert_eq!(attempts.len(), 3, "one attempt per client");
    }

    #[tokio::test]
    async fn selftest_rejects_malformed_ids_without_network() {
        let app = router(Arc::new(None));
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/selftest?video_id=short")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(json_body(response).await["error"], "invalid_video_id");
    }

    #[tokio::test]
    async fn play_rejects_an_unknown_forced_client() {
        let app = router(Arc::new(None));
        let response = app
            .oneshot(post(
                "/api/play",
                r#"{"video_id":"dQw4w9WgXcQ","client":"web"}"#,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = json_body(response).await;
        assert_eq!(body["error"], "unknown_client");
        assert!(body["detail"].as_str().unwrap_or_default().contains("android"));
    }

    #[tokio::test]
    async fn player_endpoint_reports_initial_state() {
        // On a host without an audio device this is `error`; on Android it is
        // `idle` until the first track is handed over. Either is a valid boot.
        let app = router(Arc::new(None));
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/player")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let state = json_body(response).await["state"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        assert!(state == "idle" || state == "error", "unexpected state {state}");
    }

    #[tokio::test]
    async fn search_requires_a_query() {
        let app = router(Arc::new(None));
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/search?q=")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }


    /// Proves the bytes that reach the phone's decoder are real audio.
    ///
    /// Uses the same resolve, the same pump and the same decoder as `/api/play`
    /// on the APK, then writes the bytes out so an external tool (ffprobe) can
    /// inspect the very same file. Network test:
    /// `cargo test --lib live_audio_pipeline -- --ignored --nocapture`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "hits youtubei and googlevideo"]
    async fn live_audio_pipeline_delivers_decodable_audio() {
        use rodio::Source;
        use std::time::{Duration, Instant};

        let Some(client) = http_client() else {
            panic!("http client");
        };
        let visitor = visitor(client).await;
        assert!(visitor.attached(), "no visitor id");

        let audio = youtube::resolve_audio(&client, "dQw4w9WgXcQ", Some(visitor.value.as_str()))
            .await
            .unwrap_or_else(|error| panic!("resolve: {error}"));
        eprintln!(
            "resolved client={} itag={} mime={} declared_bytes={:?}",
            audio.client, audio.itag, audio.mime, audio.content_length
        );

        // 1. Range request with the exact headers the pump uses.
        let range = client
            .get(&audio.url)
            .header(reqwest::header::USER_AGENT, youtube::ANDROID.user_agent)
            .header(reqwest::header::RANGE, "bytes=0-2047")
            .send()
            .await
            .expect("range request");
        let range_status = range.status();
        let range_type = range
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string();
        let content_range = range
            .headers()
            .get(reqwest::header::CONTENT_RANGE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string();
        let range_len = range.bytes().await.expect("range body").len();
        eprintln!(
            "range status={range_status} {range_len} bytes type={range_type:?} content-range={content_range:?}"
        );
        assert_eq!(range_status, StatusCode::PARTIAL_CONTENT);
        assert!(range_type.starts_with("audio/"), "content-type {range_type}");
        assert!(content_range.starts_with("bytes 0-2047/"), "{content_range}");

        // 2. The pump `/api/play` spawns, read until it has a decodable prefix.
        let (tx, rx) = player::stream_channel();
        let pump = tokio::spawn(pump_media(
            client.clone(),
            audio.video_id.clone(),
            None,
            visitor.attached().then(|| visitor.value.clone()),
            audio.url.clone(),
            tx,
        ));
        // Read to EOF: the whole track must arrive. Before the media client was
        // split out, the API client's 12 s total timeout killed this loop.
        let mut bytes: Vec<u8> = Vec::new();
        let started = Instant::now();
        loop {
            match rx.recv_timeout(Duration::from_secs(30)) {
                Ok(Ok(chunk)) => bytes.extend_from_slice(&chunk),
                Ok(Err(error)) => panic!("pump failed after {} bytes: {error}", bytes.len()),
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                Err(error) => panic!("pump stalled after {} bytes: {error}", bytes.len()),
            }
        }
        pump.abort();
        eprintln!(
            "streamed {} bytes in {:?} (declared {:?})",
            bytes.len(),
            started.elapsed(),
            audio.content_length
        );
        assert_eq!(
            Some(bytes.len() as u64),
            audio.content_length,
            "stream ended before the declared length"
        );
        assert!(bytes.len() >= 128 * 1024, "only {} bytes pumped", bytes.len());
        assert_eq!(
            bytes.get(4..8),
            Some(b"ftyp".as_slice()),
            "not an MP4 container"
        );

        let path = std::env::temp_dir().join("aysha_full_track.m4a");
        std::fs::write(&path, &bytes).expect("write probe");
        eprintln!("wrote {}", path.display());

        // 3. Decode with the exact decoder the player threads use.
        let decoder = rodio::Decoder::new(std::io::Cursor::new(bytes)).expect("decoder");
        let sample_rate = decoder.sample_rate();
        let channels = decoder.channels();
        let decoded: Vec<f32> = decoder.take(48_000).collect();
        let non_silent = decoded.iter().any(|sample| sample.abs() > 0.001);
        eprintln!(
            "decoded {} samples, rate={sample_rate}, channels={channels}, non-silent={non_silent}",
            decoded.len()
        );
        assert!(sample_rate.get() >= 8_000, "sample rate {sample_rate}");
        assert!(decoded.len() > 4_000, "decoded {} samples", decoded.len());
        assert!(non_silent, "decoded audio was silence");
    }

}
