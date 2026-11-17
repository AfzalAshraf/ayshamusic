//! Phone-local catalogue and website API contracts. No Linux service or subprocesses.
use axum::{extract::{Extension, Query}, http::{header, StatusCode}, routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tokio::sync::{Semaphore,Mutex as AsyncMutex};
use std::time::{Duration,Instant};

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
    search_cache: AsyncMutex<Vec<(String,Instant,Vec<Track>)>>,
    pub mix: Mutex<crate::mix::Mix>,
    mix_slot: AsyncMutex<()>,
    pub autoplay_error: Mutex<Option<String>>,
    pub operation_slot: Semaphore,
    pub listening: Mutex<crate::listening::Listening>,
}
pub type Phone = Arc<PhoneState>;
pub fn new_state() -> Phone {
    Arc::new(PhoneState { library: Mutex::new(Library::default()), search_cache: AsyncMutex::new(Vec::new()), mix: Mutex::new(crate::mix::Mix::default()), mix_slot: AsyncMutex::new(()), autoplay_error: Mutex::new(None), operation_slot: Semaphore::new(1), listening: Mutex::new(crate::listening::Listening::open(crate::paths().map(|(files,_)| std::path::Path::new(files).join("listening-preferences.json")))) })
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
        .route("/api/recommendation/profile", get(profile_status))
        .route("/api/recommendation/reset", post(reset_learning))
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
    // Single-flight: wait for active metadata rather than rejecting normal tab
    // changes. Cache is rechecked after waiting; duplicate queries fetch once.
    let Ok(mut cache)=tokio::time::timeout(Duration::from_secs(30),state.search_cache.lock()).await else {
        return error(StatusCode::SERVICE_UNAVAILABLE,"Search is busy. Please try again shortly");
    };
    cache.retain(|(_,time,_)|time.elapsed()<Duration::from_secs(300));
    let key=q.to_lowercase();
    if let Some((_,_,tracks))=cache.iter().find(|(query,_,_)|query==&key) {
        return (StatusCode::OK,Json(json!(tracks)));
    }
    let Some(http) = crate::http_client() else { return error(StatusCode::SERVICE_UNAVAILABLE, "Search unavailable"); };
    match fetch_search(http, q).await {
        Ok(tracks) => {
            if cache.len()>=12 {cache.remove(0);}
            cache.push((key,Instant::now(),tracks.clone()));
            (StatusCode::OK, Json(json!(tracks)))
        },
        Err(code) => error(StatusCode::BAD_GATEWAY, code),
    }
}
async fn fetch_search(http: &reqwest::Client, q: &str) -> Result<Vec<Track>, &'static str> {
    let visitor = crate::visitor(http).await;
    let value = visitor.attached().then_some(visitor.value.as_str());
    crate::youtube::search(http, q, value, 20).await.map(|hits| hits.into_iter().filter_map(|hit| Track {
        id:hit.video_id,title:hit.title,artist:hit.channel,thumbnail:String::new(),url:String::new()
    }.normalized().ok()).collect()).map_err(|_| "YouTube search unavailable; try again later")
}

