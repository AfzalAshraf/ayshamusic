//! Bounded, private persistence for explicit comfort-quote choices.
use serde::{Deserialize, Serialize};
use std::{collections::{BTreeMap, HashSet}, path::Path};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuotePreferences {
    pub version: u8,
    pub saved: Vec<String>,
    pub more: BTreeMap<String, u8>,
    pub personalize: bool,
}
impl QuotePreferences {
    pub fn valid(&self) -> bool {
        if self.version != 1 || self.saved.len() > 24 || self.more.len() > 4 { return false; }
        let mut seen = HashSet::new();
        for id in &self.saved {
            let Some((category, number)) = id.rsplit_once('-') else { return false };
            if !matches!(category,"love"|"rest"|"kindness"|"courage") || !matches!(number,"1"|"2"|"3"|"4"|"5"|"6") || !seen.insert(id) { return false; }
        }
        self.more.iter().all(|(category, count)| matches!(category.as_str(),"love"|"rest"|"kindness"|"courage") && *count <= 8)
    }
    pub fn open(path: Option<&Path>) -> Option<Self> {
        let path = path?;
        let metadata = std::fs::metadata(path).ok()?;
        if !metadata.is_file() || metadata.len() > 8192 { return None; }
        let value: Self = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
        value.valid().then_some(value)
    }
    pub fn save(&self, path: Option<&Path>) -> std::io::Result<()> {
        if !self.valid() { return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput,"Invalid quote preferences")); }
        if let Some(path) = path {
            let temp = path.with_extension("pending");
            let result = (|| {
                std::fs::write(&temp, serde_json::to_vec(self).map_err(std::io::Error::other)?)?;
                std::fs::File::open(&temp)?.sync_all()?;
                std::fs::rename(&temp, path)
            })();
            if result.is_err() { let _ = std::fs::remove_file(temp); }
            result
        } else { Ok(()) }
    }
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn bounded_validation_roundtrip_and_corruption() {
        let path=std::env::temp_dir().join(format!("aysha-quote-{}.json",std::process::id()));
        let _=std::fs::remove_file(&path);
        assert_eq!(QuotePreferences::open(Some(&path)),None);
        let mut value=QuotePreferences{version:1,saved:vec!["love-1".into(),"rest-6".into()],more:BTreeMap::from([("love".into(),8)]),personalize:true};
        value.save(Some(&path)).unwrap();assert_eq!(QuotePreferences::open(Some(&path)),Some(value.clone()));
        value.saved.push("love-1".into());assert!(!value.valid());
        value.saved.pop();value.more.insert("love".into(),9);assert!(!value.valid());
        value.more.clear();value.saved.push("love-7".into());assert!(!value.valid());
        assert!(serde_json::from_str::<QuotePreferences>(r#"{"version":1,"saved":[],"more":{},"personalize":false,"other":1}"#).is_err());
        std::fs::write(&path,vec![b'x';8193]).unwrap();assert_eq!(QuotePreferences::open(Some(&path)),None);
        let _=std::fs::remove_file(path);
    }
}
