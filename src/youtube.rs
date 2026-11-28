//! Rust direct-AAC metadata resolution. Media3's installable source-repair
//! candidate uses ONE researched public-client profile, not profile rotation.
//! No GPL extractor, JS execution, credentials, token provider or hosted helper.
//! A metadata URL is not acceptance: native playback still prepares head+tail.

use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::fmt;
use std::time::Duration;

const PLAYER_URL: &str = "https://www.youtube.com/youtubei/v1/player?prettyPrint=false";
const SEARCH_URL: &str = "https://www.youtube.com/youtubei/v1/search?prettyPrint=false";

/// One InnerTube client identity. `extra` is merged into the `client` object.
#[derive(Clone, Copy, Debug)]
pub struct InnertubeClient {
    pub name: &'static str,
    pub version: &'static str,
    pub user_agent: &'static str,
    extra: &'static str,
}

pub const ANDROID: InnertubeClient = InnertubeClient {
    name: "ANDROID",
    version: "20.10.38",
    user_agent: "com.google.android.youtube/20.10.38 (Linux; U; Android 11) gzip",
    extra: r#"{"androidSdkVersion":30,"osName":"Android","osVersion":"11"}"#,
};

pub const IOS: InnertubeClient = InnertubeClient {
    name: "IOS",
    version: "20.10.4",
    user_agent: "com.google.ios.youtube/20.10.4 (iPhone16,2; U; CPU iOS 18_1_0 like Mac OS X)",
    extra: r#"{"deviceModel":"iPhone16,2","osName":"iPhone","osVersion":"18.1.0.22B83"}"#,
};

pub const TVHTML5: InnertubeClient = InnertubeClient {
    name: "TVHTML5_SIMPLY_EMBEDDED_PLAYER",
    version: "2.0",
    user_agent: "Mozilla/5.0 (ChromiumStylePlatform) Cobalt/Version",
    extra: "{}",
};

/// Public profile fields reviewed in MIT youtubei.js18.3.0 Constants/HTTPClient.
/// Normal anonymous acquisition only, never a fallback after a sign-in challenge.
/// No source-access or phone-playback success is inferred from these constants.
pub const VISIONOS: InnertubeClient = InnertubeClient {
    name: "VISIONOS",
    version: "1.02",
    user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 15_7_3) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15",
    extra: r#"{"deviceMake":"Apple","deviceModel":"RealityDevice17,1","osName":"visionOS","osVersion":"26.5.23O471","platform":"MOBILE"}"#,
};

/// Legacy defaults retained for the non-Media3 Rust library path.
pub const CLIENTS: [InnertubeClient; 3] = [ANDROID, IOS, TVHTML5];
const SUPPORTED_CLIENTS: [InnertubeClient; 4] = [ANDROID, IOS, TVHTML5, VISIONOS];
pub const MEDIA3_CLIENTS: [InnertubeClient; 1] = [VISIONOS];
pub fn playback_clients(media3: bool) -> &'static [InnertubeClient] {
    if media3 { &MEDIA3_CLIENTS } else { &CLIENTS }
}

/// A provider denial is terminal, not a prompt to try another identity.
pub fn terminal_provider_error(error: &str) -> bool {
    let text = error.to_ascii_lowercase();
    ["login_required", "sign in", "sign-in", "not a bot", "captcha", "provider_sign_in_or_bot_challenge"].iter().any(|word| text.contains(word))
}

/// The names accepted by `/api/play`'s optional `client` field.
pub const CLIENT_ALIASES: [(&str, &str); 4] = [
    ("android", "ANDROID"),
    ("ios", "IOS"),
    ("tv", "TVHTML5_SIMPLY_EMBEDDED_PLAYER"),
    ("visionos", "VISIONOS"),
];

/// Resolves `"android" | "ios" | "tv" | "visionos"` (any case) to a client profile.
pub fn client_by_name(name: &str) -> Option<InnertubeClient> {
    let needle = name.trim().to_ascii_lowercase();
    let canonical = CLIENT_ALIASES
        .iter()
        .find(|(alias, _)| *alias == needle)
        .map(|(_, canonical)| *canonical)
        .or_else(|| {
            SUPPORTED_CLIENTS
                .iter()
                .find(|client| client.name.eq_ignore_ascii_case(&needle))
                .map(|client| client.name)
        })?;
    SUPPORTED_CLIENTS.iter().copied().find(|client| client.name == canonical)
}

pub fn client_alias_list() -> String {
    CLIENT_ALIASES
        .iter()
        .map(|(alias, _)| *alias)
        .collect::<Vec<_>>()
        .join(", ")
}

/// An audio stream that `rodio` can decode, plus the metadata the UI needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedAudio {
    pub video_id: String,
    pub title: Option<String>,
    pub url: String,
    pub itag: u64,
    pub mime: String,
    pub bitrate: u64,
    pub content_length: Option<u64>,
    pub client: &'static str,
}

impl ResolvedAudio {
    /// Media reads must keep the identity that acquired this signed URL.
    pub fn user_agent(&self)->&'static str {
        client_by_name(self.client).map(|p|p.user_agent).unwrap_or(ANDROID.user_agent)
    }
}

/// One client's outcome, including YouTube's own explanation and whether the
/// request carried a visitor id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientAttempt {
    pub client: &'static str,
    pub status: Option<u16>,
    pub visitor: &'static str,
    pub error: String,
}

