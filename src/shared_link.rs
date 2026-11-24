//! Bounded, offline parsing of a single Android-shared YouTube video link.
//! The input is never fetched or passed through to a network request.
use crate::youtube::valid_video_id;

pub fn video_id(text: &str) -> Option<String> {
    if text.len() > 4096 { return None; }
    let lower = text.to_ascii_lowercase();
    let mut matches = lower.match_indices("https://");
    let (start, _) = matches.next()?;
    if matches.next().is_some() { return None; }
    let tail = &text[start..];
    let end = tail.find(|c: char| c.is_whitespace() || c == '<' || c == '>').unwrap_or(tail.len());
    let link = tail[..end].trim_end_matches(['.', ',', ')', ']', '!', '?']);
    let authority = link.get(8..)?.split(['/', '?', '#']).next()?;
    // Reject credentials and explicit ports, including the default HTTPS port.
    if authority.is_empty() || authority.chars().any(|c| matches!(c, '@' | ':' | '\\' | '%')) { return None; }
    let url = reqwest::Url::parse(link).ok()?;
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() || url.port().is_some() || url.fragment().is_some() { return None; }
    let host = url.host_str()?;
    let path = url.path();
    let segments: Vec<&str> = url.path_segments()?.filter(|s| !s.is_empty()).collect();
    let candidate = match host {
        "youtu.be" if segments.len() == 1 => Some(segments[0].to_owned()),
        "youtube.com" | "www.youtube.com" | "m.youtube.com" | "music.youtube.com" => {
            if path == "/watch" {
                let mut ids = url.query_pairs().filter(|(key,_)| key == "v").map(|(_, value)| value.into_owned());
                let id = ids.next();
                if ids.next().is_some() { None } else { id }
            } else if segments.len() == 2 && matches!(segments[0], "shorts" | "live" | "embed") {
                Some(segments[1].to_owned())
            } else { None }
        },
        _ => None,
    }?;
    if valid_video_id(&candidate) { Some(candidate) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn accepts_one_supported_video_without_network() {
        for text in ["https://youtu.be/abcdefghijk?si=abc", "Watch https://www.youtube.com/watch?v=abcdefghijk", "https://music.youtube.com/watch?v=abcdefghijk&list=test", "https://youtube.com/shorts/abcdefghijk", "https://m.youtube.com/live/abcdefghijk", "HTTPS://YOUTU.BE/abcdefghijk!", "https://youtube.com/embed/abcdefghijk"] {
            assert_eq!(video_id(text).as_deref(), Some("abcdefghijk"), "{text}");
        }
    }
    #[test] fn rejects_untrusted_ambiguous_or_excessive_input() {
        for text in ["https://youtube.com.evil.test/watch?v=abcdefghijk", "https://youtube.com@evil.test/watch?v=abcdefghijk", "http://youtu.be/abcdefghijk", "https://youtube.com/watch?v=abcdefghijk&v=another0001", "https://youtube.com/playlist?list=x", "https://youtu.be/short", "https://youtu.be/abcdefghijk https://youtu.be/nexttrack01", "https://youtube.com:443/watch?v=abcdefghijk", "https://youtube.com:444/watch?v=abcdefghijk", "https://youtu.be/abcdefghijk#fragment", "https://youtube.com\\@evil.test/watch?v=abcdefghijk", "javascript:alert(1)"] {
            assert_eq!(video_id(text), None, "{text}");
        }
        assert_eq!(video_id(&"x".repeat(5000)), None);
    }
}
