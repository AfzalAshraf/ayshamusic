//! Narrow compatibility for Python JSON's NaN/Infinity constants in discarded
//! config metadata only. This never evaluates JavaScript or changes protected data.
use std::{borrow::Cow, ops::Range};
const MAX_REPLACEMENTS: usize = 256;

struct Scanner<'a> {
    data: &'a [u8],
    pos: usize,
    replacements: Vec<Range<usize>>,
}
impl Scanner<'_> {
    fn whitespace(&mut self) {
        while self.data.get(self.pos).is_some_and(u8::is_ascii_whitespace) { self.pos += 1; }
    }
    fn string(&mut self) -> Result<Range<usize>, &'static str> {
        let start = self.pos;
        self.pos += 1;
        let mut escaped = false;
        while let Some(&b) = self.data.get(self.pos) {
            self.pos += 1;
            if escaped { escaped = false; }
            else if b == b'\\' { escaped = true; }
            else if b == b'"' { return Ok(start..self.pos); }
        }
        Err("watch_config_json_incomplete")
    }
    fn value(&mut self, protected: bool, depth: usize) -> Result<(), &'static str> {
        if depth > 64 { return Err("watch_json_depth_limit"); }
        self.whitespace();
        match self.data.get(self.pos).copied().ok_or("watch_config_json_incomplete")? {
            b'"' => { self.string()?; }
            b'{' => {
                self.pos += 1; self.whitespace();
                if self.data.get(self.pos) == Some(&b'}') { self.pos += 1; return Ok(()); }
                loop {
                    self.whitespace();
                    if self.data.get(self.pos) != Some(&b'"') { return Err("watch_config_json_key_or_trailing_comma"); }
                    let key = self.string()?;
                    // Decode escaped key names too, so \u0056ISITOR_DATA cannot
                    // evade the protected-field rule. Never emit the key value.
                    let key: String = serde_json::from_slice(&self.data[key]).map_err(|_| "watch_config_json_invalid_key")?;
                    let protect_child = protected || matches!(key.as_str(),
                        "VISITOR_DATA" | "LOGGED_IN" | "INNERTUBE_CONTEXT" | "visitorData" |
                        "playabilityStatus" | "videoDetails" | "responseContext");
                    drop(key);
                    self.whitespace();
                    if self.data.get(self.pos) != Some(&b':') { return Err("watch_config_json_missing_colon"); }
                    self.pos += 1;
                    self.value(protect_child,depth+1)?;
                    self.whitespace();
                    match self.data.get(self.pos) {
                        Some(b'}') => { self.pos += 1; break; }
                        Some(b',') => self.pos += 1,
                        None => return Err("watch_config_json_incomplete"),
                        _ => return Err("watch_config_json_object_separator"),
                    }
                }
            }
            b'[' => {
                self.pos += 1; self.whitespace();
                if self.data.get(self.pos) == Some(&b']') { self.pos += 1; return Ok(()); }
                loop {
                    self.value(protected,depth+1)?;
                    self.whitespace();
                    match self.data.get(self.pos) {
                        Some(b']') => { self.pos += 1; break; }
                        Some(b',') => self.pos += 1,
                        None => return Err("watch_config_json_incomplete"),
                        _ => return Err("watch_config_json_array_separator"),
                    }
                }
            }
            _ => {
                let start = self.pos;
                while self.data.get(self.pos).is_some_and(|b| !b.is_ascii_whitespace() && !matches!(*b,b','|b']'|b'}')) {
                    self.pos += 1;
                }
                let token = &self.data[start..self.pos];
                match token {
                    b"NaN" | b"Infinity" | b"-Infinity" => {
                        if protected { return Err("watch_config_nonfinite_protected"); }
                        if self.replacements.len() >= MAX_REPLACEMENTS { return Err("watch_config_nonfinite_limit"); }
                        self.replacements.push(start..self.pos);
                    }
                    b"true" | b"false" | b"null" => {}
                    b"undefined" => return Err("watch_config_js_undefined"),
                    _ => match token.first() {
                        // Numbers are left byte-for-byte for Serde to validate.
                        Some(b'0'..=b'9' | b'-') => {}
                        Some(b'\'') => return Err("watch_config_js_single_quote"),
                        Some(b'`') => return Err("watch_config_js_template"),
                        Some(b'/' ) => return Err("watch_config_js_comment_or_regex"),
                        Some(b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'$') => return Err("watch_config_js_identifier"),
                        _ => return Err("watch_config_js_expression"),
                    },
                }
            }
        }
        Ok(())
    }
}

