//! First Rust settings milestone: bounded palette validation and persistence.
//! None means this installation has not migrated its WebView preference yet.
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorTheme { Crimson, Charcoal, Aurora }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemePreference { pub theme: ColorTheme }

impl ThemePreference {
    pub fn open(path: Option<&Path>) -> Option<Self> {
        path.and_then(|p| std::fs::metadata(p).ok().filter(|m| m.is_file() && m.len() <= 256).and_then(|_| std::fs::read(p).ok()))
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
    }
    pub fn save(&self, path: Option<&Path>) -> std::io::Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validated_bounded_preference_roundtrips_and_recovers() {
        for bad in [r#"{"theme":"ultraviolet"}"#, r#"{"theme":"aurora","extra":true}"#, r#"{"theme":3}"#] {
            assert!(serde_json::from_str::<ThemePreference>(bad).is_err());
        }
        let path = std::env::temp_dir().join(format!("aysha-theme-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        assert_eq!(ThemePreference::open(Some(&path)), None);
        let theme = ThemePreference { theme: ColorTheme::Aurora };
        theme.save(Some(&path)).unwrap();
        assert_eq!(ThemePreference::open(Some(&path)), Some(theme));
        std::fs::write(&path, vec![b'x'; 257]).unwrap();
        assert_eq!(ThemePreference::open(Some(&path)), None);
        let _ = std::fs::remove_file(path);
    }
}