impl ClientAttempt {
    pub fn to_json(&self) -> Value {
        json!({
            "client": self.client,
            "status": self.status,
            "visitor": self.visitor,
            "error": self.error,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ResolveError {
    InvalidVideoId,
    /// HTTP transport failure before YouTube was reached.
    Http(String),
    /// YouTube answered, but refused to play this video.
    Unplayable(String),
    /// YouTube answered, but offered no direct audio URL.
    NoAudio,
    AllClientsFailed(Vec<ClientAttempt>),
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidVideoId => write!(f, "video_id must be 11 characters of [A-Za-z0-9_-]"),
            Self::Http(detail) => write!(f, "youtube request failed: {detail}"),
            Self::Unplayable(detail) => write!(f, "video is not playable: {detail}"),
            Self::NoAudio => write!(f, "no audio-only AAC stream offered"),
            Self::AllClientsFailed(attempts) => {
                write!(f, "all clients failed: ")?;
                for (index, attempt) in attempts.iter().enumerate() {
                    if index > 0 {
                        write!(f, "; ")?;
                    }
                    write!(f, "{}: {}", attempt.client, attempt.error)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ResolveError {}

/// A shared HTTP client sized for phone networks.
pub fn http_client() -> Result<Client, reqwest::Error> {
    let builder=Client::builder()
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::none())
        .pool_max_idle_per_host(2);
    // The Windows media reader uses no proxy. Keep metadata and media on the
    // same explicit route, rather than accidentally resolving behind a proxy
    // and fetching its signed URL directly. Never rotate or fall back routes.
    #[cfg(target_os="windows")]
    let builder=windows_direct_network(builder);
    builder.build()
}

#[cfg(target_os="windows")]
fn windows_direct_network(builder:reqwest::ClientBuilder)->reqwest::ClientBuilder {builder.no_proxy()}

/// Client for `googlevideo` bytes.
///
/// A track streams for minutes, so it must not inherit the API client's 12 s
/// total timeout: that cap silently cut every song off after ~12 seconds of
/// audio. Stalled reads are still bounded so a dead connection cannot hang the
/// playback thread forever.
pub fn media_client() -> Result<Client, reqwest::Error> {
    Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(30))
        .pool_max_idle_per_host(2)
        .build()
}

const VISITOR_URL: &str = "https://www.youtube.com/youtubei/v1/visitor_id?prettyPrint=false";
const WEB_VERSION: &str = "2.20241126.01.00";
const WEB_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36";

/// A visitor identity shared by every player request. YouTube ties its bot
/// checks to this value, so it is persisted and reused across app restarts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Visitor {
    pub value: String,
    pub source: VisitorSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisitorSource {
    Disk,
    Network,
    Unavailable,
}

impl VisitorSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disk => "disk",
            Self::Network => "network",
            Self::Unavailable => "unavailable",
        }
    }
}

impl Visitor {
    pub fn attached(&self) -> bool {
        !self.value.is_empty()
    }

    pub fn preview(&self) -> String {
        self.value.chars().take(8).collect()
    }
}

pub fn valid_video_id(id: &str) -> bool {
    id.len() == 11 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Meta query helper: load a cached visitor id from `path` if it is sane.
pub fn load_visitor(path: Option<&std::path::Path>) -> Option<String> {
    let path = path?;
    let bytes = std::fs::read(path).ok()?;
    let json: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let value = json["value"].as_str()?;
    sane_visitor(value).then(|| value.to_string())
}

/// Returns a visitor id, preferring the cached one, and persists a fresh fetch.
pub async fn ensure_visitor(
    http: &Client,
    path: Option<&std::path::Path>,
) -> Visitor {
    if let Some(value) = load_visitor(path) {
        return Visitor {
            value,
            source: VisitorSource::Disk,
        };
    }
    match fetch_visitor(http).await {
        Some(value) => {
            save_visitor(path, &value);
            Visitor {
                value,
                source: VisitorSource::Network,
            }
        }
        None => Visitor {
            value: String::new(),
            source: VisitorSource::Unavailable,
        },
    }
}

/// A visitor id is an opaque URL-encoded blob; anything with whitespace or
/// quotes is a poisoned cache entry, not an id.
fn sane_visitor(value: &str) -> bool {
    let length = value.len();
    (16..=4096).contains(&length)
        && !value
            .chars()
            .any(|c| c.is_whitespace() || c == '"' || c == '\\')
}

fn save_visitor(path: Option<&std::path::Path>, value: &str) {
    let Some(path) = path else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let payload = json!({
        "value": value,
        "fetched_at_unix": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .unwrap_or(0),
    });
    let _ = std::fs::write(path, payload.to_string());
}

async fn fetch_visitor(http: &Client) -> Option<String> {
    let body = json!({
        "context": {
            "client": {
                "clientName": "WEB",
                "clientVersion": WEB_VERSION,
                "hl": "en",
                "gl": "US",
            }
        }
    });
    let response = http
        .post(VISITOR_URL)
        .header(reqwest::header::USER_AGENT, WEB_USER_AGENT)
        .json(&body)
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let json: serde_json::Value = response.json().await.ok()?;
    let value = json["responseContext"]["visitorData"]
        .as_str()
        .or_else(|| json["visitorData"].as_str())?;
    sane_visitor(value).then(|| value.to_string())
}

/// Only plain HTTPS URLs on YouTube's media CDN are ever fetched.
pub fn is_allowed_media_url(raw: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(raw) else {
        return false;
    };
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    if url.port().is_some_and(|port| port != 443) {
        return false;
    }
    url.host_str()
        .is_some_and(|host| host == "googlevideo.com" || host.ends_with(".googlevideo.com"))
}

/// Resolves `video_id` to a decodable audio URL, falling back across clients.
pub async fn resolve_audio(
    http: &Client,
    video_id: &str,
    visitor: Option<&str>,
) -> Result<ResolvedAudio, ResolveError> {
    resolve_with_clients(http, video_id, &CLIENTS, visitor).await
}

/// Resolves with exactly one client, so its real error is reported verbatim.
pub async fn resolve_forced(
    http: &Client,
    video_id: &str,
    profile: InnertubeClient,
    visitor: Option<&str>,
) -> Result<ResolvedAudio, ResolveError> {
    resolve_with_clients(http, video_id, &[profile], visitor).await
}

async fn resolve_with_clients(
    http: &Client,
    video_id: &str,
    profiles: &[InnertubeClient],
    visitor: Option<&str>,
) -> Result<ResolvedAudio, ResolveError> {
    if !valid_video_id(video_id) {
        return Err(ResolveError::InvalidVideoId);
    }

    let visitor_label = if visitor.is_some() { "attached" } else { "none" };
    let mut attempts: Vec<ClientAttempt> = Vec::new();
    for profile in profiles {
        let (status, outcome) = resolve_with(http, video_id, *profile, visitor).await;
        match outcome {
            Ok(audio) => return Ok(audio),
            Err(error) => {
                let terminal = terminal_provider_error(&error.to_string());
                attempts.push(ClientAttempt { client: profile.name, status,
                    visitor: visitor_label, error: error.to_string() });
                if terminal { break; }
            },
        }
    }
    Err(ResolveError::AllClientsFailed(attempts))
}

async fn resolve_with(
    http: &Client,
    video_id: &str,
    profile: InnertubeClient,
    visitor: Option<&str>,
) -> (Option<u16>, Result<ResolvedAudio, ResolveError>) {
    resolve_with_endpoint(http, video_id, profile, visitor, PLAYER_URL).await
}

// Private injection point for offline transport tests, not exposed to API callers.
async fn resolve_with_endpoint(http: &Client, video_id: &str, profile: InnertubeClient,
    visitor: Option<&str>, endpoint: &str) -> (Option<u16>, Result<ResolvedAudio, ResolveError>) {
    let body = request_body(video_id, profile, visitor);
    let mut request = http
        .post(endpoint)
        .header(reqwest::header::USER_AGENT, profile.user_agent)
        .header("X-YouTube-Client-Name", client_id(profile.name))
        .header("X-YouTube-Client-Version", profile.version);
    if let Some(visitor) = visitor {
        request = request.header("X-Goog-Visitor-Id", visitor);
    }
    let response = match request.json(&body).send().await {
        Ok(response) => response,
        Err(error) => return (None, Err(ResolveError::Http(error.without_url().to_string()))),
    };

    let status = response.status();
    if !status.is_success() {
        if profile.name == VISIONOS.name {
            return (Some(status.as_u16()), Err(ResolveError::Http(format!("provider_http_{}", status.as_u16()))));
        }
        // Keep YouTube's own words: HTTP 400/403 bodies explain the refusal
        // (e.g. FAILED_PRECONDITION vs LOGIN_REQUIRED) and drive the next fix.
        let text = response.text().await.unwrap_or_default();
        return (
            Some(status.as_u16()),
            Err(ResolveError::Http(format!(
                "HTTP {status}: {}",
                summarize_error_body(&text)
            ))),
        );
    }
    // Bound metadata buffering; never deserialize an unlimited provider body.
    use futures_util::StreamExt;
    let mut chunks = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = chunks.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(_) => return (Some(status.as_u16()), Err(ResolveError::Http("player_body_failed".into()))),
        };
        if bytes.len() + chunk.len() > 2 * 1024 * 1024 {
            return (Some(status.as_u16()), Err(ResolveError::Http("player_metadata_size_limit".into())));
        }
        bytes.extend_from_slice(&chunk);
    }
    let json: Value = match serde_json::from_slice(&bytes) {
        Ok(json) => json,
        Err(_) => return (Some(status.as_u16()), Err(ResolveError::Http("player_metadata_invalid".into()))),
    };

    (
        Some(status.as_u16()),
        parse_player_json(&json, video_id, profile.name),
    )
}

fn client_id(name: &str) -> &'static str {
    match name {
        "ANDROID" => "3",
        "IOS" => "5",
        "VISIONOS" => "101",
        "TVHTML5_SIMPLY_EMBEDDED_PLAYER" => "85",
        _ => "1",
    }
}

