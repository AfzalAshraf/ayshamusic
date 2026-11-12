//! Conservative inert watch-page reader, not an HTML/JavaScript interpreter.
//! Accepts only literal JSON assignments at the beginning of inline script text.
//! Unsupported wrappers/markup fail closed. No raw values implement Debug or Display.
use serde::Deserialize;
#[cfg(test)]
use serde_json::Value;

pub const PAGE_MAX: usize = 2 * 1024 * 1024;
const JSON_MAX: usize = 1024 * 1024;
const VISITOR_MAX: usize = 4096;

pub struct Visitor(String);
impl Visitor {
    pub fn as_header(&self) -> &str { &self.0 }
}
#[derive(Default, Deserialize)]
struct Config {
    #[serde(rename="VISITOR_DATA")] visitor: Option<String>,
    #[serde(rename="INNERTUBE_CONTEXT")] context: Option<Context>,
    #[serde(rename="LOGGED_IN")] logged_in: Option<bool>,
}
#[derive(Deserialize)]
struct Context { client: Option<ConfigClient> }
#[derive(Deserialize)]
struct ConfigClient { #[serde(rename="visitorData")] visitor: Option<String> }
#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
struct InitialPlayer {
    playability_status: Option<super::Playability>,
    video_details: Option<super::Details>,
    response_context: Option<ResponseContext>,
}
#[derive(Deserialize)]
struct ResponseContext { #[serde(rename="visitorData")] visitor: Option<String> }

fn remember(found: &mut Option<String>, value: Option<String>) -> Result<(), &'static str> {
    if let Some(value) = value {
        // Visitor data is an opaque provider value. Never generate/transform/decode it.
        // Restrict header alphabet and length; reject CRLF/control/Unicode injection.
        if value.is_empty() || value.len() > VISITOR_MAX || !value.bytes().all(|b|
            b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'%' | b'=' | b'.')) {
            return Err("watch_visitor_invalid");
        }
        if found.as_ref().is_some_and(|old| old != &value) {
            return Err("watch_visitor_ambiguous");
        }
        *found = Some(value);
    }
    Ok(())
}
fn eq(a: &[u8], b: &[u8]) -> bool { a.eq_ignore_ascii_case(b) }
fn find_ci(data: &[u8], needle: &[u8]) -> Option<usize> {
    data.windows(needle.len()).position(|w| eq(w, needle))
}
fn trim(mut s: &[u8]) -> &[u8] {
    while s.first().is_some_and(u8::is_ascii_whitespace) { s = &s[1..]; }
    s
}
// Tags are scanned quote-aware so an attribute containing '>' cannot inject markup.
fn tag_end(data: &[u8]) -> Option<usize> {
    let mut quote = None;
    for (i, &b) in data.iter().enumerate() {
        if let Some(q) = quote { if b == q { quote = None; } }
        else if b == b'\'' || b == b'"' { quote = Some(b); }
        else if b == b'>' { return Some(i); }
    }
    None
}
fn attributes_safe(mut data: &[u8]) -> bool {
    loop {
        data = trim(data);
        if data.is_empty() { return true; }
        let n = data.iter().take_while(|b| b.is_ascii_alphanumeric() || **b == b'-').count();
        if n == 0 { return false; }
        let name = &data[..n];
        data = trim(&data[n..]);
        // Only common inline-script attributes are accepted. External/module scripts
        // and unknown browser parsing modes are not sources of trusted literals.
        if !eq(name,b"nonce") && !eq(name,b"type") { return false; }
        if data.first() != Some(&b'=') { return false; }
        data = trim(&data[1..]);
        let Some(&q) = data.first() else { return false };
        if q != b'\'' && q != b'"' { return false; }
        let Some(end) = data[1..].iter().position(|b| *b == q) else { return false };
        let value = &data[1..end+1];
        if eq(name,b"type") && !eq(value,b"text/javascript") && !eq(value,b"application/javascript") {
            return false;
        }
        data = &data[end+2..];
    }
}
// Locate a real raw-text closing delimiter, not </scriptSomething> inside a string.
fn raw_close(data: &[u8], name: &[u8]) -> Option<usize> {
    let mut needle = b"</".to_vec(); needle.extend_from_slice(name);
    let mut pos = 0;
    while let Some(n) = find_ci(&data[pos..], &needle) {
        let at = pos+n;
        if data.get(at+needle.len()).is_some_and(|b| b.is_ascii_whitespace() || *b == b'>') {
            return Some(at);
        }
        pos = at+needle.len();
    }
    None
}
fn json_end(data: &[u8]) -> Result<usize, &'static str> {
    if data.first() != Some(&b'{') { return Err("watch_json_object_expected"); }
    let mut depth = 0usize;
    let mut string = false;
    let mut escaped = false;
    for (i, &b) in data.iter().take(JSON_MAX+1).enumerate() {
        if i == JSON_MAX { return Err("watch_json_size_limit"); }
        if string {
            if escaped { escaped = false; }
            else if b == b'\\' { escaped = true; }
            else if b == b'"' { string = false; }
        } else {
            match b {
                b'"' => string = true,
                b'{' | b'[' => { depth += 1; if depth > 64 { return Err("watch_json_depth_limit"); } }
                b'}' | b']' => {
                    depth = depth.checked_sub(1).ok_or("watch_json_unbalanced")?;
                    if depth == 0 { return Ok(i+1); }
                }
                _ => {}
            }
        }
    }
    Err("watch_json_unterminated")
}

