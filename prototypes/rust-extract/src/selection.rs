//! Selection must rank usable AAC URLs, not metadata-only records.
use rusty_ytdl::VideoFormat;
use serde_json::{json, Value};
use crate::media_origin;

pub struct Selection<'a> {
    pub format: Option<&'a VideoFormat>,
    pub failure_code: &'static str,
    pub counts: Value,
}
fn compatible(format: &VideoFormat) -> bool {
    format.has_audio && !format.has_video && !format.is_live && !format.is_hls && !format.is_dash_mpd
        && format.mime_type.mime.essence_str() == "audio/mp4"
        && format.mime_type.audio_codec.as_deref().is_some_and(|c| c.starts_with("mp4a"))
}
pub fn select(formats: &[VideoFormat]) -> Selection<'_> {
    let (mut aac, mut empty, mut invalid, mut rejected, mut usable) = (0, 0, 0, 0, 0);
    let mut best: Option<&VideoFormat> = None;
    for format in formats.iter().filter(|f| compatible(f)) {
        aac += 1;
        match media_origin::validate(&format.url) {
            Ok(()) => {
                usable += 1;
                if best.map_or(true, |previous| format.bitrate > previous.bitrate) { best = Some(format); }
            }
            Err("media_url_missing") => empty += 1,
            Err("media_url_invalid") => invalid += 1,
            Err(_) => rejected += 1,
        }
    }
    let failure_code = if aac == 0 { "aac_mp4_unavailable" }
        else if empty == aac { "all_aac_urls_missing" }
        else { "no_validated_aac_url" };
    Selection { format: best, failure_code, counts: json!({
        "formats_total":formats.len(), "aac_candidates":aac, "empty_urls":empty,
        "invalid_urls":invalid, "rejected_origins":rejected, "usable_urls":usable,
    }) }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn format(itag: u64, bitrate: u64, url: &str) -> VideoFormat {
        serde_json::from_value(json!({
            "itag":itag, "bitrate":bitrate, "url":url,
            "mimeType":"audio/mp4; codecs=\"mp4a.40.2\"", "hasAudio":true,
            "hasVideo":false, "isLive":false, "isHLS":false, "isDashMPD":false,
        })).unwrap()
    }
    #[test]
    fn empty_best_does_not_shadow_usable_aac() {
        let items = [format(141, 256000, ""), format(140, 128000, "https://x.googlevideo.com/videoplayback")];
        let result = select(&items);
        assert_eq!(result.format.unwrap().itag, 140);
        assert_eq!(result.counts["empty_urls"], 1);
        assert_eq!(result.counts["usable_urls"], 1);
    }
    #[test]
    fn all_empty_is_dependency_failure_not_success() {
        let items = [format(140, 128000, ""), format(141, 256000, "")];
        let result = select(&items);
        assert!(result.format.is_none());
        assert_eq!(result.failure_code, "all_aac_urls_missing");
        assert_eq!(result.counts["empty_urls"], 2);
    }
    #[test]
    fn ranks_only_validated_candidates() {
        let items = [format(141, 256000, "https://evil.test/private?secret=1"),
            format(139, 48000, "https://x.googlevideo.com/a"),
            format(140, 128000, "https://x.googlevideo.com/b")];
        let result = select(&items);
        assert_eq!(result.format.unwrap().itag, 140);
        assert_eq!(result.counts["rejected_origins"], 1);
        assert!(!result.counts.to_string().contains("secret"));
    }
    #[test]
    fn no_codec_or_bad_url_never_becomes_a_pass() {
        let mut video = format(22, 500000, "https://x.googlevideo.com/a");
        video.has_video = true;
        assert_eq!(select(&[video]).failure_code, "aac_mp4_unavailable");
        let items = [format(140, 128000, "not a url")];
        let result = select(&items);
        assert!(result.format.is_none());
        assert_eq!(result.counts["invalid_urls"], 1);
        assert_eq!(result.failure_code, "no_validated_aac_url");
    }
}
