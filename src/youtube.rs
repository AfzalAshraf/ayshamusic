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
    version: "21.26.364",
    user_agent: "com.google.android.youtube/21.26.364 (Linux; U; Android 11) gzip",
    extra: r#"{"androidSdkVersion":30,"osName":"Android","osVersion":"11"}"#,
};

pub const IOS: InnertubeClient = InnertubeClient {
    name: "IOS",
    version: "21.26.4",
    user_agent: "com.google.ios.youtube/21.26.4 (iPhone16,2; U; CPU iOS 18_3_2 like Mac OS X;)",
    extra: r#"{"deviceMake":"Apple","deviceModel":"iPhone16,2","osName":"iPhone","osVersion":"18.3.2.22D82"}"#,
};

pub const TVHTML5: InnertubeClient = InnertubeClient {
    name: "TVHTML5_SIMPLY_EMBEDDED_PLAYER",
    version: "2.0",
    user_agent: "Mozilla/5.0 (ChromiumStylePlatform) Cobalt/Version",
    extra: "{}",
};

/// Client profiles in the order they are tried.
pub const CLIENTS: [InnertubeClient; 2] = [ANDROID, IOS];

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
            [ANDROID, IOS, TVHTML5]
                .iter()
                .find(|client| client.name.eq_ignore_ascii_case(&needle))
                .map(|client| client.name)
        })?;
    [ANDROID, IOS, TVHTML5].into_iter().find(|client| client.name == canonical)
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

/// One client's failure, including YouTube's own explanation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientAttempt {
    pub client: &'static str,
    pub error: String,
}

