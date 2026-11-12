//! Experimental native extractor. No production integration, cookies or subprocess fallback.
mod media_origin;
mod selection;
use rusty_ytdl::{reqwest, RequestOptions, Video, VideoOptions};
use std::{fs::{self, OpenOptions}, io::Write, path::PathBuf, time::{Duration, Instant}};
use serde_json::json;
const PROBE: usize = 65536;
const MAX_FILE: usize = 64 * 1024 * 1024;
const HELP: &str = "aysha-extract-prototype --video-id ID [--output NEW_FILE.m4a]\nDefault: resolve audio-only AAC/MP4 and validate/read 64 KiB; discard bytes.\nOptional output: exclusive new file, maximum 64 MiB, removed on failure.\nOnly public non-live permitted content; 60s cooperative timeout. No credentials/fallback.\nSigned URLs and raw upstream errors are never printed. Not a production yt-dlp replacement.";
#[derive(Debug, PartialEq)]
struct Options { id: String, output: Option<PathBuf> }
fn valid_id(id: &str) -> bool {
    id.len() == 11 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
fn parse(args: impl IntoIterator<Item=String>) -> Result<Option<Options>, &'static str> {
    let mut args = args.into_iter();
    let (mut id, mut output) = (None, None);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(None),
            "--video-id" if id.is_none() => id = Some(args.next().ok_or("missing_id")?),
            "--output" if output.is_none() => output = Some(PathBuf::from(args.next().ok_or("missing_output")?)),
            _ => return Err("invalid_arguments"),
        }
    }
    let id = id.ok_or("missing_id")?;
    if !valid_id(&id) { return Err("invalid_id"); }
    Ok(Some(Options { id, output }))
}
fn mp4(bytes: &[u8]) -> bool { bytes.len() >= 8 && &bytes[4..8] == b"ftyp" }
// Exclusive output guard: retain only on success. Never overwrite/delete a preexisting file.
struct Output { file: std::fs::File, path: PathBuf, complete: bool }
impl Output {
    fn new(path: &PathBuf) -> Result<Self, &'static str> {
        let file = OpenOptions::new().write(true).create_new(true).open(path).map_err(|_| "output_create_failed")?;
        Ok(Self { file, path:path.clone(), complete:false })
    }
}
impl Drop for Output {
    fn drop(&mut self) { if !self.complete { let _ = fs::remove_file(&self.path); } }
}
fn memory_kib() -> Option<u64> {
    fs::read_to_string("/proc/self/status").ok()?.lines().find_map(|line| {
        line.strip_prefix("VmHWM:")?.split_whitespace().next()?.parse().ok()
    })
}
struct Failure { code: &'static str, format_counts: Option<serde_json::Value> }
impl From<&'static str> for Failure {
    fn from(code: &'static str) -> Self { Self { code, format_counts: None } }
}
async fn extract(options: Options) -> Result<serde_json::Value, Failure> {
    let start = Instant::now();
    let client = reqwest::Client::builder().https_only(true).no_proxy()
        .redirect(reqwest::redirect::Policy::none()).connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(20)).build().map_err(|_| "client_init_failed")?;
    let video = Video::new_with_options(format!("https://www.youtube.com/watch?v={}", options.id), VideoOptions {
        request_options: RequestOptions { client:Some(client.clone()), max_retries:Some(0), ..Default::default() },
        ..Default::default()
    }).map_err(|_| "extract_init_failed")?;
    // Candidate handles YouTube signatures with an embedded Rust JS engine, not Node.
    let info = video.get_info().await.map_err(|_| "extraction_failed")?;
    let selected = selection::select(&info.formats);
    let format = selected.format.ok_or_else(|| Failure {
        code: selected.failure_code, format_counts: Some(selected.counts.clone()),
    })?;
    // Revalidate at the request boundary. Never request rejected/empty candidates.
    media_origin::validate(&format.url)?;
    let resolve_ms = start.elapsed().as_millis();
    let mut request = client.get(&format.url);
    if options.output.is_none() { request = request.header("Range", "bytes=0-65535"); }
    let mut response = request.send().await.map_err(|_| "media_request_failed")?;
    if !response.status().is_success() { return Err("media_http_failed".into()); }
    let mut output = match options.output { Some(ref path) => Some(Output::new(path)?), None => None };
    let limit = if output.is_some() { MAX_FILE } else { PROBE };
    let mut bytes = 0usize;
    let mut prefix = Vec::with_capacity(12);
    let mut first_byte_ms = None;
    while let Some(chunk) = response.chunk().await.map_err(|_| "media_read_failed")? {
        if chunk.is_empty() { continue; }
        first_byte_ms.get_or_insert_with(|| start.elapsed().as_millis());
        let head = (12usize.saturating_sub(prefix.len())).min(chunk.len());
        prefix.extend_from_slice(&chunk[..head]);
        if prefix.len() >= 8 && !mp4(&prefix) { return Err("invalid_mp4_header".into()); }
        if output.is_some() && bytes.saturating_add(chunk.len()) > limit { return Err("output_size_limit".into()); }
        let take = chunk.len().min(limit - bytes);
        if let Some(ref mut out) = output { out.file.write_all(&chunk[..take]).map_err(|_| "output_write_failed")?; }
        bytes += take;
        if output.is_none() && bytes == PROBE { break; }
    }
    if !mp4(&prefix) || bytes == 0 { return Err("empty_or_invalid_media".into()); }
    if output.is_none() && bytes != PROBE { return Err("short_probe".into()); }
    if let Some(ref mut out) = output {
        out.file.sync_all().map_err(|_| "output_sync_failed")?;
        out.complete = true;
    }
    Ok(json!({"status":"pass", "candidate":"rusty_ytdl-0.7.4", "mode":if output.is_some(){"file"}else{"probe"},
        "resolve_ms":resolve_ms, "first_byte_ms":first_byte_ms, "total_ms":start.elapsed().as_millis(),
        "bytes":bytes, "itag":format.itag, "mime":"audio/mp4", "peak_rss_kib":memory_kib(),
        "format_counts":selected.counts, "external_tools":false, "production_ready":false}))
}
#[tokio::main(flavor="current_thread")]
async fn main() {
    let options = match parse(std::env::args().skip(1)) {
        Ok(None) => { println!("{HELP}"); return; },
        Ok(Some(options)) => options,
        Err(code) => { println!("{}", json!({"status":"fail","code":code})); std::process::exit(2); }
    };
    let result = tokio::time::timeout(Duration::from_secs(60), extract(options)).await;
    match result {
        Ok(Ok(metrics)) => println!("{metrics}"),
        error => {
            let failure = match error { Ok(Err(failure)) => failure, _ => "deadline_exceeded".into() };
            println!("{}", json!({"status":"fail","code":failure.code,"format_counts":failure.format_counts,
                "peak_rss_kib":memory_kib(),"production_ready":false}));
            std::process::exit(1);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn ids_reject_urls_and_options() {
        assert!(valid_id("Jm7Satp7Zqk"));
        for id in ["", "--exec=evil", "https://youtube.com/watch?v=x", "é1234567890", "abcdefghij/"] { assert!(!valid_id(id)); }
    }
    #[test] fn parse_is_strict() {
        assert!(parse(["--video-id","Jm7Satp7Zqk"].map(String::from)).unwrap().is_some());
        assert!(parse(["--video-id","bad"].map(String::from)).is_err());
        assert!(parse(["--unknown"].map(String::from)).is_err());
        assert!(parse(["--video-id","Jm7Satp7Zqk","--video-id","Jm7Satp7Zqk"].map(String::from)).is_err());
    }
    #[test] fn validates_mp4_prefix() { assert!(mp4(b"\0\0\0\x20ftypM4A ")); assert!(!mp4(b"<html>error")); }
    #[test] fn output_guard_cleans_only_its_own_file() {
        let path = std::env::temp_dir().join(format!("aysha-extract-test-{}",std::process::id()));
        { let _out = Output::new(&path).unwrap(); assert!(Output::new(&path).is_err()); }
        assert!(!path.exists());
    }
}