pub fn metadata_only(data: &[u8], max: usize) -> Result<Cow<'_, [u8]>, &'static str> {
    if data.len() > max { return Err("watch_json_size_limit"); }
    let mut scanner = Scanner { data, pos:0, replacements:Vec::new() };
    scanner.value(false,0)?;
    scanner.whitespace();
    if scanner.pos != data.len() { return Err("watch_config_json_trailing_data"); }
    if scanner.replacements.is_empty() { return Ok(Cow::Borrowed(data)); }
    let mut output = Vec::with_capacity(data.len());
    let mut previous = 0;
    for range in scanner.replacements {
        let segment = &data[previous..range.start];
        if segment.len()+4 > max.saturating_sub(output.len()) { return Err("watch_json_size_limit"); }
        output.extend_from_slice(segment);
        // Config's typed reader discards these metadata values; null is only an
        // intermediate placeholder, never returned as visitor/auth/session data.
        output.extend_from_slice(b"null");
        previous = range.end;
    }
    if data.len()-previous > max.saturating_sub(output.len()) { return Err("watch_json_size_limit"); }
    output.extend_from_slice(&data[previous..]);
    Ok(Cow::Owned(output))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn only_exact_nonfinite_metadata_tokens_are_replaced() {
        let input = br#"{"IGNORED":{"values":[NaN,Infinity,-Infinity]},"VISITOR_DATA":"Infinity","LOGGED_IN":false}"#;
        let out = metadata_only(input,1024).unwrap();
        assert!(matches!(out,Cow::Owned(_)));
        let value: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(value["IGNORED"]["values"],serde_json::json!([null,null,null]));
        assert_eq!(value["VISITOR_DATA"],"Infinity");
        assert_eq!(value["LOGGED_IN"],false);
    }
    #[test] fn protected_fields_and_escaped_keys_never_get_coerced() {
        for data in [br#"{"VISITOR_DATA":NaN}"#.as_slice(), br#"{"LOGGED_IN":Infinity}"#,
            br#"{"INNERTUBE_CONTEXT":{"ignored":-Infinity}}"#,
            br#"{"\u0056ISITOR_DATA":NaN}"#,br#"{"IGNORED":{"visitorData":NaN}}"#] {
            assert_eq!(metadata_only(data,1024).err(),Some("watch_config_nonfinite_protected"));
        }
    }
    #[test] fn strings_and_valid_json_are_borrowed_unchanged() {
        let input = br#"{"IGNORED":"NaN, Infinity, -Infinity, undefined, \\" ,"n":1.25,"x":null}"#;
        let out = metadata_only(input,1024).unwrap();
        assert!(matches!(out,Cow::Borrowed(_)));
        assert_eq!(&*out,input);
        assert!(serde_json::from_slice::<serde_json::Value>(&out).is_ok());
    }
    #[test] fn unknown_js_tokens_report_fixed_subtypes_without_data() {
        for (token,code) in [("undefined","watch_config_js_undefined"),("PRIVATE.call()","watch_config_js_identifier"),
            ("'PRIVATE'","watch_config_js_single_quote"),("`PRIVATE`","watch_config_js_template"),
            ("/*PRIVATE*/1","watch_config_js_comment_or_regex"),("(PRIVATE)","watch_config_js_expression")] {
            let input = format!("{{\"IGNORED\":{token}}}");
            let error = metadata_only(input.as_bytes(),1024).err().unwrap();
            assert_eq!(error,code);
            assert!(!error.contains("PRIVATE"));
        }
        assert!(metadata_only(br#"{"x":InfinityPrivate}"#,1024).is_err());
    }
    #[test] fn bounds_and_replacement_expansion_are_enforced() {
        assert_eq!(metadata_only(b"{}",1).err(),Some("watch_json_size_limit"));
        assert_eq!(metadata_only(br#"{"x":NaN}"#,9).err(),Some("watch_json_size_limit"));
        let many = format!("[{}]",vec!["NaN";MAX_REPLACEMENTS+1].join(","));
        assert_eq!(metadata_only(many.as_bytes(),4096).err(),Some("watch_config_nonfinite_limit"));
        let deep = format!("{}null{}","[".repeat(66),"]".repeat(66));
        assert_eq!(metadata_only(deep.as_bytes(),4096).err(),Some("watch_json_depth_limit"));
    }
    #[test] fn it_does_not_repair_separators_comments_or_executable_continuations() {
        for data in [br#"{"x":NaN,}"#.as_slice(),br#"{"x":[NaN 1]}"#,
            br#"{"x":NaN/*PRIVATE*/}"#,br#"{"x":Infinity+1}"#] {
            assert!(metadata_only(data,1024).is_err());
        }
    }
}