/// Extracts the useful part of an InnerTube error body, bounded and single-line.
pub fn summarize_error_body(body: &str) -> String {
    const LIMIT: usize = 300;

    if let Ok(json) = serde_json::from_str::<Value>(body) {
        let message = json["error"]["message"]
            .as_str()
            .or_else(|| json["error"]["status"].as_str());
        if let Some(message) = message {
            let detail = format!("{message}");
            let reason = json["error"]["errors"][0]["reason"].as_str();
            return match reason {
                Some(reason) if !reason.is_empty() => truncate(&format!("{detail} ({reason})"), LIMIT),
                _ => truncate(&detail, LIMIT),
            };
        }
    }

    let compact = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.is_empty() {
        return "empty response body".to_string();
    }
    truncate(&compact, LIMIT)
}

fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut truncated: String = text.chars().take(limit).collect();
    truncated.push('…');
    truncated
}

fn client_context(profile: InnertubeClient, visitor: Option<&str>) -> Value {
    let extra: Value = serde_json::from_str(profile.extra).unwrap_or_else(|_| json!({}));
    let mut client = json!({
        "clientName": profile.name,
        "clientVersion": profile.version,
        "hl": "en",
        "gl": "US",
        "utcOffsetMinutes": 0,
    });
    if let (Some(client), Value::Object(extra)) = (client.as_object_mut(), extra) {
        client.extend(extra);
        if profile.name == VISIONOS.name {
            client.insert("userAgent".into(), Value::String(profile.user_agent.into()));
        }
        if let Some(visitor) = visitor {
            client.insert("visitorData".to_string(), Value::String(visitor.to_string()));
        }
    }
    json!({
        "client": client,
        "user": { "lockedSafetyMode": false },
        "request": { "useSsl": true },
    })
}

fn request_body(video_id: &str, profile: InnertubeClient, visitor: Option<&str>) -> Value {
    json!({
        "context": client_context(profile, visitor),
        "videoId": video_id,
        "contentCheckOk": true,
        "racyCheckOk": true,
    })
}

/// One search result: enough to show a card and resolve it later.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchHit {
    pub video_id: String,
    pub title: String,
    pub channel: String,
}

/// Normal anonymous YouTube radio metadata, not a title-search approximation.
/// Public request/playlist shapes reviewed in MIT youtubei.js18.3.0.
pub async fn mix(http:&Client, seed:&str, visitor:Option<&str>)->Result<Vec<SearchHit>,ResolveError> {
    if !valid_video_id(seed) {return Err(ResolveError::InvalidVideoId);}
    let web=InnertubeClient{name:"WEB",version:"2.20260623.01.00",user_agent:WEB_USER_AGENT,extra:"{}"};
    let body=json!({"context":client_context(web,visitor),"videoId":seed,"playlistId":format!("RD{seed}")});
    let mut request=http.post("https://www.youtube.com/youtubei/v1/next?prettyPrint=false")
        .header(reqwest::header::USER_AGENT,web.user_agent)
        .header("X-YouTube-Client-Name","1").header("X-YouTube-Client-Version",web.version);
    if let Some(visitor)=visitor {request=request.header("X-Goog-Visitor-Id",visitor);}
    let response=request.json(&body).send().await.map_err(|_|ResolveError::Http("mix_network_unavailable".into()))?;
    if !response.status().is_success() {return Err(ResolveError::Http(format!("mix_http_{}",response.status().as_u16())));}
    use futures_util::StreamExt;
    let mut stream=response.bytes_stream();let mut bytes=Vec::new();
    while let Some(chunk)=stream.next().await {
        let chunk=chunk.map_err(|_|ResolveError::Http("mix_response_incomplete".into()))?;
        if bytes.len()+chunk.len()>2*1024*1024 {return Err(ResolveError::Http("mix_metadata_size_limit".into()));}
        bytes.extend_from_slice(&chunk);
    }
    let value:Value=serde_json::from_slice(&bytes).map_err(|_|ResolveError::Http("mix_metadata_invalid".into()))?;
    parse_mix_json(&value,seed)
}
fn mix_label(value:&Value)->String {
    value["simpleText"].as_str().map(str::to_owned).unwrap_or_else(||value["runs"].as_array()
        .map(|runs|runs.iter().take(20).filter_map(|r|r["text"].as_str()).collect()).unwrap_or_default())
        .chars().take(300).collect()
}
fn parse_mix_json(value:&Value,seed:&str)->Result<Vec<SearchHit>,ResolveError> {
    if value.get("error").is_some() || value["playabilityStatus"]["status"].as_str().is_some_and(|s|s!="OK") {
        return Err(ResolveError::Unplayable("mix_unavailable".into()));
    }
    let panel=value.pointer("/contents/twoColumnWatchNextResults/playlist/playlist")
        .or_else(||value.pointer("/contents/singleColumnWatchNextResults/playlist/playlist"))
        .ok_or_else(||ResolveError::Unplayable("mix_not_offered_for_this_song".into()))?;
    if panel["playlistId"].as_str()!=Some(format!("RD{seed}").as_str()) {
        return Err(ResolveError::Unplayable("mix_playlist_mismatch".into()));
    }
    let items=panel["contents"].as_array().ok_or(ResolveError::NoAudio)?;
    let mut seen=std::collections::HashSet::new();let mut out=Vec::new();
    for item in items.iter().take(200) {
        let node=&item["playlistPanelVideoRenderer"];
        let Some(id)=node["videoId"].as_str().filter(|id|valid_video_id(id)) else {continue;};
        if node["isPlayable"].as_bool()==Some(false) || !seen.insert(id.to_owned()) {continue;}
        let title=mix_label(&node["title"]);if title.is_empty(){continue;}
        let mut channel=mix_label(&node["shortBylineText"]);
        if channel.is_empty(){channel=mix_label(&node["longBylineText"]);}
        out.push(SearchHit{video_id:id.into(),title,channel});if out.len()==40{break;}
    }
    Ok(out)
}

