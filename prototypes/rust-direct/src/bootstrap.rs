//! Conservative inert watch-page reader, not an HTML/JavaScript interpreter.
//! Reads static data-literal assignments at the beginning of inline script text.
//! Unsupported wrappers/markup fail closed. No raw values implement Debug or Display.
use serde::{Deserialize, Deserializer, de::{Error as _, IgnoredAny}};
#[cfg(test)]
use serde_json::Value;

pub const PAGE_MAX: usize = 2 * 1024 * 1024;
const JSON_MAX: usize = 1024 * 1024;
const VISITOR_MAX: usize = 4096;

// Fixed counts/booleans only: never page fragments, keys, values, offsets or hashes.
#[derive(Default, serde::Serialize)]
pub struct Observation {
    inline_scripts: usize,
    skipped_script_attributes: usize,
    ignored_script_tails: usize,
    config_objects: usize,
    wrapped_config_blocks: usize,
    initial_players: usize,
    config_visitor_present: bool,
    context_visitor_present: bool,
    player_visitor_present: bool,
    #[serde(skip_serializing_if="Option::is_none")]
    coverage: Option<super::coverage::Inventory>,
}
impl Observation {
    pub fn coverage_only() -> Self {
        Self { coverage: Some(super::coverage::Inventory::default()), ..Self::default() }
    }
}