#[cfg(test)]
fn label(value: &Value) -> String {
    if let Some(text) = value["simpleText"].as_str() { return text.chars().take(300).collect(); }
    value["runs"].as_array().map(|runs| runs.iter().take(20).filter_map(|r| r["text"].as_str()).collect::<String>().chars().take(300).collect()).unwrap_or_default()
}
#[cfg(test)]
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
        library.liked.push(track.clone());
    }
    state.listening.lock().unwrap().like(&track,true);
    (StatusCode::OK, Json(json!({"status":"ok"})))
}
async fn unlike(Extension(state): Extension<Phone>, Json(track): Json<Track>) -> Json<Value> {
    state.library.lock().unwrap().liked.retain(|t| t.id != track.id);
    state.listening.lock().unwrap().like(&track,false); Json(json!({"status":"ok"}))
}
async fn profile_status(Extension(state): Extension<Phone>) -> Json<Value> {
    let model=state.listening.lock().unwrap();
    Json(json!({"local_only":true,"learned_tracks":model.preferences().observations,"saved_on_device":model.durable}))
}
async fn reset_learning(Extension(state): Extension<Phone>) -> (StatusCode, Json<Value>) {
    let mut model=state.listening.lock().unwrap();
    if !model.reset() {return error(StatusCode::SERVICE_UNAVAILABLE,"Could not clear saved preferences");}
    (StatusCode::OK,Json(json!({"status":"ok","saved_on_device":model.durable})))
}
/// Metadata-only warmup, generation-checked so an old seed cannot overwrite a
/// song the user just chose. Never prefetch audio or mark it as played/saved.
pub fn start_mix(state:Phone,seed:&str) {
    state.mix.lock().unwrap().reset(seed);
    *state.autoplay_error.lock().unwrap()=None;
    tokio::spawn(async move {let _=ensure_mix(&state).await;});
}
async fn ensure_mix(state:&Phone)->Result<(), &'static str> {
    let _permit=state.mix_slot.lock().await;
    let (generation,seed)={
        let mix=state.mix.lock().unwrap();
        if mix.fresh(){return Ok(());}
        (mix.generation,mix.seed.clone())
    };
    if !crate::youtube::valid_video_id(&seed){return Err("Play a song to start your mix");}
    let Some(http)=crate::http_client() else{return Err("Mix unavailable");};
    let visitor=crate::visitor(http).await;
    let result=crate::youtube::mix(http,&seed,visitor.attached().then_some(visitor.value.as_str())).await;
    let failed=result.is_err();
    let tracks=result.unwrap_or_default().into_iter().filter_map(|hit|Track{
        id:hit.video_id,title:hit.title,artist:hit.channel,thumbnail:String::new(),url:String::new()
    }.normalized().ok()).collect();
    state.mix.lock().unwrap().fill(generation,tracks);
    if failed {Err("YouTube did not offer a mix for this song. Choose another song to start a new mix.")}else{Ok(())}
}
pub async fn auto_next(state:Phone,ticket:u64)->(StatusCode,Json<Value>) {navigate_checked(state,false,Some(ticket)).await}
async fn next(Extension(state): Extension<Phone>) -> (StatusCode, Json<Value>) { navigate(state, false).await }
async fn previous(Extension(state): Extension<Phone>) -> (StatusCode, Json<Value>) { navigate(state, true).await }
async fn navigate(state: Phone, previous: bool) -> (StatusCode, Json<Value>) {
    navigate_checked(state,previous,None).await
}
async fn navigate_checked(state:Phone,previous:bool,expected:Option<u64>)->(StatusCode,Json<Value>) {
    let Ok(_permit) = state.operation_slot.try_acquire() else { return error(StatusCode::TOO_MANY_REQUESTS, "Track change already running"); };
    if expected.is_some_and(|ticket|ticket!=*crate::PLAY_TICKET.lock().unwrap()) {
        return error(StatusCode::CONFLICT,"Playback already changed");
    }
    let navigation_ticket=*crate::PLAY_TICKET.lock().unwrap();
    crate::sample_listening(&state);
    let (mut track, current, history, _liked) = {
        let library = state.library.lock().unwrap();
        (if previous { library.history.last().cloned() } else { library.queue.first().cloned() },
         library.current.clone(), library.history.clone(), library.liked.clone())
    };
    let old_current = current.clone();
    let queued = track.is_some();
    if !previous && track.is_none() {
        let Some(current) = current else { return error(StatusCode::CONFLICT, "Play or queue a song first"); };
        if state.mix.lock().unwrap().seed.is_empty(){state.mix.lock().unwrap().reset(&current.id);}
        if let Err(reason)=ensure_mix(&state).await {return error(StatusCode::BAD_GATEWAY,reason);}
        track=state.mix.lock().unwrap().choose(&current,&history);
    }
    let Some(track) = track else { return error(StatusCode::CONFLICT, if previous { "No previous track" } else { "No fresh songs left in this mix. Play another romantic song to start a new mix." }); };
    if navigation_ticket!=*crate::PLAY_TICKET.lock().unwrap() {
        return error(StatusCode::CONFLICT,"Playback already changed");
    }
    let result = crate::play_audio(crate::PlayBody { video_id: track.id.clone(), client: None, track: Some(track.clone()) }).await;
    if result.0.is_success() {
        *state.autoplay_error.lock().unwrap()=None;
        if !previous {
            state.mix.lock().unwrap().consumed(&track);
            if expected.is_none() {if let Some(old)=old_current {state.listening.lock().unwrap().skip(&old);}}
        }
        let mut library = state.library.lock().unwrap();
        if previous {
            library.history.pop();
            if let Some(current) = library.current.take() { library.queue.insert(0, current); library.queue.truncate(200); }
            library.current = Some(track.clone());
        } else {
            if queued && library.queue.first().is_some_and(|t| t.id == track.id) { library.queue.remove(0); }
            library.remember(track.clone());
        }
        drop(library);
        state.listening.lock().unwrap().flush();
        return (StatusCode::OK, Json(json!({"status":"playing","track":track,"mode":"native","already_started":true})));
    }
    if expected.is_some() && result.0!=StatusCode::CONFLICT {
        *state.autoplay_error.lock().unwrap()=Some("Your next song could not start. Tap Next or choose another song.".into());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test] async fn overlapping_cached_search_waits_instead_of_local429() {
        let state=new_state();
        let mut guard=state.search_cache.lock().await;
        guard.push(("romantic songs".into(),Instant::now(),vec![Track::from_id("abcdefghijk",Some("Melody".into()))]));
        let pending=tokio::spawn({let state=state.clone();async move {search(Extension(state),Query(Search{q:"Romantic songs".into()})).await}});
        tokio::task::yield_now().await;
        assert!(!pending.is_finished());
        drop(guard);
        let (status,body)=pending.await.unwrap();
        assert_eq!(status,StatusCode::OK);assert_eq!(body.0[0]["id"],"abcdefghijk");
        assert_eq!(search(Extension(state),Query(Search{q:"romantic songs".into()})).await.0,StatusCode::OK);
    }
    #[tokio::test] async fn stale_natural_end_cannot_start_after_stop_or_new_selection() {
        let state=new_state();
        let stale=(*crate::PLAY_TICKET.lock().unwrap()).wrapping_add(1);
        assert_eq!(auto_next(state,stale).await.0,StatusCode::CONFLICT);
    }
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
