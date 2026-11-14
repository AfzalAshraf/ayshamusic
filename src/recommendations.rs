//! Explicit mood + simple preference ranking, never '<current title> similar songs'.
//! This is metadata-based selection, not audio analysis or a trained recommender.
use crate::android_catalog::Track;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Mood { #[default] Romantic, Melody }
impl Mood {
    pub fn query(self) -> &'static str {
        match self { Self::Romantic => "romantic love melody songs", Self::Melody => "soft soothing melody songs" }
    }
}
fn words(value: &str) -> Vec<String> {
    value.to_lowercase().split(|ch: char| !ch.is_alphanumeric()).filter(|word| !word.is_empty()).map(str::to_owned).collect()
}
fn song_key(title: &str) -> Vec<String> {
    let segment = title.split(['|', '(', '[', ':']).next().unwrap_or(title);
    let tokens = words(segment);
    let mut key = Vec::new();
    for token in tokens {
        if matches!(token.as_str(), "official" | "full" | "lyrics" | "lyrical" | "audio" | "video" | "cover" | "remix" | "reprise" | "slowed" | "lofi" | "acoustic" | "feat" | "ft" | "version") { break; }
        if token != "song" { key.push(token); }
    }
    key.truncate(12); key
}
pub fn same_song(a: &Track, b: &Track) -> bool {
    if a.id == b.id { return true; }
    let left = song_key(&a.title); let right = song_key(&b.title);
    if left.is_empty() || right.is_empty() { return false; }
    if left == right { return true; }
    // Also catch artist-prefixed alternate uploads and translated suffix labels.
    let contains = |short: &[String], long: &[String]| short.len() >= 2 && long.windows(short.len()).any(|w| w == short);
    contains(&left, &words(&b.title)) || contains(&right, &words(&a.title))
}
pub fn select(candidates: Vec<Track>, current: &Track, history: &[Track], liked: &[Track], mood: Mood) -> Option<Track> {
    let preferred: HashSet<String> = liked.iter().map(|t| t.artist.trim().to_lowercase()).filter(|s| !s.is_empty()).collect();
    let mut seen = HashSet::new();
    let mut ranked: Vec<(i32, usize, Track)> = candidates.into_iter().enumerate()
        .filter(|(_, t)| seen.insert(t.id.clone()) && !same_song(current, t) && !history.iter().rev().take(50).any(|old| same_song(old, t)))
        .map(|(index, track)| {
            let mut score = if preferred.contains(&track.artist.trim().to_lowercase()) { 2 } else { 0 };
            let title = track.title.to_lowercase();
            let cues: &[&str] = match mood { Mood::Romantic => &["romantic", "love"], Mood::Melody => &["melody", "soothing", "soft"] };
            score += cues.iter().filter(|cue| title.contains(**cue)).count() as i32;
            (score, index, track)
        }).collect();
    ranked.sort_by(|a,b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    ranked.into_iter().next().map(|(_,_,t)| t)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn track(id:&str,title:&str,artist:&str)->Track { Track { id:id.into(),title:title.into(),artist:artist.into(),thumbnail:String::new(),url:String::new() } }
    #[test]
    fn query_uses_only_mood_not_current_song_or_private_like_titles() {
        assert_eq!(Mood::Romantic.query(), "romantic love melody songs");
        assert_eq!(Mood::Melody.query(), "soft soothing melody songs");
        assert!(!Mood::Romantic.query().contains("Paniyon"));
    }
    #[test]
    fn same_name_variants_are_rejected_across_uploads_and_artist_prefixes() {
        let current = track("abcdefghijk", "PANIYON SA Full Song | Satyameva Jayate | John Abraham", "T-Series");
        for title in ["Paniyon Sa (Lyrics)","PANIYON SA - slowed + reverb", "Atif Aslam - Paniyon Sa Official Audio", "Paniyon Sa Remix", "Paniyon Sa Cover"] {
            assert!(same_song(&current, &track("12345678901",title,"Other")), "{title}");
        }
        assert!(!same_song(&track("a","Love Story","x"), &track("b","Love Me Again","x")));
    }
    #[test]
    fn selection_skips_same_song_recent_history_and_prefers_liked_channel() {
        let current=track("abcdefghijk","Paniyon Sa Full Song","A");
        let recent=track("zzzzzzzzzzz","Tum Hi Ho","A");
        let candidates=vec![track("11111111111","Paniyon Sa Lyrics","A"),track("22222222222","Tum Hi Ho Cover","A"),track("33333333333","New Melody","B"),track("44444444444","Different Romantic Song","Favorite")];
        let chosen=select(candidates,&current,&[recent],&[track("x","Private favorite","Favorite")],Mood::Romantic).unwrap();
        assert_eq!(chosen.id,"44444444444");
    }
    #[test]
    fn no_replacement_is_better_than_replaying_same_song() {
        let current=track("abcdefghijk","Paniyon Sa Full Song","A");
        assert!(select(vec![track("11111111111","Paniyon Sa Lyrics","A")],&current,&[],&[],Mood::Melody).is_none());
    }
}
