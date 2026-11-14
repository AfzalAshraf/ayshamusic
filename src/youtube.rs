//! Pure-Rust YouTube audio URL resolution via the InnerTube `player` endpoint.
//!
//! No `yt-dlp`, no JS runtime: we ask YouTube's mobile/TV player APIs, which
//! return direct `googlevideo` URLs for audio-only AAC formats. If one client
//! profile starts getting 403s (YouTube rotates requirements), the next profile
//! is tried before reporting failure. Every failure keeps YouTube's own error
//! body so callers can see *why* each client was rejected.

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

/// Client profiles in the order they are tried.
pub const CLIENTS: [InnertubeClient; 3] = [ANDROID, IOS, TVHTML5];

/// The names accepted by `/api/play`'s optional `client` field.
pub const CLIENT_ALIASES: [(&str, &str); 3] = [
    ("android", "ANDROID"),
    ("ios", "IOS"),
    ("tv", "TVHTML5_SIMPLY_EMBEDDED_PLAYER"),
];

/// Resolves `"android" | "ios" | "tv"` (any case) to a client profile.
pub fn client_by_name(name: &str) -> Option<InnertubeClient> {
    let needle = name.trim().to_ascii_lowercase();
    let canonical = CLIENT_ALIASES
        .iter()
        .find(|(alias, _)| *alias == needle)
        .map(|(_, canonical)| *canonical)
        .or_else(|| {
            CLIENTS
                .iter()
                .find(|client| client.name.eq_ignore_ascii_case(&needle))
                .map(|client| client.name)
        })?;
    CLIENTS.iter().copied().find(|client| client.name == canonical)
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
    Client::builder()
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(12))
        .pool_max_idle_per_host(2)
        .build()
}

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
            Err(error) => attempts.push(ClientAttempt {
                client: profile.name,
                status,
                visitor: visitor_label,
                error: error.to_string(),
            }),
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
    let body = request_body(video_id, profile, visitor);
    let mut request = http
        .post(PLAYER_URL)
        .header(reqwest::header::USER_AGENT, profile.user_agent)
        .header("X-YouTube-Client-Name", client_id(profile.name))
        .header("X-YouTube-Client-Version", profile.version);
    if let Some(visitor) = visitor {
        request = request.header("X-Goog-Visitor-Id", visitor);
    }
    let response = match request.json(&body).send().await {
        Ok(response) => response,
        Err(error) => return (None, Err(ResolveError::Http(error.to_string()))),
    };

    let status = response.status();
    if !status.is_success() {
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
    let json: Value = match response.json().await {
        Ok(json) => json,
        Err(error) => return (Some(status.as_u16()), Err(ResolveError::Http(error.to_string()))),
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
        assert_eq!(client_alias_list(), "android, ios, tv");
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
