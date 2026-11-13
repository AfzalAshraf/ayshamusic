//! Optional, value-free watch-only inventory. This is NOT a JS lexer/parser.
//! Raw marker hits may be in strings, comments or dead code. They never supply
//! visitor/auth data, relax parsing, or authorize a downstream request.
use serde::Serialize;

#[derive(Default, Serialize)]
pub struct Regions {
    inspected: usize,
    nonempty: usize,
    with_raw_ytcfg_name: usize,
    with_raw_visitor_name: usize,
}
impl Regions {
    fn inspect(&mut self, data: &[u8]) {
        self.inspected += 1;
        self.nonempty += usize::from(data.iter().any(|b| !b.is_ascii_whitespace()));
        self.with_raw_ytcfg_name += usize::from(contains(data,b"ytcfg"));
        self.with_raw_visitor_name += usize::from(contains(data,b"VISITOR_DATA") || contains(data,b"visitorData"));
    }
}
#[derive(Default, Serialize)]
pub struct Inventory {
    ignored_tails: Regions,
    skipped_attribute_bodies: Regions,
    declaration_prefixes: usize,
    parenthesis_prefixes: usize,
    window_prefixes: usize,
    other_prefixes: usize,
}
fn contains(data: &[u8], marker: &[u8]) -> bool {
    data.windows(marker.len()).any(|w| w == marker)
}
fn word(data: &[u8], token: &[u8]) -> bool {
    data.strip_prefix(token).is_some_and(|rest| !rest.first().is_some_and(|b|
        b.is_ascii_alphanumeric() || matches!(b,b'_'|b'$')))
}
impl Inventory {
    // Called only on an already ignored nonempty tail, after parser trivia.
    // No search result is passed back into the data reader.
    pub fn ignored(&mut self, data: &[u8]) {
        self.ignored_tails.inspect(data);
        if [b"var".as_slice(),b"let",b"const",b"function"].iter().any(|w| word(data,w)) {
            self.declaration_prefixes += 1;
        } else if data.first() == Some(&b'(') { self.parenthesis_prefixes += 1; }
        else if word(data,b"window") { self.window_prefixes += 1; }
        else { self.other_prefixes += 1; }
    }
    pub fn skipped_attributes(&mut self, body: &[u8]) {
        self.skipped_attribute_bodies.inspect(body);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn coverage_counts_regions_not_occurrences_and_never_serializes_values() {
        let mut inventory=Inventory::default();
        inventory.ignored(b"var PRIVATE_VALUE='visitorData visitorData ytcfg ytcfg';");
        inventory.ignored(b"(function(){if(false){ytcfg.set({VISITOR_DATA:'PRIVATE_VALUE'});}})();");
        inventory.skipped_attributes(b"ytcfg.set({VISITOR_DATA:'PRIVATE_VALUE'});");
        inventory.skipped_attributes(b" \n");
        let report=serde_json::to_value(inventory).unwrap();
        assert_eq!(report["ignored_tails"]["inspected"],2);
        assert_eq!(report["ignored_tails"]["with_raw_visitor_name"],2);
        assert_eq!(report["ignored_tails"]["with_raw_ytcfg_name"],2);
        assert_eq!(report["skipped_attribute_bodies"]["inspected"],2);
        assert_eq!(report["skipped_attribute_bodies"]["nonempty"],1);
        assert_eq!(report["declaration_prefixes"],1); assert_eq!(report["parenthesis_prefixes"],1);
        assert!(!report.to_string().contains("PRIVATE_VALUE"));
    }
    #[test] fn coverage_prefix_buckets_are_only_coarse_hints_and_respect_boundaries() {
        let mut inventory=Inventory::default();
        for tail in [b"window['x']=1".as_slice(),b"windowExtra='x'",b"variable=1",b"let x=1",b"'literal'",b"(x)"] {
            inventory.ignored(tail);
        }
        assert_eq!(inventory.window_prefixes,1); assert_eq!(inventory.declaration_prefixes,1);
        assert_eq!(inventory.parenthesis_prefixes,1); assert_eq!(inventory.other_prefixes,3);
    }
    #[test] fn coverage_raw_markers_are_not_syntax_or_absence_proof() {
        let mut inventory=Inventory::default();
        inventory.ignored(br#"'ytcfg VISITOR_DATA' /* visitorData */"#);
        inventory.ignored(br#"window['VISITOR_\u0044ATA']='opaque'"#);
        assert_eq!(inventory.ignored_tails.with_raw_visitor_name,1);
        // Escaped names may be missed, and inert strings may match. No claims
        // about executable syntax, schema data, validity or the whole page.
        assert_eq!(inventory.ignored_tails.inspected,2);
    }
    #[test] fn coverage_large_region_is_scanned_without_storing_it() {
        let mut inventory=Inventory::default();
        let mut body=vec![b'x';1024*1024];body.extend_from_slice(b"visitorData");
        inventory.skipped_attributes(&body);drop(body);
        let report=serde_json::to_string(&inventory).unwrap();
        assert!(report.len()<512);
        assert_eq!(inventory.skipped_attribute_bodies.with_raw_visitor_name,1);
    }
}