/// InnerTube search (`/youtubei/v1/search`). Used by `/api/search` and by
/// `/api/next`, which asks for songs similar to the one that is playing.
pub async fn search(
    http: &Client,
    query: &str,
    visitor: Option<&str>,
    limit: usize,
) -> Result<Vec<SearchHit>, ResolveError> {
    let query = query.trim();
    if query.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }
    let body = json!({
        "context": client_context(ANDROID, visitor),
        "query": query,
    });
    let mut request = http
        .post(SEARCH_URL)
        .header(reqwest::header::USER_AGENT, ANDROID.user_agent)
        .header("X-YouTube-Client-Name", client_id(ANDROID.name))
        .header("X-YouTube-Client-Version", ANDROID.version);
    if let Some(visitor) = visitor {
        request = request.header("X-Goog-Visitor-Id", visitor);
    }
    let response = request
        .json(&body)
        .send()
        .await
        .map_err(|error| ResolveError::Http(error.to_string()))?;
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(ResolveError::Http(format!(
            "search HTTP {status}: {}",
            summarize_error_body(&text)
        )));
    }
    let json: Value = response
        .json()
        .await
        .map_err(|error| ResolveError::Http(error.to_string()))?;
    Ok(parse_search_json(&json, limit))
}

/// Walks any InnerTube search shape and keeps every video node. The web client
/// answers with `videoRenderer`, the mobile client with `compactVideoRenderer`;
/// both carry `videoId`, `title` and `longBylineText`.
fn parse_search_json(json: &Value, limit: usize) -> Vec<SearchHit> {
    let mut hits: Vec<SearchHit> = Vec::new();
    collect_video_renderers(json, &mut hits, limit);
    let mut seen = std::collections::HashSet::new();
    hits.retain(|hit| seen.insert(hit.video_id.clone()));
    hits
}

fn collect_video_renderers(value: &Value, out: &mut Vec<SearchHit>, limit: usize) {
    if out.len() >= limit {
        return;
    }
    match value {
        Value::Object(map) => {
            for key in ["videoRenderer", "compactVideoRenderer", "videoWithContextRenderer"] {
                if let Some(renderer) = map.get(key) {
                    if let Some(hit) = hit_from_renderer(renderer) {
                        out.push(hit);
                    }
                }
            }
            for child in map.values() {
                collect_video_renderers(child, out, limit);
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_video_renderers(child, out, limit);
            }
        }
        _ => {}
    }
}

fn hit_from_renderer(renderer: &Value) -> Option<SearchHit> {
    let video_id = renderer["videoId"].as_str()?.trim().to_string();
    if !valid_video_id(&video_id) {
        return None;
    }
    let title = renderer["title"]["runs"][0]["text"]
        .as_str()
        .or_else(|| renderer["title"]["simpleText"].as_str())?
        .trim()
        .to_string();
    if title.is_empty() {
        return None;
    }
    let channel = renderer["ownerText"]["runs"][0]["text"]
        .as_str()
        .or_else(|| renderer["longBylineText"]["runs"][0]["text"].as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    Some(SearchHit { video_id, title, channel })
}

/// Collapses a YouTube title to its song identity so re-uploads, lyric videos
/// and "(Official Video)" copies compare equal. Used to keep `/api/next` from
/// queueing the same song again under a different upload.
pub fn normalized_title(title: &str) -> String {
    let mut kept = String::with_capacity(title.len());
    let mut depth = 0usize;
    for ch in title.chars() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            '|' if depth == 0 => break,
            _ if depth > 0 => {}
            _ => {
                if ch.is_alphanumeric() {
                    kept.extend(ch.to_lowercase());
                } else {
                    kept.push(' ');
                }
            }
        }
    }
    kept.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Upload noise that says nothing about which song it is.
const TITLE_NOISE: [&str; 13] = [
    "official", "video", "lyrics", "lyric", "lyrical", "audio", "song", "full", "hd", "4k",
    "visualizer", "version", "music",
];

fn strip_title_noise(title: &str) -> String {
    let mut words: Vec<&str> = title.split_whitespace().collect();
    while words
        .last()
        .is_some_and(|word| TITLE_NOISE.contains(word))
    {
        words.pop();
    }
    words.join(" ")
}

/// True when `candidate` is the same song the player is already on, even if the
/// upload title differs ("Song (Official Video)" vs "Song | Lyric Video").
///
/// Comparison is exact after noise removal, so a sequel ("Song 2") is a new
/// song, while a re-upload is not.
pub fn same_song(current: &str, candidate: &str) -> bool {
    let current = strip_title_noise(&normalized_title(current));
    let candidate = strip_title_noise(&normalized_title(candidate));
    !current.is_empty() && current == candidate
}

