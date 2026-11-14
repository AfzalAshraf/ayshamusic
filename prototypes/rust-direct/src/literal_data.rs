//! Bounded static data literals, never a JavaScript interpreter.
//! Grammar: objects/arrays, quoted or ASCII identifier keys, static strings,
//! JSON numbers/bools/null, comments and trailing commas. Calls, operators,
//! computed keys, templates, regex, spread and references are rejected.
use std::borrow::Cow;

pub struct Normalized<'a> {
    pub json: Cow<'a, [u8]>,
    pub syntax_normalized: bool,
    pub nonfinite_ignored: bool,
}
const MAX_DEPTH: usize = 64;
const MAX_NONFINITE: usize = 256;

fn protected(key: &str) -> bool {
    matches!(key,"VISITOR_DATA"|"LOGGED_IN"|"INNERTUBE_CONTEXT"|"visitorData"|
        "playabilityStatus"|"videoDetails"|"responseContext")
}
fn identifier_start(b: u8) -> bool { b.is_ascii_alphabetic() || matches!(b,b'_'|b'$') }
fn identifier_part(b: u8) -> bool { identifier_start(b) || b.is_ascii_digit() }

// Shared trivia handling for data and its containing statement. Unterminated
// comments fail instead of searching past arbitrary source text.
pub fn trivia(mut data: &[u8]) -> Result<&[u8], &'static str> {
    loop {
        while data.first().is_some_and(u8::is_ascii_whitespace) { data=&data[1..]; }
        if let Some(rest) = data.strip_prefix(b"//") {
            data = rest.iter().position(|b| matches!(b,b'\r'|b'\n')).map_or(&[][..],|n| &rest[n..]);
        } else if let Some(rest) = data.strip_prefix(b"/*") {
            let end=rest.windows(2).position(|w| w==b"*/").ok_or("watch_literal_comment_unterminated")?;
            data=&rest[end+2..];
        } else { return Ok(data); }
    }
}

// Find a bounded object literal without being confused by quoted braces, escaped
// quotes or comments. Actual grammar/type validation happens after framing.
pub fn frame_end(data: &[u8], max: usize) -> Result<usize, &'static str> {
    if data.first()!=Some(&b'{') { return Err("watch_json_object_expected"); }
    let mut stack=[0u8;MAX_DEPTH]; let mut depth=0; let mut pos=0;
    let mut quote=None; let mut escaped=false;
    while pos<data.len() {
        if pos>=max { return Err("watch_json_size_limit"); }
        let b=data[pos];
        if let Some(q)=quote {
            if escaped { escaped=false; } else if b==b'\\' { escaped=true; }
            else if b==q { quote=None; }
            pos+=1; continue;
        }
        if data[pos..].starts_with(b"//") || data[pos..].starts_with(b"/*") {
            let tail=trivia(&data[pos..data.len().min(max)])?;
            pos=data.len().min(max)-tail.len(); continue;
        }
        match b {
            b'\''|b'"' => quote=Some(b),
            b'{'|b'[' => {
                if depth==MAX_DEPTH { return Err("watch_json_depth_limit"); }
                stack[depth]=b; depth+=1;
            }
            b'}'|b']' => {
                if depth==0 || stack[depth-1] != if b==b'}' { b'{' } else { b'[' } {
                    return Err("watch_json_unbalanced");
                }
                depth-=1;
                if depth==0 { return Ok(pos+1); }
            }
            _=>{}
        }
        pos+=1;
    }
    Err("watch_json_unterminated")
}