impl ClientAttempt {
    pub fn to_json(&self) -> Value {
        json!({ "client": self.client, "error": self.error })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ResolveError {
    InvalidVideoId,
    PreconditionFailed,
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
            Self::PreconditionFailed => write!(f, "YouTube rejected this playback request (HTTP 400: failedPrecondition). Audio has not started."),
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
        .read_timeout(Duration::from_secs(20))
        .pool_max_idle_per_host(2)
        .build()
}

pub fn valid_video_id(id: &str) -> bool {
    id.len() == 11 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
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
pub async fn resolve_audio(http: &Client, video_id: &str) -> Result<ResolvedAudio, ResolveError> {
    resolve_with_clients(http, video_id, &CLIENTS).await
}

/// Resolves with exactly one client, so its real error is reported verbatim.
pub async fn resolve_forced(
    http: &Client,
    video_id: &str,
    profile: InnertubeClient,
) -> Result<ResolvedAudio, ResolveError> {
    resolve_with_clients(http, video_id, &[profile]).await
}

async fn resolve_with_clients(
    http: &Client,
    video_id: &str,
    profiles: &[InnertubeClient],
) -> Result<ResolvedAudio, ResolveError> {
    if !valid_video_id(video_id) {
        return Err(ResolveError::InvalidVideoId);
    }

    let mut attempts: Vec<ClientAttempt> = Vec::new();
    for profile in profiles {
        match resolve_with(http, video_id, *profile).await {
            Ok(audio) => return Ok(audio),
            Err(ResolveError::PreconditionFailed) => return Err(ResolveError::PreconditionFailed),
            Err(error) => attempts.push(ClientAttempt {
                client: profile.name,
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
) -> Result<ResolvedAudio, ResolveError> {
    let visitor = visitor_context();
    let response = player_request(http, video_id, profile, visitor.as_deref())
        .send().await.map_err(|_| ResolveError::Http("connection failed or timed out".into()))?;
    let status = response.status().as_u16();
    let bytes = bounded_body(response, 1024 * 1024).await?;
    parse_wire_response(status, &bytes, video_id, profile.name)
}

fn player_request(http: &Client, video_id: &str, profile: InnertubeClient, visitor: Option<&str>) -> reqwest::RequestBuilder {
    let mut body = request_body(video_id, profile);
    let mut request = http.post(PLAYER_URL).timeout(Duration::from_secs(12))
        .header(reqwest::header::USER_AGENT, profile.user_agent)
        .header("X-YouTube-Client-Name", client_id(profile.name))
        .header("X-YouTube-Client-Version", profile.version);
    if let Some(visitor) = visitor {
        body["context"]["client"]["visitorData"] = json!(visitor);
        request = request.header("X-Goog-Visitor-Id", visitor);
    }
    request.json(&body)
}
async fn bounded_body(response: reqwest::Response, limit: usize) -> Result<Vec<u8>, ResolveError> {
    use futures_util::StreamExt;
    let mut out = Vec::new(); let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| ResolveError::Http("response interrupted".into()))?;
        if out.len() + chunk.len() > limit { return Err(ResolveError::Http("response exceeded size limit".into())); }
        out.extend_from_slice(&chunk);
    }
    Ok(out)
}
fn parse_wire_response(status: u16, bytes: &[u8], video_id: &str, client: &'static str) -> Result<ResolvedAudio, ResolveError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| ResolveError::Http(format!("HTTP {status}: invalid JSON response")))?;
    if status == 400 && (value["error"]["status"] == "FAILED_PRECONDITION" || value["error"]["errors"].as_array().is_some_and(|errors| errors.iter().any(|e| e["reason"] == "failedPrecondition"))) {
        return Err(ResolveError::PreconditionFailed);
    }
    if !(200..300).contains(&status) {
        return Err(ResolveError::Http(format!("HTTP {status}: {}", summarize_error_body(&String::from_utf8_lossy(bytes)))));
    }
    observe_visitor(&value);
    parse_player_json(&value, video_id, client)
}

// Only reuse an anonymous context actually returned by YouTube. Never synthesize,
// log, persist or fetch an extra identity just to try bypassing a refusal.
static VISITOR: std::sync::OnceLock<std::sync::Mutex<Option<(String, std::time::Instant)>>> = std::sync::OnceLock::new();
pub fn observe_visitor(value: &Value) {
    if let Some(visitor) = value["responseContext"]["visitorData"].as_str().filter(|v| (16..=1024).contains(&v.len()) && v.is_ascii() && !v.bytes().any(|b| b.is_ascii_whitespace() || b == b'"' || b == b'\\')) {
        *VISITOR.get_or_init(Default::default).lock().unwrap() = Some((visitor.to_owned(), std::time::Instant::now()));
    }
}
fn visitor_context() -> Option<String> {
    VISITOR.get_or_init(Default::default).lock().unwrap().as_ref().filter(|(_, time)| time.elapsed() < Duration::from_secs(30 * 60)).map(|(v,_)| v.clone())
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

fn request_body(video_id: &str, profile: InnertubeClient) -> Value {
    let extra: Value = serde_json::from_str(profile.extra).unwrap_or_else(|_| json!({}));
    let mut client = json!({
        "clientName": profile.name,
        "clientVersion": profile.version,
        "userAgent": profile.user_agent,
        "hl": "en",
        "gl": "US",
        "utcOffsetMinutes": 0,
    });
    if let (Some(client), Value::Object(extra)) = (client.as_object_mut(), extra) {
        client.extend(extra);
    }
    json!({
        "context": {
            "client": client,
            "user": { "lockedSafetyMode": false },
            "request": { "useSsl": true },
        },
        "videoId": video_id,
        "contentCheckOk": true,
        "racyCheckOk": true,
    })
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

    #[tokio::test]
    async fn player_request_has_consistent_headers_body_and_observed_visitor() {
        let http = http_client().unwrap();
        for profile in CLIENTS {
            let request = player_request(&http, "abcdefghijk", profile, Some("provider_context_fixture")).build().unwrap();
            assert_eq!(request.headers()["user-agent"], profile.user_agent);
            assert_eq!(request.headers()["x-youtube-client-version"], profile.version);
            assert_eq!(request.headers()["x-youtube-client-name"], client_id(profile.name));
            assert_eq!(request.headers()["x-goog-visitor-id"], "provider_context_fixture");
            let value: Value = serde_json::from_slice(request.body().unwrap().as_bytes().unwrap()).unwrap();
            assert_eq!(value["context"]["client"]["userAgent"], profile.user_agent);
            assert_eq!(value["context"]["client"]["clientVersion"], profile.version);
            assert_eq!(value["context"]["client"]["visitorData"], "provider_context_fixture");
            assert_eq!(value["videoId"], "abcdefghijk");
            assert_eq!(value["context"]["client"]["clientName"], profile.name);
        }
        assert!(!CLIENTS.iter().any(|c| c.name == TVHTML5.name));
    }
    #[test]
    fn screenshot_precondition_is_a_terminal_typed_rejection_not_no_audio() {
        let bytes = br#"{"error":{"code":400,"status":"FAILED_PRECONDITION","message":"Precondition check failed.","errors":[{"reason":"failedPrecondition"}]}}"#;
        let error = parse_wire_response(400, bytes, "abcdefghijk", "ANDROID").unwrap_err();
        assert_eq!(error, ResolveError::PreconditionFailed);
        assert!(error.to_string().contains("HTTP 400"));
        assert!(!error.to_string().contains("all clients failed"));
        assert!(parse_wire_response(200, b"invalid", "abcdefghijk", "ANDROID").is_err());
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
                error: "HTTP 400 Bad Request: Precondition check failed.".into(),
            },
            ClientAttempt {
                client: "IOS",
                error: "HTTP 400 Bad Request: Precondition check failed.".into(),
            },
        ]);
        let rendered = error.to_string();
        assert!(rendered.contains("ANDROID: HTTP 400 Bad Request: Precondition check failed."));
        assert!(rendered.contains("IOS: HTTP 400 Bad Request: Precondition check failed."));
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
    /// `cargo test --lib live_ -- --ignored`.
    #[tokio::test]
    #[ignore = "hits youtubei and googlevideo"]
    async fn live_resolve_yields_a_playable_audio_url() {
        let http = http_client().expect("client");
        let audio = resolve_audio(&http, "dQw4w9WgXcQ")
            .await
            .unwrap_or_else(|error| panic!("resolve: {error}"));

        assert!(audio.mime.starts_with("audio/mp4"), "mime {}", audio.mime);
        assert!(is_allowed_media_url(&audio.url));

        let response = http
            .get(&audio.url)
            .header(reqwest::header::RANGE, "bytes=0-1023")
            .send()
            .await
            .expect("range request");
        assert!(
            response.status().is_success(),
            "media returned {}",
            response.status()
        );
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
}