#[derive(Deserialize)]
struct AdaptiveFormat {
    itag: u64,
    #[serde(rename = "mimeType")]
    mime_type: String,
    #[serde(default)]
    bitrate: u64,
    #[serde(default)]
    url: Option<String>,
    #[serde(default, rename = "contentLength")]
    content_length: Option<String>,
}

/// Parses a raw `player` response into the best audio-only AAC stream.
pub fn parse_player_json(
    json: &Value,
    video_id: &str,
    client: &'static str,
) -> Result<ResolvedAudio, ResolveError> {
    let playability = &json["playabilityStatus"];
    if client == VISIONOS.name {
        if playability["status"].as_str() == Some("LOGIN_REQUIRED") ||
            terminal_provider_error(&playability.to_string()) {
            return Err(ResolveError::Unplayable("provider_sign_in_or_bot_challenge".into()));
        }
        if playability["status"].as_str() != Some("OK") {
            return Err(ResolveError::Unplayable("provider_content_unavailable".into()));
        }
        if json["videoDetails"]["videoId"].as_str() != Some(video_id) {
            return Err(ResolveError::Unplayable("source_video_id_mismatch".into()));
        }
        if ["isLive", "isPostLiveDvr", "isUpcoming", "isPrivate"].iter()
            .any(|key| json["videoDetails"][*key].as_bool() == Some(true)) {
            return Err(ResolveError::Unplayable("source_content_unsupported".into()));
        }
    }
    if playability["status"].as_str() != Some("OK") {
        let reason = playability["reason"]
            .as_str()
            .or_else(|| playability["status"].as_str())
            .unwrap_or("unknown");
        // `playabilityStatus.errorScreen` often carries a more specific text.
        let detail = playability["errorScreen"]["playerErrorMessageRenderer"]["reason"]["simpleText"]
            .as_str()
            .unwrap_or(reason);
        return Err(ResolveError::Unplayable(detail.to_string()));
    }

    let formats = json["streamingData"]["adaptiveFormats"]
        .as_array()
        .ok_or(ResolveError::NoAudio)?;

    let parsed: Vec<AdaptiveFormat> = formats
        .iter()
        .filter(|value| client != VISIONOS.name || candidate_direct_aac(value))
        .filter_map(|value| serde_json::from_value(value.clone()).ok())
        .collect();

    let best = select_audio(&parsed)?;
    let url = best.url.as_deref().ok_or(ResolveError::NoAudio)?;
    if !is_allowed_media_url(url) {
        return Err(ResolveError::NoAudio);
    }

    Ok(ResolvedAudio {
        video_id: video_id.to_string(),
        title: json["videoDetails"]["title"].as_str().map(str::to_string),
        url: url.to_string(),
        itag: best.itag,
        mime: best.mime_type.clone(),
        bitrate: best.bitrate,
        content_length: best
            .content_length
            .as_deref()
            .and_then(|value| value.parse().ok()),
        client,
    })
}

// No silent signature/n bypass. This direct-AAC candidate needs an already
// usable URL; unsupported signing/DRM/manifest cases fail before media transfer.
fn candidate_direct_aac(value: &Value) -> bool {
    if value["signatureCipher"].is_string() || value["cipher"].is_string() ||
        value["qualityLabel"].is_string() || value["drmFamilies"].as_array().is_some_and(|v| !v.is_empty()) ||
        value["drmTrackType"].is_string() || value["type"].as_str() == Some("FORMAT_STREAM_TYPE_OTF") ||
        value["mimeType"].as_str() != Some("audio/mp4; codecs=\"mp4a.40.2\"") {
        return false;
    }
    let Some(length) = value["contentLength"].as_str().and_then(|s| s.parse::<u64>().ok()) else { return false; };
    if length == 0 || length > 512 * 1024 * 1024 { return false; }
    let Some(url) = value["url"].as_str().and_then(|s| reqwest::Url::parse(s).ok()) else { return false; };
    is_allowed_media_url(url.as_str()) && url.path() == "/videoplayback" && url.fragment().is_none() &&
        !url.query_pairs().any(|(key, _)| key == "n")
}