struct Reader<'a> {
    data: &'a [u8], pos: usize, output: Vec<u8>, max: usize,
    allow_metadata_nonfinite: bool, nonfinite: usize,
}
impl Reader<'_> {
    fn space(&mut self) -> Result<(), &'static str> {
        let tail=trivia(&self.data[self.pos..])?;
        self.pos=self.data.len()-tail.len(); Ok(())
    }
    fn emit(&mut self, value: &[u8]) -> Result<(), &'static str> {
        if value.len()>self.max.saturating_sub(self.output.len()) { return Err("watch_json_size_limit"); }
        self.output.extend_from_slice(value); Ok(())
    }
    fn hex(&mut self,n: usize) -> Result<u32, &'static str> {
        let mut value=0;
        for _ in 0..n {
            let b=*self.data.get(self.pos).ok_or("watch_literal_escape_incomplete")?;
            let digit=(b as char).to_digit(16).ok_or("watch_literal_escape_invalid")?;
            value=value*16+digit; self.pos+=1;
        }
        Ok(value)
    }
    fn unicode(&mut self) -> Result<char, &'static str> {
        let mut value=self.hex(4)?;
        if (0xd800..=0xdbff).contains(&value) {
            if !self.data[self.pos..].starts_with(b"\\u") { return Err("watch_literal_unicode_invalid"); }
            self.pos+=2;
            let low=self.hex(4)?;
            if !(0xdc00..=0xdfff).contains(&low) { return Err("watch_literal_unicode_invalid"); }
            value=0x10000+((value-0xd800)<<10)+(low-0xdc00);
        }
        char::from_u32(value).ok_or("watch_literal_unicode_invalid")
    }
    fn string(&mut self) -> Result<String, &'static str> {
        let quote=self.data[self.pos]; self.pos+=1;
        // A decoded string cannot exceed the remaining encoded payload length.
        let mut out=Vec::with_capacity(self.data.len()-self.pos);
        loop {
            let b=*self.data.get(self.pos).ok_or("watch_literal_string_unterminated")?;
            self.pos+=1;
            if b==quote { return String::from_utf8(out).map_err(|_| "watch_literal_utf8_invalid"); }
            if b==b'\\' {
                let escape=*self.data.get(self.pos).ok_or("watch_literal_escape_incomplete")?; self.pos+=1;
                match escape {
                    b'\''|b'"'|b'\\'|b'/' => out.push(escape),
                    b'b'=>out.push(8), b'f'=>out.push(12), b'n'=>out.push(10),
                    b'r'=>out.push(13), b't'=>out.push(9), b'v'=>out.push(11),
                    b'0'=> {
                        if self.data.get(self.pos).is_some_and(u8::is_ascii_digit) { return Err("watch_literal_octal_unsupported"); }
                        out.push(0);
                    }
                    b'x'=> {
                        let c=char::from_u32(self.hex(2)?).ok_or("watch_literal_escape_invalid")?;
                        let mut buf=[0;4]; out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                    }
                    b'u'=> {
                        let c=self.unicode()?; let mut buf=[0;4];
                        out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                    }
                    b'\n'=>{},
                    b'\r'=> { if self.data.get(self.pos)==Some(&b'\n') { self.pos+=1; } }
                    _=>return Err("watch_literal_escape_unsupported"),
                }
            } else {
                if b<32 { return Err("watch_literal_raw_control"); }
                out.push(b);
            }
            if out.len()>self.max { return Err("watch_json_size_limit"); }
        }
    }
    fn json_string(&mut self,value: &str) -> Result<(), &'static str> {
        self.emit(b"\"")?;
        const HEX:&[u8]=b"0123456789abcdef";
        for &b in value.as_bytes() {
            match b {
                b'"'=>self.emit(b"\\\"")?, b'\\'=>self.emit(b"\\\\")?,
                0..=31=>self.emit(&[b'\\',b'u',b'0',b'0',HEX[(b>>4) as usize],HEX[(b&15) as usize]])?,
                _=>self.emit(&[b])?,
            }
        }
        self.emit(b"\"")
    }
    fn key(&mut self) -> Result<String, &'static str> {
        match self.data.get(self.pos).copied() {
            Some(b'\''|b'"')=>self.string(),
            Some(b) if identifier_start(b)=> {
                let start=self.pos; self.pos+=1;
                while self.data.get(self.pos).is_some_and(|b| identifier_part(*b)) { self.pos+=1; }
                // ASCII only; no escaped/computed keys or execution semantics.
                Ok(String::from_utf8(self.data[start..self.pos].to_vec()).map_err(|_| "watch_literal_key_unsupported")?)
            }
            _=>Err("watch_literal_key_unsupported"),
        }
    }
    fn value(&mut self,protect: bool,depth: usize) -> Result<(), &'static str> {
        if depth>MAX_DEPTH { return Err("watch_json_depth_limit"); }
        self.space()?;
        match self.data.get(self.pos).copied().ok_or("watch_literal_value_missing")? {
            b'\''|b'"'=> { let text=self.string()?; self.json_string(&text)?; }
            b'{'=> {
                self.pos+=1; self.emit(b"{")?; self.space()?;
                if self.data.get(self.pos)==Some(&b'}') { self.pos+=1; return self.emit(b"}"); }
                loop {
                    self.space()?; let key=self.key()?;
                    if key=="__proto__" { return Err("watch_literal_prototype_unsupported"); }
                    let protect_child=protect||protected(&key);
                    self.json_string(&key)?; drop(key); self.space()?;
                    if self.data.get(self.pos)!=Some(&b':') { return Err("watch_literal_colon_missing"); }
                    self.pos+=1; self.emit(b":")?; self.value(protect_child,depth+1)?; self.space()?;
                    match self.data.get(self.pos) {
                        Some(b'}')=> { self.pos+=1; return self.emit(b"}"); }
                        Some(b',')=> {
                            self.pos+=1; self.space()?;
                            if self.data.get(self.pos)==Some(&b'}') { self.pos+=1; return self.emit(b"}"); }
                            self.emit(b",")?;
                        }
                        _=>return Err("watch_literal_object_separator"),
                    }
                }
            }
            b'['=> {
                self.pos+=1; self.emit(b"[")?; self.space()?;
                if self.data.get(self.pos)==Some(&b']') { self.pos+=1; return self.emit(b"]"); }
                loop {
                    self.value(protect,depth+1)?; self.space()?;
                    match self.data.get(self.pos) {
                        Some(b']')=> { self.pos+=1; return self.emit(b"]"); }
                        Some(b',')=> {
                            self.pos+=1; self.space()?;
                            if self.data.get(self.pos)==Some(&b']') { self.pos+=1; return self.emit(b"]"); }
                            self.emit(b",")?;
                        }
                        _=>return Err("watch_literal_array_separator"),
                    }
                }
            }
            _=> {
                let start=self.pos;
                while self.data.get(self.pos).is_some_and(|b| !b.is_ascii_whitespace() && !matches!(*b,b','|b']'|b'}'|b'/')) { self.pos+=1; }
                let token=&self.data[start..self.pos];
                match token {
                    b"true"|b"false"|b"null"=>self.emit(token)?,
                    b"NaN"|b"Infinity"|b"-Infinity"=> {
                        if protect { return Err("watch_config_nonfinite_protected"); }
                        if !self.allow_metadata_nonfinite { return Err("watch_literal_nonfinite_unsupported"); }
                        if self.nonfinite>=MAX_NONFINITE { return Err("watch_config_nonfinite_limit"); }
                        self.nonfinite+=1; self.emit(b"null")?;
                    }
                    b"undefined"=>return Err("watch_config_js_undefined"),
                    _=> {
                        if matches!(token.first(),Some(b'0'..=b'9'|b'-')) {
                            // Validate one complete JSON number without a Value tree.
                            if serde_json::from_slice::<serde::de::IgnoredAny>(token).is_err() { return Err("watch_literal_number_unsupported"); }
                            self.emit(token)?;
                        } else {
                            return Err(match self.data.get(start) {
                                Some(b'`')=>"watch_config_js_template",
                                Some(b'/')=>"watch_config_js_comment_or_regex",
                                Some(b) if identifier_start(*b)=>"watch_config_js_identifier",
                                _=>"watch_config_js_expression",
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

fn normalize(data:&[u8],max:usize,allow_metadata_nonfinite:bool) -> Result<Normalized<'_>, &'static str> {
    if data.len()>max { return Err("watch_json_size_limit"); }
    let body=trivia(data)?;
    let end=frame_end(body,max)?;
    if !trivia(&body[end..])?.is_empty() { return Err("watch_literal_expression_unsupported"); }
    if serde_json::from_slice::<serde::de::IgnoredAny>(data).is_ok() {
        return Ok(Normalized {json:Cow::Borrowed(data),syntax_normalized:false,nonfinite_ignored:false});
    }
    let mut reader=Reader {data,pos:0,output:Vec::with_capacity(max),max,allow_metadata_nonfinite,nonfinite:0};
    reader.value(false,0)?; reader.space()?;
    if reader.pos!=data.len() { return Err("watch_literal_expression_unsupported"); }
    // The output must be strict JSON; no unvalidated serializer shortcut.
    serde_json::from_slice::<serde::de::IgnoredAny>(&reader.output).map_err(|_| "watch_literal_output_invalid")?;
    Ok(Normalized {json:Cow::Owned(reader.output),syntax_normalized:true,nonfinite_ignored:reader.nonfinite>0})
}
pub fn config(data:&[u8],max:usize) -> Result<Normalized<'_>, &'static str> { normalize(data,max,true) }
pub fn player(data:&[u8],max:usize) -> Result<Normalized<'_>, &'static str> { normalize(data,max,false) }

#[cfg(test)]
mod tests {
    use super::*;
    fn value(data:&[u8]) -> serde_json::Value { serde_json::from_slice(&config(data,4096).unwrap().json).unwrap() }
    #[test] fn strings_keys_escapes_and_nested_literals_are_data_not_code() {
        let v=value(br#"{'VISITOR_DATA':'opaque_123',LOGGED_IN:false,IGNORE:{text:'I\'m "quoted" } ];',unicode:'\u2764\uFE0F',hex:'\x41',pair:'\uD83D\uDE0A',},list:[true,null,1.25,],}"#);
        assert_eq!(v["VISITOR_DATA"],"opaque_123"); assert_eq!(v["LOGGED_IN"],false);
        assert_eq!(v["IGNORE"]["text"],"I'm \"quoted\" } ];");
        assert_eq!(v["IGNORE"]["unicode"],"❤️"); assert_eq!(v["IGNORE"]["pair"],"😊");
        assert_eq!(v["IGNORE"]["hex"],"A");
    }
    #[test] fn comments_trailing_commas_and_line_continuations_do_not_execute() {
        assert_eq!(value(b"{/* head */x:'a\\\nb',// line\ny:[1,2,],}")["x"],"ab");
        assert!(config(b"{x:[1,,2]}",1024).is_err());
        assert!(config(b"{x:1 /* unfinished}",1024).is_err());
    }
    #[test] fn strict_json_is_borrowed_unchanged() {
        let raw=br#"{"x":"NaN, Infinity, undefined, \\" ,"n":1.25,"VISITOR_DATA":"opaque"}"#;
        let out=config(raw,1024).unwrap(); assert!(matches!(out.json,Cow::Borrowed(_)));
        assert!(!out.syntax_normalized); assert!(!out.nonfinite_ignored); assert_eq!(out.json.as_ref(),raw);
    }
    #[test] fn nonfinite_only_in_discarded_metadata_and_never_in_strings() {
        let out=config(br#"{x:[NaN,Infinity,-Infinity],VISITOR_DATA:'Infinity'}"#,1024).unwrap();
        assert!(out.nonfinite_ignored);
        let v:serde_json::Value=serde_json::from_slice(&out.json).unwrap();
        assert_eq!(v["x"],serde_json::json!([null,null,null])); assert_eq!(v["VISITOR_DATA"],"Infinity");
        for raw in [br#"{'VISITOR_DATA':NaN}"#.as_slice(),br#"{'LOGGED_IN':Infinity}"#,
            br#"{INNERTUBE_CONTEXT:{x:NaN}}"#,br#"{'\u0056ISITOR_DATA':NaN}"#] {
            assert_eq!(config(raw,1024).err(),Some("watch_config_nonfinite_protected"));
        }
        assert_eq!(player(b"{x:NaN}",1024).err(),Some("watch_literal_nonfinite_unsupported"));
    }
    #[test] fn duplicate_keys_are_preserved_for_the_typed_checker() {
        let out=config(b"{'LOGGED_IN':false,LOGGED_IN:true}",1024).unwrap();
        assert_eq!(std::str::from_utf8(&out.json).unwrap().matches("LOGGED_IN").count(),2);
    }
    #[test] fn expressions_accessors_spread_templates_and_prototypes_are_rejected() {
        for raw in [b"{x:secret()}".as_slice(),b"{x:'a'+'b'}",b"{x:window.value}",
            b"{x:`${secret}`}",b"{x:/regex/}",b"{x:undefined}",b"{...secret}",b"{[secret]:1}",
            b"{get x(){return 1}}",b"{__proto__:{LOGGED_IN:false}}",b"{x:Infinity+1}",
            b"{x:NaN} + secret",b"{x:1n}",b"{x:0x10}"] { assert!(config(raw,1024).is_err()); }
    }
    #[test] fn invalid_escapes_surrogates_raw_controls_and_utf8_are_rejected() {
        for raw in [br#"{x:'\uD800'}"#.as_slice(),br#"{x:'\uDC00'}"#,br#"{x:'\xGG'}"#,
            br#"{x:'\1'}"#,br#"{x:'\07'}"#,br#"{x:'\z'}"#,b"{x:'raw\nline'}",b"{x:'\xff'}"] {
            assert!(config(raw,1024).is_err());
        }
    }
    #[test] fn control_escapes_are_preserved_as_data_not_sanitized_away() {
        let v=value(br#"{VISITOR_DATA:'bad\r\nvalue',x:'\0\v\b\f\t'}"#);
        assert_eq!(v["VISITOR_DATA"],"bad\r\nvalue"); // header guard rejects this later
        assert_eq!(v["x"],"\0\u{000b}\u{0008}\u{000c}\t");
    }
    #[test] fn input_output_depth_and_constant_budgets_are_enforced() {
        assert_eq!(config(b"{}",1).err(),Some("watch_json_size_limit"));
        assert_eq!(config(b"{x:'\\0'}",9).err(),Some("watch_json_size_limit"));
        let deep=format!("{{x:{}null{}}}","[".repeat(66),"]".repeat(66));
        assert!(config(deep.as_bytes(),4096).is_err());
        let many=format!("{{x:[{}]}}",vec!["NaN";257].join(","));
        assert_eq!(config(many.as_bytes(),4096).err(),Some("watch_config_nonfinite_limit"));
    }
    #[test] fn framing_handles_quotes_escaped_braces_comments_and_limits() {
        let data=br#"{'x':'} [ \' //',/* } */ y:[1,2]} ); next"#;
        let end=frame_end(data,1024).unwrap(); assert!(data[end..].starts_with(b" );"));
        assert_eq!(frame_end(b"{x:[}",1024),Err("watch_json_unbalanced"));
        assert_eq!(frame_end(b"{'x':'missing}",1024),Err("watch_json_unterminated"));
        assert_eq!(frame_end(b"{abc:1}",4),Err("watch_json_size_limit"));
        assert!(trivia(b"/* private").is_err());
    }
    #[test] fn deterministic_string_corpus_matches_independent_json_values() {
        let samples=["plain","single ' and double \"","backslash \\ end","} [ ; // /* */",
            "Unicode ❤️ 😊","new\nline","tab\tzero\0","literal NaN Infinity undefined"];
        for (n,text) in samples.iter().enumerate() {
            let quoted=text.replace('\\',"\\\\").replace('\'',"\\'")
                .replace('\n',"\\n").replace('\t',"\\t").replace('\0',"\\0");
            let data=format!("{{ignored:'{quoted}',n:{n},array:[true,false,null,],}}");
            let parsed=value(data.as_bytes());
            assert_eq!(parsed,serde_json::json!({"ignored":text,"n":n,"array":[true,false,null]}));
        }
    }
    #[test] fn private_input_never_appears_in_static_errors() {
        for raw in [b"{x:PRIVATE.secret()}".as_slice(),b"{PRIVATE:undefined}",b"{x:'\\PRIVATE'}"] {
            let error=config(raw,1024).err().unwrap(); assert!(!error.contains("PRIVATE"));
        }
    }
}
