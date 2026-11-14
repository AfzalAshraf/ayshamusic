use ayshamusic::youtube;
use std::time::{Duration, Instant};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore]
async fn public_fallback_streams_a_real_song() {
    let http = youtube::http_client().unwrap();
    let visitor = youtube::ensure_visitor(&http, None).await;
    let id = "kJQP7kiw5Fk";
    let audio = youtube::resolve_audio(&http, id, Some(visitor.value.as_str()))
        .await
        .unwrap_or_else(|error| panic!("resolve: {error}"));
    eprintln!(
        "resolved client={} itag={} mime={} title={:?} len={:?}\n  url={}",
        audio.client, audio.itag, audio.mime, audio.title, audio.content_length, audio.url
    );
    assert_eq!(audio.client, "INVIDIOUS");
    assert!(audio.url.contains("/latest_version?") && audio.url.contains("local=true"));

    let media = youtube::media_client().unwrap();
    let range = media
        .get(&audio.url)
        .header(reqwest::header::USER_AGENT, youtube::ANDROID.user_agent)
        .header(reqwest::header::RANGE, "bytes=0-2047")
        .send()
        .await
        .expect("range");
    let status = range.status();
    let kind = range
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let ctype = range
        .headers()
        .get(reqwest::header::CONTENT_RANGE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let head = range.bytes().await.unwrap();
    eprintln!("range {status} {kind} {ctype} bytes={}", head.len());
    assert_eq!(status, reqwest::StatusCode::PARTIAL_CONTENT);
    assert!(kind.starts_with("audio/"), "{kind}");

    let started = Instant::now();
    let mut response = media
        .get(&audio.url)
        .header(reqwest::header::USER_AGENT, youtube::ANDROID.user_agent)
        .send()
        .await
        .expect("download");
    assert!(response.status().is_success(), "download {}", response.status());
    let mut bytes: Vec<u8> = Vec::new();
    while let Some(chunk) = response.chunk().await.expect("chunk") {
        bytes.extend_from_slice(&chunk);
        if started.elapsed() > Duration::from_secs(240) {
            panic!("download exceeded 240s at {} bytes", bytes.len());
        }
    }
    eprintln!(
        "downloaded {} bytes in {:?} (declared {:?})",
        bytes.len(),
        started.elapsed(),
        audio.content_length
    );
    if let Some(len) = audio.content_length {
        assert_eq!(bytes.len() as u64, len, "truncated download");
    }
    let path = std::env::temp_dir().join("aysha_public_track.bin");
    std::fs::write(&path, &bytes).unwrap();
    eprintln!("wrote {}", path.display());

    // The app's own decoder must accept these bytes, not just ffprobe.
    use rodio::Source;
    let decoder = rodio::Decoder::new(std::io::Cursor::new(bytes)).expect("decoder");
    let rate = decoder.sample_rate();
    let decoded: Vec<f32> = decoder.take(48_000).collect();
    let non_silent = decoded.iter().any(|s| s.abs() > 0.001);
    eprintln!("rodio decoded {} samples rate={} non-silent={}", decoded.len(), rate, non_silent);
    assert!(decoded.len() > 4_000 && non_silent);
}

#[tokio::test]
#[ignore]
async fn api_play_and_selftest_survive_bot_screening() {
    use axum::body::Body;
    use http_body_util::BodyExt;
    use std::sync::Arc;
    use tower::ServiceExt;

    async fn call(uri: &str, json: Option<serde_json::Value>) -> (u16, serde_json::Value) {
        let builder = axum::http::Request::builder()
            .method(if json.is_some() { "POST" } else { "GET" })
            .uri(uri.to_string())
            .header("content-type", "application/json");
        let request = builder
            .body(match json {
                Some(body) => Body::from(body.to_string()),
                None => Body::empty(),
            })
            .unwrap();
        let response = ayshamusic::router(Arc::new(None))
            .oneshot(request)
            .await
            .unwrap();
        let status = response.status().as_u16();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        (status, value)
    }

    let (status, body) = call("/api/play", Some(serde_json::json!({"video_id": "kJQP7kiw5Fk"}))).await;
    eprintln!("play {status} {}", serde_json::to_string_pretty(&body).unwrap());
    assert_eq!(status, 200);
    assert_eq!(body["client"], "INVIDIOUS");
    assert!(body["duration_secs"].as_f64().unwrap() > 60.0);

    let (status, body) = call("/api/selftest?video_id=kJQP7kiw5Fk", None).await;
    eprintln!("selftest {status} ok={} client={} media={} bytes={}",
        body["ok"], body["client"], body["media_status"], body["bytes_fetched"]);
    assert_eq!(status, 200);
    assert_eq!(body["ok"], true);
    assert_eq!(body["client"], "INVIDIOUS");
    assert_eq!(body["media_status"], 206);
    assert!(body["bytes_fetched"].as_u64().unwrap() >= 1024);
}