/// Highest-bitrate `audio/mp4` format with a direct, allow-listed URL.
fn select_audio(formats: &[AdaptiveFormat]) -> Result<&AdaptiveFormat, ResolveError> {
    formats
        .iter()
        .filter(|format| format.mime_type.starts_with("audio/mp4"))
        .filter(|format| format.url.as_deref().is_some_and(is_allowed_media_url))
        .max_by_key(|format| format.bitrate)
        .ok_or(ResolveError::NoAudio)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os="windows")]
    #[tokio::test]
    async fn windows_direct_network_disables_explicit_proxy_offline() {
        use std::io::{Read,Write};
        use std::net::TcpListener;
        let target=TcpListener::bind("127.0.0.1:0").unwrap();
        let target_addr=target.local_addr().unwrap();
        target.set_nonblocking(true).unwrap();
        let proxy=TcpListener::bind("127.0.0.1:0").unwrap();
        proxy.set_nonblocking(true).unwrap();
        let server=std::thread::spawn(move || {
            let deadline=std::time::Instant::now()+std::time::Duration::from_secs(4);
            while std::time::Instant::now()<deadline {
                if let Ok((mut conn,_))=target.accept() {
                    let mut bytes=[0u8;2048];let _=conn.read(&mut bytes);
                    conn.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok").unwrap();
                    return true;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            false
        });
        let proxy_url=format!("http://{}",proxy.local_addr().unwrap());
        let builder=Client::builder().proxy(reqwest::Proxy::all(proxy_url).unwrap()).timeout(Duration::from_secs(2));
        let client=windows_direct_network(builder).build().unwrap();
        let result=client.get(format!("http://{target_addr}/fixture")).send().await;
        assert!(server.join().unwrap(),"request did not reach the direct fixture");
        assert_eq!(result.unwrap().text().await.unwrap(),"ok");
        assert!(matches!(proxy.accept(),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock));
    }

    fn visionos_fixture() -> Value {
        json!({"playabilityStatus":{"status":"OK"},
            "videoDetails":{"videoId":"fixture0001","title":"Offline fixture, not the song"},
            "streamingData":{"adaptiveFormats":[{"itag":140,"mimeType":"audio/mp4; codecs=\"mp4a.40.2\"",
                "bitrate":128000,"contentLength":"123456",
                "url":"https://rr1-fixture.googlevideo.com/videoplayback?sig=FAKE_ONLY"}]}})
    }

    #[test] fn media3_uses_one_reviewed_profile_without_changing_legacy_defaults() {
        assert_eq!(playback_clients(true).len(), 1);
        assert_eq!(playback_clients(true)[0].name, "VISIONOS");
        assert_eq!(playback_clients(false).iter().map(|p|p.name).collect::<Vec<_>>(), vec![ANDROID.name, IOS.name, TVHTML5.name]);
        assert_eq!(client_by_name("VISIONOS").unwrap().user_agent, VISIONOS.user_agent);
        assert_eq!(client_id(VISIONOS.name), "101");
        let body = request_body("fixture0001", VISIONOS, Some("fixture_visitor_only"));
        assert_eq!(body["context"]["client"]["clientVersion"], "1.02");
        assert_eq!(body["context"]["client"]["platform"], "MOBILE");
        assert_eq!(body["context"]["client"]["visitorData"], "fixture_visitor_only");
        assert_eq!(body["context"]["client"]["userAgent"], VISIONOS.user_agent);
        assert!(body.get("serviceIntegrityDimensions").is_none());
    }

    #[test] fn candidate_exact_id_denials_and_unsupported_sources_are_rejected() {
        let good = visionos_fixture();
        let audio = parse_player_json(&good,"fixture0001",VISIONOS.name).unwrap();
        assert_eq!(audio.itag,140); assert_eq!(audio.user_agent(), VISIONOS.user_agent);
        assert_eq!(audio.content_length,Some(123456));
        let mut wrong=good.clone(); wrong["videoDetails"]["videoId"]=json!("other000001");
        assert!(parse_player_json(&wrong,"fixture0001",VISIONOS.name).unwrap_err().to_string().contains("source_video_id_mismatch"));
        let mut denied=good.clone(); denied["playabilityStatus"]=json!({"status":"LOGIN_REQUIRED","reason":"PRIVATE_DO_NOT_LOG"});
        let error=parse_player_json(&denied,"fixture0001",VISIONOS.name).unwrap_err().to_string();
        assert!(terminal_provider_error(&error)); assert!(!error.contains("PRIVATE_DO_NOT_LOG"));
        for (key,value) in [("signatureCipher",json!("s=FAKE")),("cipher",json!("s=FAKE")),
            ("contentLength",json!("0")),("contentLength",json!("536870913")),("contentLength",json!("123bad")),
            ("url",json!("https://rr1-fixture.googlevideo.com/videoplayback?n=FAKE")),
            ("url",json!("https://evil.test/videoplayback")),("drmFamilies",json!(["WIDEVINE"])),
            ("qualityLabel",json!("1080p")),("type",json!("FORMAT_STREAM_TYPE_OTF"))] {
            let mut bad=good.clone();bad["streamingData"]["adaptiveFormats"][0][key]=value;
            assert!(parse_player_json(&bad,"fixture0001",VISIONOS.name).is_err(),"accepted {key}");
        }
    }

    #[tokio::test] async fn candidate_actual_http_request_keeps_profile_and_visitor_without_credentials() {
        use axum::{Router,routing::post,http::HeaderMap,Json};
        let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint=format!("http://{}/player",listener.local_addr().unwrap());
        let app=Router::new().route("/player",post(|headers:HeaderMap,Json(body):Json<Value>|async move {
            assert_eq!(headers["user-agent"], VISIONOS.user_agent);
            assert_eq!(headers["x-youtube-client-name"], "101");
            assert_eq!(headers["x-youtube-client-version"], "1.02");
            assert_eq!(headers["x-goog-visitor-id"], "fixture_visitor_only");
            assert!(!headers.contains_key("cookie"));assert!(!headers.contains_key("authorization"));
            assert_eq!(body["videoId"],"fixture0001");
            assert_eq!(body["context"]["client"]["clientName"],"VISIONOS");
            Json(visionos_fixture())
        }));
        let task=tokio::spawn(async move {axum::serve(listener,app).await.unwrap();});
        let (status,result)=resolve_with_endpoint(&http_client().unwrap(),"fixture0001",VISIONOS,Some("fixture_visitor_only"),&endpoint).await;
        task.abort(); assert_eq!(status,Some(200));assert_eq!(result.unwrap().client,VISIONOS.name);
    }

    #[tokio::test] async fn candidate_metadata_size_http_errors_and_redirects_fail_safely() {
        use axum::{Router,routing::post,http::{StatusCode,header},response::IntoResponse};
        let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base=format!("http://{}",listener.local_addr().unwrap());
        let app=Router::new()
            .route("/large",post(||async {"x".repeat(2*1024*1024+1)}))
            .route("/invalid",post(||async {"PRIVATE_INVALID_METADATA"}))
            .route("/denied",post(||async {(StatusCode::FORBIDDEN,"PRIVATE_BODY_DO_NOT_LOG")}))
            .route("/redirect",post(||async {(StatusCode::FOUND,[(header::LOCATION,"http://127.0.0.1:1/never-follow")],"").into_response()}));
        let task=tokio::spawn(async move {axum::serve(listener,app).await.unwrap();});
        for (path,code) in [("large","player_metadata_size_limit"),("invalid","player_metadata_invalid"),("denied","provider_http_403"),("redirect","provider_http_302")] {
            let (_,result)=resolve_with_endpoint(&http_client().unwrap(),"fixture0001",VISIONOS,None,&format!("{base}/{path}")).await;
            let error=result.unwrap_err().to_string();assert!(error.contains(code),"{error}");
            assert!(!error.contains("PRIVATE"));assert!(!error.contains("127.0.0.1"));
        }
        task.abort();
    }

    #[test] fn radio_parser_keeps_actual_playlist_order_not_secondary_recommendations() {
        let value=json!({"contents":{"twoColumnWatchNextResults":{
            "secondaryResults":{"videoRenderer":{"videoId":"unrelated01","title":{"simpleText":"Not mix"}}},
            "playlist":{"playlist":{"playlistId":"RDfixture0001","contents":[
                {"playlistPanelVideoRenderer":{"videoId":"fixture0001","title":{"simpleText":"Seed"}}},
                {"playlistPanelVideoRenderer":{"videoId":"nexttrack01","title":{"runs":[{"text":"Next melody"}]},"shortBylineText":{"runs":[{"text":"Channel"}]}}},
                {"playlistPanelVideoRenderer":{"videoId":"nexttrack01","title":{"simpleText":"Duplicate"}}},
                {"playlistPanelVideoRenderer":{"videoId":"private0001","title":{"simpleText":"Private"},"isPlayable":false}}
            ]}}}}});
        let hits=parse_mix_json(&value,"fixture0001").unwrap();
        assert_eq!(hits.iter().map(|h|h.video_id.as_str()).collect::<Vec<_>>(),vec!["fixture0001","nexttrack01"]);
        assert_eq!(hits[1].channel,"Channel");
        assert!(parse_mix_json(&value,"otherseed01").is_err());
        assert!(parse_mix_json(&json!({"contents":{}}),"fixture0001").is_err());
        assert!(parse_mix_json(&json!({"playabilityStatus":{"status":"LOGIN_REQUIRED"}}),"fixture0001").is_err());
    }

    #[test]
    fn video_ids_are_strictly_shaped() {
        assert!(valid_video_id("dQw4w9WgXcQ"));
        assert!(valid_video_id("a_b-c_D1234"));
        assert!(!valid_video_id("dQw4w9WgXc"));
        assert!(!valid_video_id("dQw4w9WgXcQQ"));
        assert!(!valid_video_id("dQw4w9WgXc/"));
        assert!(!valid_video_id("../etc/passwd"));
    }

    #[test]
    fn media_urls_are_restricted_to_the_youtube_cdn() {
        assert!(is_allowed_media_url(
            "https://rr1---sn-abc.googlevideo.com/videoplayback?itag=140"
        ));
        assert!(is_allowed_media_url(
            "https://googlevideo.com/videoplayback?itag=140"
        ));
        assert!(!is_allowed_media_url(
            "http://rr1---sn-abc.googlevideo.com/videoplayback"
        ));
        assert!(!is_allowed_media_url(
            "https://googlevideo.com.evil.example/videoplayback"
        ));
        assert!(!is_allowed_media_url(
            "https://evil.example/googlevideo.com"
        ));
        assert!(!is_allowed_media_url(
            "https://user:pass@rr1.googlevideo.com/videoplayback"
        ));
        assert!(!is_allowed_media_url(
            "https://rr1.googlevideo.com:8443/videoplayback"
        ));
        assert!(!is_allowed_media_url("file:///data/local/tmp/a.m4a"));
    }

    #[test]
    fn client_aliases_and_names_are_accepted() {
        assert_eq!(client_by_name("android").map(|c| c.name), Some("ANDROID"));
        assert_eq!(client_by_name(" iOS ").map(|c| c.name), Some("IOS"));
        assert_eq!(
            client_by_name("tv").map(|c| c.name),
            Some("TVHTML5_SIMPLY_EMBEDDED_PLAYER")
        );
        assert_eq!(
            client_by_name("TVHTML5_SIMPLY_EMBEDDED_PLAYER").map(|c| c.name),
            Some("TVHTML5_SIMPLY_EMBEDDED_PLAYER")
        );
        assert!(client_by_name("web").is_none());
        assert!(client_by_name("").is_none());
        assert_eq!(client_alias_list(), "android, ios, tv, visionos");
    }

    #[test]
    fn youtube_error_body_is_summarized_not_dropped() {
        let body = r#"{
          "error": {
            "code": 400,
            "message": "Precondition check failed.",
            "errors": [{ "message": "Precondition check failed.", "reason": "failedPrecondition" }],
            "status": "FAILED_PRECONDITION"
          }
        }"#;
        assert_eq!(
            summarize_error_body(body),
            "Precondition check failed. (failedPrecondition)"
        );

        assert_eq!(
            summarize_error_body("<html><body>Sorry...\n</body></html>"),
            "<html><body>Sorry... </body></html>"
        );
        assert_eq!(summarize_error_body("   "), "empty response body");
        assert!(summarize_error_body(&"x".repeat(1000)).chars().count() <= 301);
    }

    #[test]
    fn all_clients_failed_lists_every_body() {
        let error = ResolveError::AllClientsFailed(vec![
            ClientAttempt {
                client: "ANDROID",
                status: Some(400),
                visitor: "attached",
                error: "HTTP 400 Bad Request: Precondition check failed.".into(),
            },
            ClientAttempt {
                client: "IOS",
                status: Some(502),
                visitor: "none",
                error: "youtube request failed: connection reset".into(),
            },
        ]);
        let rendered = error.to_string();
        assert!(rendered.contains("ANDROID: HTTP 400 Bad Request: Precondition check failed."));
        assert!(rendered.contains("IOS: youtube request failed: connection reset"));
    }

    fn format(itag: u64, mime: &str, bitrate: u64) -> AdaptiveFormat {
        AdaptiveFormat {
            itag,
            mime_type: mime.to_string(),
            bitrate,
            url: Some(format!(
                "https://rr1---sn-abc.googlevideo.com/videoplayback?itag={itag}"
            )),
            content_length: Some("123456".to_string()),
        }
    }

    #[test]
    fn picks_the_best_audio_only_aac_format() {
        let formats = [
            format(137, "video/mp4; codecs=\"avc1.640028\"", 5_000_000),
            format(139, "audio/mp4; codecs=\"mp4a.40.5\"", 48_000),
            format(251, "audio/webm; codecs=\"opus\"", 160_000),
            format(140, "audio/mp4; codecs=\"mp4a.40.2\"", 129_000),
        ];
        let best = select_audio(&formats).expect("audio format");
        assert_eq!(best.itag, 140);
        assert_eq!(best.bitrate, 129_000);
    }

    #[test]
    fn rejects_response_without_direct_audio_url() {
        let mut ciphered = format(140, "audio/mp4; codecs=\"mp4a.40.2\"", 129_000);
        ciphered.url = None;
        assert!(matches!(
            select_audio(&[ciphered]),
            Err(ResolveError::NoAudio)
        ));
    }

    #[test]
    fn unplayable_video_reports_youtube_reason() {
        let json = json!({
            "playabilityStatus": { "status": "ERROR", "reason": "Video unavailable" }
        });
        assert_eq!(
            parse_player_json(&json, "dQw4w9WgXcQ", "ANDROID"),
            Err(ResolveError::Unplayable("Video unavailable".to_string()))
        );
    }

    #[test]
    fn unplayable_prefers_the_error_screen_text() {
        let json = json!({
            "playabilityStatus": {
                "status": "UNPLAYABLE",
                "reason": "The page needs to be reloaded.",
                "errorScreen": {
                    "playerErrorMessageRenderer": {
                        "reason": { "simpleText": "Sign in to confirm you're not a bot" }
                    }
                }
            }
        });
        assert_eq!(
            parse_player_json(&json, "dQw4w9WgXcQ", "TVHTML5_SIMPLY_EMBEDDED_PLAYER"),
            Err(ResolveError::Unplayable(
                "Sign in to confirm you're not a bot".to_string()
            ))
        );
    }

    /// Network check, not part of the default suite:
    /// `cargo test --lib live_ -- --ignored --nocapture`.
    ///
    /// Proves the visitor id is attached, that the first client answering is
    /// not rejected with 400, and that the media CDN serves bytes.
    #[tokio::test]
    #[ignore = "hits youtubei and googlevideo"]
    async fn live_resolve_sends_visitor_and_is_not_rejected() {
        let http = http_client().expect("client");
        let visitor = ensure_visitor(&http, None).await;
        eprintln!(
            "visitor: source={} attached={} preview={}",
            visitor.source.as_str(),
            visitor.attached(),
            visitor.preview()
        );
        assert!(
            visitor.attached(),
            "no visitor id could be obtained: {:?}",
            visitor.source
        );

        let audio = match resolve_audio(&http, "dQw4w9WgXcQ", Some(visitor.value.as_str())).await {
            Ok(audio) => audio,
            Err(ResolveError::AllClientsFailed(attempts)) => {
                for attempt in &attempts {
                    eprintln!(
                        "attempt {} status={:?} visitor={} error={}",
                        attempt.client, attempt.status, attempt.visitor, attempt.error
                    );
                }
                panic!("all clients failed");
            }
            Err(error) => panic!("resolve: {error}"),
        };
        eprintln!(
            "resolved client={} itag={} mime={} bytes={:?}",
            audio.client, audio.itag, audio.mime, audio.content_length
        );
        assert!(audio.mime.starts_with("audio/mp4"), "mime {}", audio.mime);
        assert!(is_allowed_media_url(&audio.url));

        let response = http
            .get(&audio.url)
            .header(reqwest::header::RANGE, "bytes=0-2047")
            .send()
            .await
            .expect("range request");
        eprintln!("media HTTP {}", response.status());
        assert!(
            response.status().is_success(),
            "media returned {}",
            response.status()
        );
    }

    #[test]
    fn visitor_cache_roundtrips_and_rejects_poison() {
        let path = std::env::temp_dir().join(format!(
            "aysha-visitor-test-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or(0)
        ));
        let value = "CgtET1p4NGZsZlVUQSj-7-DXBjIKCgJVUxIEGgAgQA%3D%3D";
        save_visitor(Some(&path), value);
        assert_eq!(load_visitor(Some(&path)).as_deref(), Some(value));

        std::fs::write(&path, r#"{"value":"bad value with spaces"}"#).unwrap();
        assert_eq!(load_visitor(Some(&path)), None);

        std::fs::write(&path, "not json").unwrap();
        assert_eq!(load_visitor(Some(&path)), None);

        let _ = std::fs::remove_file(&path);
        assert_eq!(load_visitor(None), None);
    }

    #[test]
    fn client_bodies_carry_the_cached_visitor_id() {
        let visitor = "CgtET1p4NGZsZlVUQSj-7-DXBjIKCgJVUxIEGgAgQA%3D%3D";
        let body = request_body("dQw4w9WgXcQ", ANDROID, Some(visitor));
        assert_eq!(body["context"]["client"]["visitorData"], visitor);
        assert_eq!(body["context"]["client"]["clientVersion"], "20.10.38");

        let without = request_body("dQw4w9WgXcQ", ANDROID, None);
        assert!(without["context"]["client"]["visitorData"].is_null());
    }

    #[test]
    fn parses_title_and_content_length() {
        let json = json!({
            "playabilityStatus": { "status": "OK" },
            "videoDetails": { "title": "Never Gonna Give You Up" },
            "streamingData": { "adaptiveFormats": [{
                "itag": 140,
                "mimeType": "audio/mp4; codecs=\"mp4a.40.2\"",
                "bitrate": 129000,
                "url": "https://rr1---sn-abc.googlevideo.com/videoplayback?itag=140",
                "contentLength": "4123456"
            }] }
        });
        let audio = parse_player_json(&json, "dQw4w9WgXcQ", "ANDROID").expect("resolved");
        assert_eq!(audio.title.as_deref(), Some("Never Gonna Give You Up"));
        assert_eq!(audio.content_length, Some(4_123_456));
        assert_eq!(audio.itag, 140);
    }

    #[test]
    fn titles_collapse_to_the_same_song() {
        assert_eq!(normalized_title("Tum Hi Ho (Official Video)"), "tum hi ho");
        assert_eq!(normalized_title("Tum Hi Ho | Lyrical"), "tum hi ho");
        assert!(same_song("Tum Hi Ho", "Tum Hi Ho - Lyrical Video"));
        assert!(same_song("Tum Hi Ho (Official Music Video)", "Tum Hi Ho"));
        assert!(!same_song("Tum Hi Ho", "Tum Hi Ho 2"));
        assert!(!same_song("Tum Hi Ho", "Kesariya"));
    }

    #[test]
    fn search_json_collects_video_renderers() {
        let json: Value = serde_json::from_str(
            r#"{"contents":{"twoColumnSearchResultsRenderer":{"primaryContents":{"sectionListRenderer":{"contents":[{"itemSectionRenderer":{"contents":[{"videoRenderer":{"videoId":"dQw4w9WgXcQ","title":{"runs":[{"text":"A Song (Official Video)"}]},"ownerText":{"runs":[{"text":"A Channel"}]}}},{"videoRenderer":{"videoId":"too-short","title":{"runs":[{"text":"Ignored"}]}}}]}}]}}}}}"#,
        )
        .unwrap();
        let hits = parse_search_json(&json, 10);
        assert_eq!(hits.len(), 1);
        let compact: Value = serde_json::from_str(
            r#"{"contents":{"sectionListRenderer":{"contents":[{"itemSectionRenderer":{"contents":[{"compactVideoRenderer":{"videoId":"zeVWTY31Vn8","title":{"runs":[{"text":"Romantic Jukebox"}]},"longBylineText":{"runs":[{"text":"Sony Music India"}]}}}]}}]}}}"#,
        )
        .unwrap();
        let mobile = parse_search_json(&compact, 10);
        assert_eq!(mobile.len(), 1);
        assert_eq!(mobile[0].video_id, "zeVWTY31Vn8");
        assert_eq!(mobile[0].channel, "Sony Music India");
        assert_eq!(hits[0].video_id, "dQw4w9WgXcQ");
        assert_eq!(hits[0].title, "A Song (Official Video)");
        assert_eq!(hits[0].channel, "A Channel");
    }

}