// Consume only two unrelated display-setting calls. No expression is evaluated,
// and no dynamic visitor/auth/config key is skipped. Names/expressions are fixed.
fn viewport_call(tail: &[u8]) -> Option<&[u8]> {
    for (key, expressions) in [
        (b"initialBodyClientWidth".as_slice(), [b"document.body.clientWidth".as_slice(), b"window.innerWidth"]),
        (b"initialBodyClientHeight".as_slice(), [b"document.body.clientHeight".as_slice(), b"window.innerHeight"]),
    ] {
        for quote in [b'\'',b'"'] {
            if tail.first() != Some(&quote) { continue; }
            let Some(after_key) = tail[1..].strip_prefix(key).and_then(|s| s.strip_prefix(&[quote])) else { continue };
            let Some(value) = trim(after_key).strip_prefix(b",") else { continue };
            for expression in expressions {
                if let Some(after_value) = trim(value).strip_prefix(expression) {
                    if let Some(after_close) = trim(after_value).strip_prefix(b")") {
                        return Some(after_close);
                    }
                }
            }
        }
    }
    None
}
fn statement_end(after: &[u8], config: bool) -> Result<&[u8], &'static str> {
    let after = trim(after);
    if after.is_empty() { return Ok(after); } // legal end-of-script semicolon omission
    after.strip_prefix(b";").ok_or(if config { "watch_config_terminator_invalid" } else { "watch_player_terminator_invalid" })
}

