//! Fixed diagnostic codes only. Never return/log the URL, hostname, path or query.
use rusty_ytdl::reqwest::Url;

pub fn validate(value: &str) -> Result<(), &'static str> {
    if value.is_empty() { return Err("media_url_missing"); }
    let url = Url::parse(value).map_err(|_| "media_url_invalid")?;
    if url.scheme() != "https" { return Err("media_scheme_rejected"); }
    if !url.username().is_empty() || url.password().is_some() { return Err("media_credentials_rejected"); }
    if url.port().is_some() { return Err("media_port_rejected"); }
    let host = url.host_str().ok_or("media_host_missing")?;
    if !host.ends_with(".googlevideo.com") { return Err("media_host_rejected"); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_and_malformed_are_not_reported_as_untrusted_hosts() {
        assert_eq!(validate(""), Err("media_url_missing"));
        assert_eq!(validate("not a url"), Err("media_url_invalid"));
        assert_eq!(validate("/videoplayback?secret=redacted"), Err("media_url_invalid"));
    }
    #[test]
    fn keeps_origin_guard_closed() {
        assert!(validate("https://rr1---sn-test.googlevideo.com/videoplayback?secret=redacted").is_ok());
        for (url, code) in [
            ("http://x.googlevideo.com/", "media_scheme_rejected"),
            ("https://googlevideo.com.evil.test/", "media_host_rejected"),
            ("https://evilgooglevideo.com/", "media_host_rejected"),
            ("https://googlevideo.com/", "media_host_rejected"),
            ("https://127.0.0.1/", "media_host_rejected"),
            ("https://[::1]/", "media_host_rejected"),
            ("https://u:secret@x.googlevideo.com/", "media_credentials_rejected"),
            ("https://x.googlevideo.com:444/", "media_port_rejected"),
        ] {
            assert_eq!(validate(url), Err(code));
        }
    }
}
