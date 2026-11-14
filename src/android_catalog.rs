//! Phone-local catalogue and website API contracts. No Linux service or subprocesses.
use axum::{extract::{Extension, Query}, http::{header, StatusCode}, routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub title: String,
    #[serde(default)] pub artist: String,
    #[serde(default)] pub thumbnail: String,
    #[serde(default)] pub url: String,
}
impl Track {
    pub(super) fn normalized(mut self) -> Result<Self, &'static str> {
        if !crate::youtube::valid_video_id(&self.id) { return Err("invalid_video_id"); }
        self.title = self.title.chars().take(300).collect();
        self.artist = self.artist.chars().take(150).collect();
        self.url = format!("https://www.youtube.com/watch?v={}", self.id);
        self.thumbnail = format!("https://img.youtube.com/vi/{}/hqdefault.jpg", self.id);
        Ok(self)
    }
    pub fn from_id(id: &str, title: Option<String>) -> Self {
        Self { id: id.into(), title: title.unwrap_or_else(|| "YouTube track".into()),
            artist: String::new(), thumbnail: String::new(), url: String::new() }.normalized().expect("validated ID")
    }
}
#[derive(Default)]
pub struct Library {
    pub current: Option<Track>,
    queue: Vec<Track>,
    history: Vec<Track>,
    liked: Vec<Track>,
}
impl Library {
    pub fn remember(&mut self, track: Track) {
        if let Some(old) = &self.current {
            if old.id != track.id { self.history.push(old.clone()); }
        }
        if self.history.len() > 200 { self.history.remove(0); }
        self.current = Some(track);
    }
}
pub struct PhoneState {
    pub library: Mutex<Library>,
    search_slot: Semaphore,
    pub operation_slot: Semaphore,
}
pub type Phone = Arc<PhoneState>;
pub fn new_state() -> Phone {
    Arc::new(PhoneState { library: Mutex::new(Library::default()), search_slot: Semaphore::new(1), operation_slot: Semaphore::new(1) })
}
pub fn routes() -> Router<crate::SharedCache> {
    Router::new()
        .route("/icons.svg", get(icons))
        .route("/api/search", get(search))
        .route("/api/queue", get(queue))
        .route("/api/queue_add", post(queue_add))
        .route("/api/queue_clear", post(queue_clear))
        .route("/api/liked", get(liked))
        .route("/api/like", post(like))
        .route("/api/unlike", post(unlike))
        .route("/api/next", post(next))
        .route("/api/prev", post(previous))
        .route("/api/bt/status", get(bluetooth))
}
async fn icons() -> impl axum::response::IntoResponse {
    ([(header::CONTENT_TYPE, "image/svg+xml; charset=utf-8"), (header::CACHE_CONTROL, "no-cache")], include_str!("icons.svg"))
}
async fn bluetooth() -> Json<Value> {
    // Android manages the actual speaker/headset output; do not invoke Linux tools.
    Json(json!({"connected": false, "managed_by_android": true, "name": "Use Android sound settings"}))
}
fn error(status: StatusCode, code: &str) -> (StatusCode, Json<Value>) {
    (status, Json(json!({"status":"failed", "error":code})))
}
#[derive(Deserialize)]
struct Search { q: String }
async fn search(Extension(state): Extension<Phone>, Query(query): Query<Search>) -> (StatusCode, Json<Value>) {
    let q = query.q.trim();
    if q.is_empty() || q.chars().count() > 200 { return error(StatusCode::BAD_REQUEST, "Enter a search of 1–200 characters"); }
    let Ok(_permit) = state.search_slot.try_acquire() else { return error(StatusCode::TOO_MANY_REQUESTS, "Search already running; retry shortly"); };
    let Some(http) = crate::http_client() else { return error(StatusCode::SERVICE_UNAVAILABLE, "Search unavailable"); };
    match fetch_search(http, q).await {
        Ok(tracks) => (StatusCode::OK, Json(json!(tracks))),
        Err(code) => error(StatusCode::BAD_GATEWAY, code),
    }
}
async fn fetch_search(http: &reqwest::Client, q: &str) -> Result<Vec<Track>, &'static str> {
    use futures_util::StreamExt;
    let response = http.post("https://www.youtube.com/youtubei/v1/search?prettyPrint=false")
        .timeout(std::time::Duration::from_secs(12))
        .header(reqwest::header::USER_AGENT, "Mozilla/5.0")
        .json(&json!({"context":{"client":{"clientName":"WEB","clientVersion":"2.20241120.01.00","hl":"en"}},"query":q,"params":"EgIQAQ%3D%3D"}))
        .send().await.map_err(|_| "Music provider search connection failed; tap the tab to retry")?;
    if !response.status().is_success() { return Err("Music provider refused search; retry later"); }
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| "Music provider search interrupted")?;
        if bytes.len() + chunk.len() > 2 * 1024 * 1024 { return Err("Search response exceeded its size limit"); }
        bytes.extend_from_slice(&chunk);
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| "Music provider returned invalid search data")?;
    parse_search(&value)
}
fn label(value: &Value) -> String {
    if let Some(text) = value["simpleText"].as_str() { return text.chars().take(300).collect(); }
    value["runs"].as_array().map(|runs| runs.iter().take(20).filter_map(|r| r["text"].as_str()).collect::<String>().chars().take(300).collect()).unwrap_or_default()
}
fn parse_search(value: &Value) -> Result<Vec<Track>, &'static str> {
    if !value.get("contents").is_some_and(Value::is_object) { return Err("Music provider search format unavailable"); }
    let mut out = Vec::new();
    let mut pending = vec![(value, 0)];
    let mut visited = 0;
    while let Some((value, depth)) = pending.pop() {
        visited += 1;
        if visited > 20000 { return Err("Search response exceeded its complexity limit"); }
        if depth > 40 { continue; }
        if let Some(video) = value.get("videoRenderer") {
            if let Some(id) = video["videoId"].as_str().filter(|id| crate::youtube::valid_video_id(id)) {
                if !out.iter().any(|t: &Track| t.id == id) {
                    out.push(Track { id: id.into(), title: label(&video["title"]), artist: label(&video["ownerText"]), thumbnail: String::new(), url: String::new() }.normalized()?);
                    if out.len() == 20 { break; }
                }
            }
        }
        match value {
            Value::Object(map) => pending.extend(map.values().map(|v| (v, depth + 1))),
            Value::Array(items) => pending.extend(items.iter().rev().map(|v| (v, depth + 1))),
            _ => (),
        }
    }
    Ok(out)
}
async fn queue(Extension(state): Extension<Phone>) -> Json<Value> { Json(json!(state.library.lock().unwrap().queue)) }
async fn liked(Extension(state): Extension<Phone>) -> Json<Value> { Json(json!(state.library.lock().unwrap().liked)) }
async fn queue_add(Extension(state): Extension<Phone>, Json(track): Json<Track>) -> (StatusCode, Json<Value>) {
    let Ok(track) = track.normalized() else { return error(StatusCode::BAD_REQUEST, "invalid_video_id"); };
    let mut library = state.library.lock().unwrap();
    if !library.queue.iter().any(|t| t.id == track.id) {
        if library.queue.len() >= 200 { return error(StatusCode::CONFLICT, "Queue is full"); }
        library.queue.push(track);
    }
    (StatusCode::OK, Json(json!({"status":"ok"})))
}
async fn queue_clear(Extension(state): Extension<Phone>) -> Json<Value> {
    state.library.lock().unwrap().queue.clear(); Json(json!({"status":"ok"}))
}
async fn like(Extension(state): Extension<Phone>, Json(track): Json<Track>) -> (StatusCode, Json<Value>) {
    let Ok(track) = track.normalized() else { return error(StatusCode::BAD_REQUEST, "invalid_video_id"); };
    let mut library = state.library.lock().unwrap();
    if !library.liked.iter().any(|t| t.id == track.id) {
        if library.liked.len() >= 200 { return error(StatusCode::CONFLICT, "Favorites are full"); }
        library.liked.push(track);
    }
    (StatusCode::OK, Json(json!({"status":"ok"})))
}
async fn unlike(Extension(state): Extension<Phone>, Json(track): Json<Track>) -> Json<Value> {
    state.library.lock().unwrap().liked.retain(|t| t.id != track.id); Json(json!({"status":"ok"}))
}
async fn next(Extension(state): Extension<Phone>) -> (StatusCode, Json<Value>) { navigate(state, false).await }
async fn previous(Extension(state): Extension<Phone>) -> (StatusCode, Json<Value>) { navigate(state, true).await }
async fn navigate(state: Phone, previous: bool) -> (StatusCode, Json<Value>) {
    let Ok(_permit) = state.operation_slot.try_acquire() else { return error(StatusCode::TOO_MANY_REQUESTS, "Track change already running"); };
    let track = {
        let library = state.library.lock().unwrap();
        if previous { library.history.last().cloned() } else { library.queue.first().cloned() }
    };
    let Some(track) = track else { return error(StatusCode::CONFLICT, if previous { "No previous track" } else { "Queue is empty — add a song first" }); };
    let result = crate::play_audio(crate::PlayBody { video_id: track.id.clone(), client: None, track: Some(track.clone()) }).await;
    if result.0.is_success() {
        let mut library = state.library.lock().unwrap();
        if previous {
            library.history.pop();
            if let Some(current) = library.current.take() { library.queue.insert(0, current); library.queue.truncate(200); }
            library.current = Some(track.clone());
        } else {
            if library.queue.first().is_some_and(|t| t.id == track.id) { library.queue.remove(0); }
            library.remember(track.clone());
        }
        return (StatusCode::OK, Json(json!({"status":"playing","track":track,"mode":"native","already_started":true})));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn search_fixture_filters_invalid_ids_deduplicates_and_preserves_order() {
        let v = json!({"contents":{"items":[
            {"videoRenderer":{"videoId":"abcdefghijk","title":{"runs":[{"text":"First"}]},"ownerText":{"simpleText":"Artist"}}},
            {"videoRenderer":{"videoId":"invalid"}},
            {"videoRenderer":{"videoId":"abcdefghijk"}},
            {"videoRenderer":{"videoId":"12345678901","title":{"simpleText":"Second"}}}
        ]}});
        let tracks = parse_search(&v).unwrap();
        assert_eq!(tracks.len(), 2); assert_eq!(tracks[0].title, "First");
        assert_eq!(tracks[0].artist, "Artist"); assert_eq!(tracks[1].title, "Second");
        assert!(tracks[0].url.starts_with("https://www.youtube.com/watch?v="));
        assert!(parse_search(&json!({"error":"private"})).is_err());
        assert!(parse_search(&json!({"contents":{}})).unwrap().is_empty());
    }
    #[test]
    fn metadata_normalization_does_not_keep_arbitrary_urls() {
        let t = Track { id:"abcdefghijk".into(), title:"A".repeat(400), artist:String::new(), thumbnail:"http://private".into(), url:"http://private".into() }.normalized().unwrap();
        assert_eq!(t.title.len(),300); assert!(!t.url.contains("private"));
        let mut lib = Library::default(); lib.remember(t.clone()); lib.remember(t);
        assert!(lib.history.is_empty());
    }
}