pub struct Visitor { value: String, metadata_nonfinite_ignored: bool, literal_syntax_normalized: bool }
impl Visitor {
    pub fn as_header(&self) -> &str { &self.value }
    pub fn metadata_nonfinite_ignored(&self) -> bool { self.metadata_nonfinite_ignored }
    pub fn literal_syntax_normalized(&self) -> bool { self.literal_syntax_normalized }
}
#[derive(Default, Deserialize)]
struct Config {
    #[serde(rename="VISITOR_DATA", default, deserialize_with="visitor_field")] visitor: Option<String>,
    #[serde(rename="INNERTUBE_CONTEXT", default, deserialize_with="context_field")] context: Option<Context>,
    #[serde(rename="LOGGED_IN", default, deserialize_with="login_field")] logged_in: Option<bool>,
    #[serde(skip)] metadata_nonfinite_ignored: bool,
    #[serde(skip)] literal_syntax_normalized: bool,
}
#[derive(Deserialize)]
struct Context { #[serde(default, deserialize_with="client_field")] client: Option<ConfigClient> }
#[derive(Deserialize)]
struct ConfigClient { #[serde(rename="visitorData", default, deserialize_with="client_visitor_field")] visitor: Option<String> }
#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
struct InitialPlayer {
    playability_status: Option<super::Playability>,
    video_details: Option<super::Details>,
    response_context: Option<ResponseContext>,
}
#[derive(Deserialize)]
struct ResponseContext { #[serde(rename="visitorData")] visitor: Option<String> }

// Classify schema failures with static tags before any underlying exception reaches
// the CLI. No raw error/value/line/snippet is printed. Syntax is checked separately
// with IgnoredAny so these field wrappers cannot turn invalid JSON into valid data.
struct MapOnly<T>(std::marker::PhantomData<T>);
impl<'de,T: Deserialize<'de>> serde::de::Visitor<'de> for MapOnly<T> {
    type Value = T;
    fn expecting(&self,f: &mut std::fmt::Formatter) -> std::fmt::Result { f.write_str("object") }
    fn visit_map<A: serde::de::MapAccess<'de>>(self,map: A) -> Result<T,A::Error> {
        T::deserialize(serde::de::value::MapAccessDeserializer::new(map))
    }
}
struct ObjectOrNull<T>(std::marker::PhantomData<T>);
impl<'de,T: Deserialize<'de>> serde::de::Visitor<'de> for ObjectOrNull<T> {
    type Value = Option<T>;
    fn expecting(&self,f: &mut std::fmt::Formatter) -> std::fmt::Result { f.write_str("object or null") }
    fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value,E> { Ok(None) }
    fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value,E> { Ok(None) }
    fn visit_some<D: Deserializer<'de>>(self,d: D) -> Result<Self::Value,D::Error> {
        d.deserialize_map(MapOnly::<T>(std::marker::PhantomData)).map(Some)
    }
}
fn object_option<'de,D: Deserializer<'de>,T: Deserialize<'de>>(d: D) -> Result<Option<T>,D::Error> {
    d.deserialize_option(ObjectOrNull::<T>(std::marker::PhantomData))
}
fn visitor_field<'de,D: Deserializer<'de>>(d: D) -> Result<Option<String>,D::Error> {
    Option::<String>::deserialize(d).map_err(|_| D::Error::custom("cfg_visitor_shape"))
}
fn login_field<'de,D: Deserializer<'de>>(d: D) -> Result<Option<bool>,D::Error> {
    Option::<bool>::deserialize(d).map_err(|_| D::Error::custom("cfg_login_shape"))
}
fn client_visitor_field<'de,D: Deserializer<'de>>(d: D) -> Result<Option<String>,D::Error> {
    Option::<String>::deserialize(d).map_err(|_| D::Error::custom("cfg_client_visitor_shape"))
}
fn client_field<'de,D: Deserializer<'de>>(d: D) -> Result<Option<ConfigClient>,D::Error> {
    object_option::<D,ConfigClient>(d).map_err(|e| {
        if e.to_string().starts_with("cfg_client_visitor_shape") { e }
        else { D::Error::custom("cfg_client_shape") }
    })
}
fn context_field<'de,D: Deserializer<'de>>(d: D) -> Result<Option<Context>,D::Error> {
    object_option::<D,Context>(d).map_err(|e| {
        if e.to_string().starts_with("cfg_client_") { e }
        else { D::Error::custom("cfg_context_shape") }
    })
}
// Serde's public category collapses many syntax errors. Match only its known,
// static message prefixes and emit our fixed codes, never the message or position.
fn config_syntax_code(error: &serde_json::Error) -> &'static str {
    if error.classify() == serde_json::error::Category::Eof { return "watch_config_json_incomplete"; }
    let detail = error.to_string();
    for (prefix,code) in [
        ("invalid escape", "watch_config_json_invalid_escape"),
        ("invalid unicode code point", "watch_config_json_unicode"),
        ("lone leading surrogate", "watch_config_json_surrogate"),
        ("unexpected end of hex escape", "watch_config_json_hex_escape"),
        ("control character", "watch_config_json_raw_control"),
        ("key must be a string", "watch_config_json_key_or_trailing_comma"),
        ("expected value", "watch_config_json_expected_value"),
        ("expected ident", "watch_config_json_invalid_keyword"),
        ("expected `:`", "watch_config_json_missing_colon"),
        ("expected `,` or `]`", "watch_config_json_array_separator"),
        ("expected `,` or `}`", "watch_config_json_object_separator"),
        ("expected `\"`", "watch_config_json_expected_quote"),
        ("trailing comma", "watch_config_json_trailing_comma"),
        ("trailing characters", "watch_config_json_trailing_data"),
        ("invalid number", "watch_config_json_invalid_number"),
        ("number out of range", "watch_config_json_number_range"),
        ("recursion limit exceeded", "watch_config_json_parser_depth"),
    ] {
        if detail.starts_with(prefix) { return code; }
    }
    "watch_config_json_syntax_other"
}
fn decode_config(data: &[u8]) -> Result<Config, &'static str> {
    // Canonicalize a bounded static literal, preserving strings/types/duplicates.
    // Nonfinite constants remain metadata-only, never visitor/auth/context values.
    let normalized = super::literal_data::config(data,JSON_MAX)?;
    let metadata_nonfinite_ignored = normalized.nonfinite_ignored;
    let literal_syntax_normalized = normalized.syntax_normalized;
    let data = normalized.json.as_ref();
    // No full Value tree or new dependencies. Both passes stay within JSON_MAX.
    serde_json::from_slice::<IgnoredAny>(data).map_err(|e| config_syntax_code(&e))?;
    let mut config = serde_json::from_slice::<Config>(data).map_err(|e| {
        let detail = e.to_string(); // classification only; never emit underlying text
        for (prefix, code) in [
            ("cfg_visitor_shape", "watch_config_visitor_type"),
            ("cfg_login_shape", "watch_config_login_type"),
            ("cfg_context_shape", "watch_config_context_type"),
            ("cfg_client_shape", "watch_config_client_schema"),
            ("cfg_client_visitor_shape", "watch_config_client_visitor_type"),
            ("duplicate field `VISITOR_DATA`", "watch_config_duplicate_visitor"),
            ("duplicate field `INNERTUBE_CONTEXT`", "watch_config_duplicate_context"),
            ("duplicate field `LOGGED_IN`", "watch_config_duplicate_login"),
        ] {
            if detail.starts_with(prefix) { return code; }
        }
        "watch_config_schema_unsupported"
    })?;
    config.metadata_nonfinite_ignored = metadata_nonfinite_ignored;
    config.literal_syntax_normalized = literal_syntax_normalized;
    Ok(config)
}

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
    super::literal_data::frame_end(data,JSON_MAX)
}