pub fn parse(page: &[u8], id: &str) -> Result<Visitor, &'static str> {
    if page.len() > PAGE_MAX { return Err("watch_size_limit"); }
    // Deliberately conservative challenge/consent detection, not a general detector.
    // False positives stop the probe; no consent automation or challenge response.
    for marker in [b"consent.youtube.com/save".as_slice(), b"g-recaptcha", b"/sorry/index", b"confirm you're not a bot", b"confirm you\xe2\x80\x99re not a bot"] {
        if find_ci(page, marker).is_some() { return Err("watch_restriction_detected"); }
    }
    let mut visitor = None;
    let mut initial_seen = false;
    let mut template_depth = 0usize;
    let mut rest = page;
    while let Some(start) = rest.iter().position(|b| *b == b'<') {
        rest = &rest[start..];
        if rest.starts_with(b"<!--") {
            let end = find_ci(&rest[4..], b"-->").ok_or("watch_markup_unsupported")?;
            rest = &rest[4+end+3..]; continue;
        }
        let end = tag_end(rest).ok_or("watch_markup_unsupported")?;
        let mut tag = &rest[1..end];
        let closing = tag.first() == Some(&b'/');
        if closing { tag = &tag[1..]; }
        let n = tag.iter().take_while(|b| b.is_ascii_alphanumeric()).count();
        let name = &tag[..n];
        let attrs = &tag[n..];
        rest = &rest[end+1..];
        if eq(name,b"template") {
            if closing { template_depth = template_depth.saturating_sub(1); }
            else { template_depth += 1; }
            continue;
        }
        if closing { continue; }
        if eq(name,b"plaintext") { break; }
        let raw = [b"script".as_slice(),b"style",b"textarea",b"title",b"xmp",b"iframe",b"noembed",b"noframes",b"noscript"];
        if !raw.iter().any(|kind| eq(name,kind)) { continue; }
        let close = raw_close(rest,name).ok_or("watch_markup_unsupported")?;
        let script = &rest[..close];
        rest = &rest[close..];
        if !eq(name,b"script") || template_depth != 0 || !attributes_safe(attrs) { continue; }
        parse_script(script,id,&mut visitor,&mut initial_seen)?;
    }
    if !initial_seen { return Err("watch_player_missing"); }
    visitor.map(Visitor).ok_or("watch_visitor_missing")
}
fn parse_script(mut script: &[u8], id: &str, visitor: &mut Option<String>, initial_seen: &mut bool) -> Result<(), &'static str> {
    // Intentionally don't search inside arbitrary JS (strings/comments/functions).
    // Only recognized literals at script start or after another accepted literal.
    loop {
        script = trim(script);
        let (tail, config) = if let Some(tail) = script.strip_prefix(b"ytcfg.set(") {
            (trim(tail),true)
        } else if let Some(tail) = script.strip_prefix(b"var ytInitialPlayerResponse =") {
            (trim(tail),false)
        } else if let Some(tail) = script.strip_prefix(b"ytInitialPlayerResponse =") {
            (trim(tail),false)
        } else { return Ok(()) };
        if config && tail.first() != Some(&b'{') {
            if let Some(after) = viewport_call(tail) {
                script = statement_end(after,true)?;
                continue;
            }
            return Err("watch_config_nonliteral_unsupported");
        }
        if !config && tail.first() != Some(&b'{') { return Err("watch_player_nonliteral_unsupported"); }
        let end = json_end(tail)?;
        let after = trim(&tail[end..]);
        let after = if config {
            trim(after.strip_prefix(b")").ok_or("watch_config_closer_missing")?)
        } else { after };
        let next = statement_end(after,config)?;
        if config {
            let cfg: Config = serde_json::from_slice(&tail[..end]).map_err(|_| "watch_config_invalid")?;
            if cfg.logged_in == Some(true) { return Err("watch_authenticated_unsupported"); }
            remember(visitor,cfg.visitor)?;
            remember(visitor,cfg.context.and_then(|c| c.client).and_then(|c| c.visitor))?;
        } else {
            if *initial_seen { return Err("watch_player_ambiguous"); }
            let pr: InitialPlayer = serde_json::from_slice(&tail[..end]).map_err(|_| "watch_player_invalid")?;
            // Do not carry watch-page formats into the media path, and honor its
            // playability restrictions before making a second provider request.
            let player = super::Player { playability_status: pr.playability_status,
                video_details: pr.video_details, streaming_data: None };
            super::playability(&player)?;
            super::identity(&player,id)?;
            remember(visitor,pr.response_context.and_then(|r| r.visitor))?;
            *initial_seen = true;
        }
        script = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const ID: &str = "Jm7Satp7Zqk";
    fn page(cfg: Value, pr: Value) -> Vec<u8> {
        format!("<html><script nonce=\"x\">ytcfg.set({cfg});</script><script>var ytInitialPlayerResponse = {pr};</script></html>").into_bytes()
    }
    fn player() -> Value { serde_json::json!({"playabilityStatus":{"status":"OK"},"videoDetails":{"videoId":ID}}) }
    #[test] fn only_literal_opaque_value_and_matching_identity_are_accepted() {
        let data = page(serde_json::json!({"VISITOR_DATA":"anonymous_123%3D","ignored":{"x":"};\\\"{"}}),player());
        assert_eq!(parse(&data,ID).unwrap().as_header(),"anonymous_123%3D");
        assert_eq!(parse(&data,"aaaaaaaaaaa").err(),Some("video_identity_mismatch"));
    }
    #[test] fn absent_conflicting_and_duplicate_player_data_stop() {
        assert_eq!(parse(&page(serde_json::json!({}),player()),ID).err(),Some("watch_visitor_missing"));
        assert_eq!(parse(&page(serde_json::json!({"VISITOR_DATA":"one","INNERTUBE_CONTEXT":{"client":{"visitorData":"two"}}}),player()),ID).err(),Some("watch_visitor_ambiguous"));
        assert_eq!(parse(b"<script>ytcfg.set({\"VISITOR_DATA\":\"one\"});</script>",ID).err(),Some("watch_player_missing"));
        let mut data = page(serde_json::json!({"VISITOR_DATA":"one"}),player());
        data.extend_from_slice(format!("<script>var ytInitialPlayerResponse = {};</script>",player()).as_bytes());
        assert_eq!(parse(&data,ID).err(),Some("watch_player_ambiguous"));
    }
    #[test] fn response_and_context_paths_are_supported_without_generating_values() {
        let mut pr = player(); pr["responseContext"] = serde_json::json!({"visitorData":"same"});
        let data = page(serde_json::json!({"INNERTUBE_CONTEXT":{"client":{"visitorData":"same"}}}),pr);
        assert_eq!(parse(&data,ID).unwrap().as_header(),"same");
    }
    #[test] fn restriction_auth_live_and_consent_are_never_bypassed() {
        let mut pr = player(); pr["playabilityStatus"] = serde_json::json!({"status":"LOGIN_REQUIRED"});
        assert_eq!(parse(&page(serde_json::json!({"VISITOR_DATA":"x"}),pr),ID).err(),Some("player_login_required"));
        let mut pr = player(); pr["videoDetails"]["isLiveContent"] = serde_json::json!(true);
        assert_eq!(parse(&page(serde_json::json!({"VISITOR_DATA":"x"}),pr),ID).err(),Some("live_unsupported"));
        assert_eq!(parse(&page(serde_json::json!({"LOGGED_IN":true}),player()),ID).err(),Some("watch_authenticated_unsupported"));
        assert_eq!(parse(b"<form action='https://consent.youtube.com/save'>",ID).err(),Some("watch_restriction_detected"));
        assert_eq!(parse(b"confirm you're not a bot",ID).err(),Some("watch_restriction_detected"));
    }
    #[test] fn invalid_headers_values_and_bounds_fail_with_fixed_codes() {
        for value in ["", "x\r\nCookie: secret", "\u{2665}", "x y"] {
            let err = parse(&page(serde_json::json!({"VISITOR_DATA":value}),player()),ID).err();
            assert_eq!(err,Some("watch_visitor_invalid"));
            assert!(!err.unwrap().contains(value) || value.is_empty());
        }
        assert_eq!(parse(&page(serde_json::json!({"VISITOR_DATA":"x".repeat(VISITOR_MAX+1)}),player()),ID).err(),Some("watch_visitor_invalid"));
        assert_eq!(parse(&vec![b'x';PAGE_MAX+1],ID).err(),Some("watch_size_limit"));
        assert_eq!(json_end(&[b"{\"x\":\"".as_slice(), &vec![b'x';JSON_MAX]].concat()),Err("watch_json_size_limit"));
        assert_eq!(json_end(&[vec![b'{';65],vec![b'}';65]].concat()),Err("watch_json_depth_limit"));
    }
    #[test] fn inert_strings_comments_attributes_and_rawtext_are_not_sources() {
        let fake = "<script>ytcfg.set({\"VISITOR_DATA\":\"evil\"});</script>";
        for prefix in [format!("<!--{fake}-->"),format!("<textarea>{fake}</textarea>"),
            format!("<template>{fake}</template>"),format!("<style>{fake}</style>"),
            format!("<div title='{fake}'>text</div>"),
            "<script>const s = 'ytcfg.set({\"VISITOR_DATA\":\"evil\"});';</script>".into(),
            "<script>/* ytcfg.set({\"VISITOR_DATA\":\"evil\"}); */</script>".into(),
            "<script src='x'>ytcfg.set({\"VISITOR_DATA\":\"evil\"});</script>".into()] {
            let mut data = prefix.into_bytes(); data.extend(page(serde_json::json!({"VISITOR_DATA":"good"}),player()));
            assert_eq!(parse(&data,ID).unwrap().as_header(),"good");
        }
    }
    #[test] fn viewport_key_value_calls_do_not_block_real_literal_config() {
        for viewport in ["ytcfg.set('initialBodyClientWidth', document.body.clientWidth);",
                         "ytcfg.set(\"initialBodyClientHeight\", window.innerHeight ) ;",
                         "ytcfg.set('initialBodyClientHeight', document.body.clientHeight)"] {
            let mut data = format!("<script>{viewport}</script>").into_bytes();
            data.extend(page(serde_json::json!({"VISITOR_DATA":"good"}),player()));
            assert_eq!(parse(&data,ID).unwrap().as_header(),"good");
        }
        let data = format!("<script>ytcfg.set('initialBodyClientWidth', document.body.clientWidth);ytcfg.set({{\"VISITOR_DATA\":\"good\"}});</script><script>var ytInitialPlayerResponse = {};</script>",player());
        assert_eq!(parse(data.as_bytes(),ID).unwrap().as_header(),"good");
    }
    #[test] fn only_fixed_viewport_expressions_can_be_ignored() {
        for script in ["ytcfg.set('VISITOR_DATA', arbitrary());", "ytcfg.set('LOGGED_IN', true);",
                       "ytcfg.set('initialBodyClientWidth', steal());",
                       "ytcfg.set('initialBodyClientWidth', document.body.clientWidth.valueOf());",
                       "ytcfg.set('initialBodyClientWidth', window.innerWidth) + evil();"] {
            let mut data = format!("<script>{script}</script>").into_bytes();
            data.extend(page(serde_json::json!({"VISITOR_DATA":"good"}),player()));
            assert!(parse(&data,ID).is_err());
        }
    }
    #[test] fn whitespace_and_end_of_script_terminators_are_supported_without_expressions() {
        for cfg in ["ytcfg.set({\"VISITOR_DATA\":\"good\"})", "ytcfg.set({\"VISITOR_DATA\":\"good\"} ) ; "] {
            for suffix in [";", "", " \n"] {
                let data = format!("<script>{cfg}</script><script>var ytInitialPlayerResponse = {}{suffix}</script>",player());
                assert_eq!(parse(data.as_bytes(),ID).unwrap().as_header(),"good");
            }
        }
    }
    #[test] fn structural_errors_are_specific_and_never_echo_page_data() {
        for (script,code) in [
            ("ytcfg.set('PRIVATE_UNKNOWN', secret)","watch_config_nonliteral_unsupported"),
            ("var ytInitialPlayerResponse = secret;","watch_player_nonliteral_unsupported"),
            ("ytcfg.set({\"VISITOR_DATA\":\"secret\"};","watch_config_closer_missing"),
            ("ytcfg.set({\"VISITOR_DATA\":\"secret\"}) + evil();","watch_config_terminator_invalid"),
            ("var ytInitialPlayerResponse = {} + secret;","watch_player_terminator_invalid"),
            ("ytcfg.set({\"VISITOR_DATA\":\"secret","watch_json_unterminated"),
        ] {
            let data = format!("<script>{script}</script>");
            let err = parse(data.as_bytes(),ID).err().unwrap();
            assert_eq!(err,code);
            assert!(!err.contains("secret"));
            assert!(!err.contains("PRIVATE"));
        }
    }
    #[test] fn malformed_literals_duplicate_keys_and_unsupported_wrappers_stop_safely() {
        for script in [b"ytcfg.set({\"VISITOR_DATA\":\"a\",\"VISITOR_DATA\":\"b\"});".as_slice(),
                       b"ytcfg.set({oops});",b"ytcfg.set({\"VISITOR_DATA\":\"a\"}) + evil();"] {
            let mut data = b"<script>".to_vec();data.extend_from_slice(script);data.extend_from_slice(b"</script>");
            assert!(parse(&data,ID).is_err());
        }
        assert_eq!(parse(b"<script>(() => {ytcfg.set({});})();</script>",ID).err(),Some("watch_player_missing"));
    }
}