// Consume only allowlisted unrelated display-setting calls. No expression is evaluated,
// and no dynamic visitor/auth/config key is skipped. Names/expressions are fixed.
fn viewport_call(tail: &[u8]) -> Option<&[u8]> {
    for (key, expressions) in [
        (b"initialBodyClientWidth".as_slice(), [b"document.body.clientWidth".as_slice(), b"window.innerWidth"]),
        (b"initialBodyClientHeight".as_slice(), [b"document.body.clientHeight".as_slice(), b"window.innerHeight"]),
        (b"initialInnerWidth".as_slice(), [b"window.innerWidth".as_slice(), b"window.innerWidth"]),
        (b"initialInnerHeight".as_slice(), [b"window.innerHeight".as_slice(), b"window.innerHeight"]),
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
    let after = super::literal_data::trivia(after)?;
    if after.is_empty() { return Ok(after); } // legal end-of-script semicolon omission
    after.strip_prefix(b";").ok_or(if config { "watch_config_terminator_invalid" } else { "watch_player_terminator_invalid" })
}

#[cfg(test)]
pub fn parse(page: &[u8], id: &str) -> Result<Visitor, &'static str> {
    parse_observed(page,id,&mut Observation::default())
}
pub fn parse_observed(page: &[u8], id: &str, observation: &mut Observation) -> Result<Visitor, &'static str> {
    if page.len() > PAGE_MAX { return Err("watch_size_limit"); }
    // Deliberately conservative challenge/consent detection, not a general detector.
    // False positives stop the probe; no consent automation or challenge response.
    for marker in [b"consent.youtube.com/save".as_slice(), b"g-recaptcha", b"/sorry/index", b"confirm you're not a bot", b"confirm you\xe2\x80\x99re not a bot"] {
        if find_ci(page, marker).is_some() { return Err("watch_restriction_detected"); }
    }
    let mut visitor = None;
    let mut initial_seen = false;
    let mut metadata_nonfinite_ignored = false;
    let mut literal_syntax_normalized = false;
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
        if !eq(name,b"script") || template_depth != 0 { continue; }
        if !attributes_safe(attrs) {
            observation.skipped_script_attributes += 1;
            if let Some(coverage)=observation.coverage.as_mut() { coverage.skipped_attributes(script); }
            continue;
        }
        observation.inline_scripts += 1;
        parse_script(script,id,&mut visitor,&mut initial_seen,&mut metadata_nonfinite_ignored,&mut literal_syntax_normalized,observation)?;
    }
    if !initial_seen { return Err("watch_player_missing"); }
    visitor.map(|value| Visitor { value, metadata_nonfinite_ignored, literal_syntax_normalized }).ok_or("watch_visitor_missing")
}
fn word<'a>(data: &'a [u8], token: &[u8]) -> Option<&'a [u8]> {
    let rest=data.strip_prefix(token)?;
    if rest.first().is_some_and(|b| b.is_ascii_alphanumeric() || matches!(b,b'_'|b'$')) { None } else { Some(rest) }
}
fn config_start(script: &[u8]) -> Result<Option<&[u8]>, &'static str> {
    let Some(rest)=word(script,b"ytcfg") else { return Ok(None) };
    let Some(rest)=super::literal_data::trivia(rest)?.strip_prefix(b".") else { return Ok(None) };
    let Some(rest)=word(super::literal_data::trivia(rest)?,b"set") else { return Ok(None) };
    Ok(super::literal_data::trivia(rest)?.strip_prefix(b"("))
}
fn player_start(script: &[u8]) -> Result<Option<&[u8]>, &'static str> {
    let mut data=script;
    for declaration in [b"var".as_slice(),b"let",b"const"] {
        if let Some(rest)=word(data,declaration) { data=super::literal_data::trivia(rest)?; break; }
    }
    let Some(rest)=word(data,b"ytInitialPlayerResponse") else { return Ok(None) };
    Ok(super::literal_data::trivia(rest)?.strip_prefix(b"="))
}
// Match fixed scaffold tokens, never search arbitrary JS or interpret expressions.
// Identifier boundaries matter: functionX/window.ytplayerExtra are not matches.
fn scaffold<'a>(mut data: &'a [u8], pattern: &str) -> Result<Option<&'a [u8]>, &'static str> {
    for token in pattern.split_ascii_whitespace() {
        data=super::literal_data::trivia(data)?;
        let next=if token.as_bytes()[0].is_ascii_alphabetic() || token.starts_with('_') {
            word(data,token.as_bytes())
        } else { data.strip_prefix(token.as_bytes()) };
        let Some(rest)=next else { return Ok(None) }; data=rest;
    }
    Ok(Some(data))
}
fn wrapper_open(script: &[u8]) -> Result<Option<&[u8]>, &'static str> {
    let Some(body)=scaffold(script,"( function ( ) {")? else { return Ok(None) };
    // Only a straight-line data prefix, optionally preceded by the exact known
    // player namespace reset. Never descend into branches/named/async functions.
    let body=super::literal_data::trivia(body)?;
    if let Some(rest)=scaffold(body,"window . ytplayer = { } ;")? {
        return Ok(Some(rest));
    }
    if config_start(body)?.is_some() { return Ok(Some(body)); }
    Ok(None)
}
fn wrapper_close(script: &[u8]) -> Result<Option<&[u8]>, &'static str> {
    for pattern in ["} ) ( )", "} ( ) )"] {
        if let Some(rest)=scaffold(script,pattern)? {
            return statement_end(rest,true).map(Some).map_err(|_| "watch_config_wrapper_terminator_invalid");
        }
    }
    Ok(None)
}
// The archive's localization helper is accepted only as this entire fixed scaffold
// followed by one static object and its call terminator. It is NOT run. Unknown
// suffixes, including later auth/config mutations inside the wrapper, fail closed.
fn localization_message(script: &[u8]) -> Result<Option<&[u8]>, &'static str> {
    let pattern="var setMessage = function ( msg ) { if ( window . yt && yt . setMsg ) yt . setMsg ( msg ) ; else { window . ytcfg = window . ytcfg || { } ; ytcfg . msgs = msg } } ; setMessage (";
    let Some(tail)=scaffold(script,pattern)? else { return Ok(None) };
    let tail=super::literal_data::trivia(tail)?;
    let end=json_end(tail)?;
    let normalized=super::literal_data::player(&tail[..end],JSON_MAX)?;
    serde_json::from_slice::<IgnoredAny>(&normalized.json).map_err(|_| "watch_messages_invalid")?;
    let rest=super::literal_data::trivia(&tail[end..])?.strip_prefix(b")").ok_or("watch_messages_closer_missing")?;
    Ok(Some(statement_end(rest,true)?))
}
fn parse_script(mut script: &[u8], id: &str, visitor: &mut Option<String>, initial_seen: &mut bool, metadata_nonfinite_ignored: &mut bool, literal_syntax_normalized: &mut bool, observation: &mut Observation) -> Result<(), &'static str> {
    let mut wrapper = None;
    let mut messages_seen = false;
    // Intentionally don't search inside arbitrary JS (strings/comments/functions).
    // Only recognized literals at script start or after another accepted literal.
    loop {
        script = super::literal_data::trivia(script)?;
        if let Some(before) = wrapper {
            if let Some(next)=wrapper_close(script)? {
                if observation.config_objects == before { return Err("watch_config_wrapper_empty"); }
                observation.wrapped_config_blocks += 1;
                wrapper=None; script=next; continue;
            }
            if !messages_seen && observation.config_objects > before {
                if let Some(next)=localization_message(script)? {
                    messages_seen=true; script=next; continue;
                }
            }
        }
        if let Some(body)=wrapper_open(script)? {
            if wrapper.is_some() { return Err("watch_config_wrapper_nested"); }
            wrapper=Some(observation.config_objects); messages_seen=false; script=body; continue;
        }
        let (tail, config) = if let Some(tail)=config_start(script)? {
            (super::literal_data::trivia(tail)?,true)
        } else if let Some(tail)=player_start(script)? {
            (super::literal_data::trivia(tail)?,false)
        } else {
            if wrapper.is_some() { return Err("watch_config_wrapper_unsupported"); }
            if !script.is_empty() {
                observation.ignored_script_tails += 1;
                if let Some(coverage)=observation.coverage.as_mut() { coverage.ignored(script); }
            }
            return Ok(());
        };
        if wrapper.is_some() && !config { return Err("watch_config_wrapper_unsupported"); }
        if config && tail.first() != Some(&b'{') {
            if let Some(after) = viewport_call(tail) {
                script = statement_end(after,true)?;
                continue;
            }
            return Err("watch_config_nonliteral_unsupported");
        }
        if !config && tail.first() != Some(&b'{') { return Err("watch_player_nonliteral_unsupported"); }
        let end = json_end(tail)?;
        let after = super::literal_data::trivia(&tail[end..])?;
        let after = if config {
            trim(after.strip_prefix(b")").ok_or("watch_config_closer_missing")?)
        } else { after };
        let next = statement_end(after,config)?;
        if config {
            let cfg = decode_config(&tail[..end])?;
            *metadata_nonfinite_ignored |= cfg.metadata_nonfinite_ignored;
            *literal_syntax_normalized |= cfg.literal_syntax_normalized;
            if cfg.logged_in == Some(true) { return Err("watch_authenticated_unsupported"); }
            observation.config_objects += 1;
            observation.config_visitor_present |= cfg.visitor.is_some();
            let context_visitor=cfg.context.and_then(|c| c.client).and_then(|c| c.visitor);
            observation.context_visitor_present |= context_visitor.is_some();
            remember(visitor,cfg.visitor)?;
            remember(visitor,context_visitor)?;
        } else {
            if *initial_seen { return Err("watch_player_ambiguous"); }
            let normalized = super::literal_data::player(&tail[..end],JSON_MAX)?;
            *literal_syntax_normalized |= normalized.syntax_normalized;
            let pr: InitialPlayer = serde_json::from_slice(&normalized.json).map_err(|_| "watch_player_invalid")?;
            // Do not carry watch-page formats into the media path, and honor its
            // playability restrictions before making a second provider request.
            let player = super::Player { playability_status: pr.playability_status,
                video_details: pr.video_details, streaming_data: None };
            super::playability(&player)?;
            super::identity(&player,id)?;
            let response_visitor=pr.response_context.and_then(|r| r.visitor);
            observation.player_visitor_present |= response_visitor.is_some();
            remember(visitor,response_visitor)?;
            observation.initial_players += 1;
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
    #[test] fn config_diagnostics_classify_types_without_values_or_coercion() {
        for (data,code) in [
            (r#"{"VISITOR_DATA":123}"#,"watch_config_visitor_type"),
            (r#"{"LOGGED_IN":"PRIVATE_AUTH"}"#,"watch_config_login_type"),
            (r#"{"INNERTUBE_CONTEXT":[]}"#,"watch_config_context_type"),
            (r#"{"INNERTUBE_CONTEXT":{"client":false}}"#,"watch_config_client_schema"),
            (r#"{"INNERTUBE_CONTEXT":{"client":{"visitorData":["PRIVATE"]}}}"#,"watch_config_client_visitor_type"),
            (r#"{"VISITOR_DATA":"a","VISITOR_DATA":"b"}"#,"watch_config_duplicate_visitor"),
            (r#"{"INNERTUBE_CONTEXT":{},"INNERTUBE_CONTEXT":{}}"#,"watch_config_duplicate_context"),
            (r#"{"LOGGED_IN":false,"LOGGED_IN":true}"#,"watch_config_duplicate_login"),
            (r#"{"VISITOR_DATA":undefined}"#,"watch_config_js_undefined"),
            (r#"{"VISITOR_DATA":"PRIVATE""#,"watch_json_unterminated"),
        ] {
            let error = decode_config(data.as_bytes()).err().unwrap();
            assert_eq!(error,code);
            assert!(!error.contains("PRIVATE"));
        }
    }
    #[test] fn null_and_missing_still_do_not_bypass_visitor_or_auth_checks() {
        assert!(decode_config(br#"{"VISITOR_DATA":null,"LOGGED_IN":null,"INNERTUBE_CONTEXT":null}"#).is_ok());
        let data = page(serde_json::json!({"VISITOR_DATA":null}),player());
        assert_eq!(parse(&data,ID).err(),Some("watch_visitor_missing"));
        assert_eq!(parse(&page(serde_json::json!({"LOGGED_IN":true}),player()),ID).err(),Some("watch_authenticated_unsupported"));
    }
    #[test] fn syntax_subtypes_are_fixed_and_never_include_input_or_positions() {
        for (data,expected) in [
            (r#"{"PRIVATE":"\x41"}"#,"watch_config_json_invalid_escape"),
            ("{\"PRIVATE\":\"raw\nline\"}","watch_config_json_raw_control"),
            (r#"{PRIVATE:1}"#,"watch_config_json_key_or_trailing_comma"),
            (r#"{"PRIVATE":undefined}"#,"watch_config_json_expected_value"),
            (r#"{"PRIVATE":NaN}"#,"watch_config_json_expected_value"),
            (r#"{"PRIVATE":truX}"#,"watch_config_json_invalid_keyword"),
            (r#"{"PRIVATE" 1}"#,"watch_config_json_missing_colon"),
            (r#"{"PRIVATE":[1 2]}"#,"watch_config_json_array_separator"),
            (r#"{"PRIVATE":1 "OTHER":2}"#,"watch_config_json_object_separator"),
            (r#"{"PRIVATE":1,}"#,"watch_config_json_key_or_trailing_comma"),
            (r#"{"PRIVATE":1} false"#,"watch_config_json_trailing_data"),
            (r#"{"PRIVATE":01}"#,"watch_config_json_invalid_number"),
            (r#"{"PRIVATE":"#,"watch_config_json_incomplete"),
        ] {
            let error = serde_json::from_slice::<IgnoredAny>(data.as_bytes()).unwrap_err();
            let code = config_syntax_code(&error);
            assert_eq!(code,expected);
            assert!(!code.contains("PRIVATE"));
            assert!(!code.contains("line "));
            assert!(!code.contains("column"));
        }
    }
    #[test] fn expressions_and_invalid_escapes_are_not_repaired_even_with_valid_known_fields() {
        for bad in [r#"{"VISITOR_DATA":"good","IGNORED":"\xGG"}"#,
                    r#"{"VISITOR_DATA":"good","IGNORED":secret()}"#,r#"{"VISITOR_DATA":"good","IGNORED":undefined}"#] {
            let data = format!("<script>ytcfg.set({bad});</script><script>var ytInitialPlayerResponse = {};</script>",player());
            assert!(parse(data.as_bytes(),ID).is_err());
        }
    }
    #[test] fn discarded_nonfinite_metadata_does_not_change_visitor_auth_or_duplicate_rules() {
        let config = br#"{"VISITOR_DATA":"opaque_fixture","LOGGED_IN":false,"EXPERIMENTS":{"values":[NaN,Infinity,-Infinity]}}"#;
        let mut data = b"<script>ytcfg.set(".to_vec(); data.extend_from_slice(config); data.extend_from_slice(b");</script>");
        data.extend_from_slice(format!("<script>var ytInitialPlayerResponse = {};</script>",player()).as_bytes());
        let visitor = parse(&data,ID).unwrap();
        assert_eq!(visitor.as_header(),"opaque_fixture");
        assert!(visitor.metadata_nonfinite_ignored());
        assert_eq!(decode_config(br#"{"LOGGED_IN":NaN}"#).err(),Some("watch_config_nonfinite_protected"));
        assert_eq!(decode_config(br#"{"VISITOR_DATA":"a","VISITOR_DATA":"b","IGNORED":Infinity}"#).err(),Some("watch_config_duplicate_visitor"));
        assert_eq!(parse(&page(serde_json::json!({"LOGGED_IN":true,"VISITOR_DATA":"x"}),player()),ID).err(),Some("watch_authenticated_unsupported"));
    }
    #[test] fn full_single_quoted_page_and_statement_spacing_preserve_restrictions() {
        let data=br#"<html><script nonce="x">/* harmless */ ytcfg . set ( {'VISITOR_DATA':'opaque_123',LOGGED_IN:false,info:'braces } [ and \' escape',} ) ;</script><script>const ytInitialPlayerResponse={playabilityStatus:{status:'OK'},videoDetails:{videoId:'Jm7Satp7Zqk'},};</script></html>"#;
        let visitor=parse(data,ID).unwrap();
        assert_eq!(visitor.as_header(),"opaque_123"); assert!(visitor.literal_syntax_normalized());
        assert!(!visitor.metadata_nonfinite_ignored());
        for (cfg,code) in [("{VISITOR_DATA:'good',LOGGED_IN:true}","watch_authenticated_unsupported"),
            ("{VISITOR_DATA:'good',LOGGED_IN:'false'}","watch_config_login_type"),
            ("{'VISITOR_DATA':'a',VISITOR_DATA:'b'}","watch_config_duplicate_visitor"),
            (r#"{VISITOR_DATA:'bad\r\nvalue'}"#,"watch_visitor_invalid")] {
            let html=format!("<script>ytcfg.set({cfg});</script><script>let ytInitialPlayerResponse = {};</script>",player());
            assert_eq!(parse(html.as_bytes(),ID).err(),Some(code));
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
    fn with_script(script: &str) -> Vec<u8> {
        format!("<script>{script}</script><script>var ytInitialPlayerResponse={};</script>",player()).into_bytes()
    }
    #[test] fn archive_shaped_synthetic_wrapper_is_read_without_evaluating_scaffold() {
        let data=include_bytes!("../fixtures/static-wrapper.html");
        let mut observed=Observation::default();
        let visitor=parse_observed(data,ID,&mut observed).unwrap();
        assert_eq!(visitor.as_header(),"fixture_opaque_NEVER_PRINT");
        assert!(visitor.literal_syntax_normalized()); assert!(visitor.metadata_nonfinite_ignored());
        let report=serde_json::to_value(observed).unwrap();
        assert_eq!(report["inline_scripts"],3); assert_eq!(report["config_objects"],1);
        assert_eq!(report["wrapped_config_blocks"],1); assert_eq!(report["initial_players"],1);
        assert_eq!(report["ignored_script_tails"],2);
        assert_eq!(report["config_visitor_present"],true); assert_eq!(report["context_visitor_present"],true);
        for private in ["fixture_opaque_NEVER_PRINT","Hello","DOWNLOAD","must_not_execute"] {
            assert!(!report.to_string().contains(private));
        }
    }
    #[test] fn complete_parameterless_wrappers_allow_trivia_and_both_invocation_forms() {
        for script in ["(function(){ytcfg.set({VISITOR_DATA:'opaque'});})();",
            "( /* a */ function /* b */ ( ) { window . ytplayer = { } ; ytcfg . set ({VISITOR_DATA:'opaque'}); } ( ) ) ;",
            "(function(){ytcfg.set({VISITOR_DATA:'opaque'});ytcfg.set({LOGGED_IN:false});})();"] {
            assert_eq!(parse(&with_script(script),ID).unwrap().as_header(),"opaque");
        }
    }
    #[test] fn uninvoked_partial_nested_or_effectful_wrappers_never_supply_context() {
        for script in ["(function(){ytcfg.set({VISITOR_DATA:'PRIVATE'});});",
            "(function(){ytcfg.set({VISITOR_DATA:'PRIVATE'});}",
            "(function(){ytcfg.set({VISITOR_DATA:'PRIVATE'});}).call(window);",
            "(function(){ytcfg.set({VISITOR_DATA:'PRIVATE'});}).bind(window)();",
            "(function(){ytcfg.set({VISITOR_DATA:'PRIVATE'});return;})();",
            "(function(){ytcfg.set({VISITOR_DATA:'PRIVATE'});evil();})();",
            "(function(){ytcfg.set({VISITOR_DATA:'PRIVATE'});(function(){ytcfg.set({});})();})();",
            "(function(){window.ytplayer={};})();",
            "(function(){ytcfg.set({VISITOR_DATA:'PRIVATE'});ytcfg.set('LOGGED_IN',getLogin());})();"] {
            let error=parse(&with_script(script),ID).err().unwrap();
            assert!(!error.contains("PRIVATE"));
        }
    }
    #[test] fn wrappers_never_downgrade_login_duplicate_conflict_or_restriction_checks() {
        for (cfg,error) in [("{VISITOR_DATA:'opaque',LOGGED_IN:true}","watch_authenticated_unsupported"),
            ("{VISITOR_DATA:'opaque',LOGGED_IN:'false'}","watch_config_login_type"),
            ("{VISITOR_DATA:'one',VISITOR_DATA:'two'}","watch_config_duplicate_visitor"),
            ("{VISITOR_DATA:'one',INNERTUBE_CONTEXT:{client:{visitorData:'two'}}}","watch_visitor_ambiguous"),
            (r#"{VISITOR_DATA:'bad\r\nvalue'}"#,"watch_visitor_invalid")] {
            let script=format!("(function(){{window.ytplayer={{}};ytcfg.set({cfg});}})();");
            assert_eq!(parse(&with_script(&script),ID).err(),Some(error));
        }
        let data=with_script("(function(){ytcfg.set({VISITOR_DATA:'x'});ytcfg.set({LOGGED_IN:true});})();");
        assert_eq!(parse(&data,ID).err(),Some("watch_authenticated_unsupported"));
        let data=include_bytes!("../fixtures/static-wrapper.html");
        let restricted=String::from_utf8_lossy(data).replace("status:'OK'","status:'LOGIN_REQUIRED'");
        assert_eq!(parse(restricted.as_bytes(),ID).err(),Some("player_login_required"));
    }
    #[test] fn no_search_in_named_conditional_async_arrow_or_quoted_functions() {
        for script in ["(function named(){ytcfg.set({VISITOR_DATA:'PRIVATE'});})();",
            "(function(arg){ytcfg.set({VISITOR_DATA:'PRIVATE'});})();",
            "(async function(){ytcfg.set({VISITOR_DATA:'PRIVATE'});})();",
            "(()=>{ytcfg.set({VISITOR_DATA:'PRIVATE'});})();",
            "(function(){if(false){ytcfg.set({VISITOR_DATA:'PRIVATE'});}})();",
            "(function(){throw 1;ytcfg.set({VISITOR_DATA:'PRIVATE'});})();",
            "var text=\"(function(){ytcfg.set({VISITOR_DATA:'PRIVATE'});})();\";",
            "/* (function(){ytcfg.set({VISITOR_DATA:'PRIVATE'});})(); */"] {
            assert_eq!(parse(&with_script(script),ID).err(),Some("watch_visitor_missing"));
        }
    }
    #[test] fn unknown_or_dynamic_message_helper_is_not_silently_skipped() {
        let data=String::from_utf8_lossy(include_bytes!("../fixtures/static-wrapper.html"));
        for changed in [data.replace("ytcfg.msgs=msg","ytcfg.LOGGED_IN=true"),
            data.replace("setMessage({","setMessage(getMessages(),{"),
            data.replace("\"HELLO\":\"Hello\"","\"HELLO\":secret()"),
            data.replace("setMessage({","setMessage(`template`,{")] {
            assert!(parse(changed.as_bytes(),ID).is_err());
        }
    }
    #[test] fn missing_visitor_observations_distinguish_absent_data_from_skipped_scaffold() {
        let mut observed=Observation::default();
        let absent=with_script("(function(){ytcfg.set({LOGGED_IN:false});})();");
        assert_eq!(parse_observed(&absent,ID,&mut observed).err(),Some("watch_visitor_missing"));
        assert_eq!(observed.config_objects,1); assert_eq!(observed.wrapped_config_blocks,1);
        assert_eq!(observed.initial_players,1); assert!(!observed.config_visitor_present);
        let mut observed=Observation::default();
        let unsupported=with_script("if(false){ytcfg.set({VISITOR_DATA:'PRIVATE'});}");
        assert_eq!(parse_observed(&unsupported,ID,&mut observed).err(),Some("watch_visitor_missing"));
        assert_eq!(observed.config_objects,0); assert_eq!(observed.ignored_script_tails,1);
        assert!(!serde_json::to_string(&observed).unwrap().contains("PRIVATE"));
    }
    #[test] fn inert_or_external_wrappers_are_not_visitor_sources_and_attributes_are_counted() {
        for container in ["<template><script>{}</script></template>","<textarea><script>{}</script></textarea>",
            "<script src='external.js'>{}</script>","<script type='application/json'>{}</script>"] {
            let data=container.replace("{}","(function(){ytcfg.set({VISITOR_DATA:'PRIVATE'});})();");
            let mut data=data.into_bytes(); data.extend_from_slice(&with_script(""));
            assert_eq!(parse(&data,ID).err(),Some("watch_visitor_missing"));
        }
        let mut data=b"<script src='PRIVATE.js'>ignored</script>".to_vec();data.extend_from_slice(&with_script(""));
        let mut observed=Observation::default();
        assert_eq!(parse_observed(&data,ID,&mut observed).err(),Some("watch_visitor_missing"));
        assert_eq!(observed.skipped_script_attributes,1);
        assert!(!serde_json::to_string(&observed).unwrap().contains("PRIVATE"));
    }

    #[test] fn coverage_inventory_does_not_turn_ignored_strings_or_functions_into_data() {
        for script in ["var text=\"ytcfg.set({VISITOR_DATA:'PRIVATE'});\";",
            "(function(arg){ytcfg.set({VISITOR_DATA:'PRIVATE'});})();",
            "window.fake='ytcfg VISITOR_DATA';"] {
            let data=with_script(script);
            let mut ordinary=Observation::default();
            let mut coverage=Observation::coverage_only();
            assert_eq!(parse_observed(&data,ID,&mut ordinary).err(),Some("watch_visitor_missing"));
            assert_eq!(parse_observed(&data,ID,&mut coverage).err(),Some("watch_visitor_missing"));
            let mut ordinary=serde_json::to_value(ordinary).unwrap();
            let mut coverage=serde_json::to_value(coverage).unwrap();
            assert_eq!(coverage["coverage"]["ignored_tails"]["with_raw_visitor_name"],1);
            assert!(!coverage.to_string().contains("PRIVATE"));
            assert!(ordinary.get("coverage").is_none());
            coverage.as_object_mut().unwrap().remove("coverage");
            assert_eq!(ordinary.take(),coverage);
        }
    }
    #[test] fn coverage_inventory_reports_skipped_attribute_bodies_but_does_not_trust_them() {
        let mut data=b"<script id='PRIVATE'>ytcfg.set({VISITOR_DATA:'PRIVATE'});</script>".to_vec();
        data.extend_from_slice(&with_script(""));
        let mut observed=Observation::coverage_only();
        assert_eq!(parse_observed(&data,ID,&mut observed).err(),Some("watch_visitor_missing"));
        let report=serde_json::to_value(observed).unwrap();
        assert_eq!(report["coverage"]["skipped_attribute_bodies"]["nonempty"],1);
        assert_eq!(report["coverage"]["skipped_attribute_bodies"]["with_raw_visitor_name"],1);
        assert_eq!(report["config_objects"],0); assert_eq!(report["config_visitor_present"],false);
        assert!(!report.to_string().contains("PRIVATE"));
    }

}
