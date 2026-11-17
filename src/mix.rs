//! Bounded metadata-only YouTube radio queue. No audio prefetch or ranking.
use crate::{android_catalog::Track,recommendations::same_song};
use std::time::{Duration,Instant};
#[derive(Default)]
pub struct Mix {
    pub generation:u64,
    pub seed:String,
    pub tracks:Vec<Track>,
    fetched:Option<Instant>,
}
impl Mix {
    pub fn reset(&mut self,seed:&str) {self.generation=self.generation.wrapping_add(1);self.seed=seed.into();self.tracks.clear();self.fetched=None;}
    pub fn fresh(&self)->bool {self.fetched.is_some_and(|t|t.elapsed()<Duration::from_secs(1800))}
    pub fn fill(&mut self,generation:u64,tracks:Vec<Track>) {
        if self.generation!=generation{return;}
        self.tracks=tracks.into_iter().take(40).collect();self.fetched=Some(Instant::now());
    }
    pub fn choose(&self,current:&Track,history:&[Track])->Option<Track> {
        self.tracks.iter().find(|t|!same_song(current,t)&&!history.iter().rev().take(50).any(|old|same_song(old,t))).cloned()
    }
    pub fn consumed(&mut self,track:&Track) {self.tracks.retain(|t|!same_song(t,track));}
}
#[cfg(test)] mod tests {
    use super::*;
    fn t(id:&str,title:&str)->Track {Track{id:id.into(),title:title.into(),artist:String::new(),thumbnail:String::new(),url:String::new()}}
    #[test] fn preserves_provider_order_and_never_repeats_current_history_or_variants() {
        let mut m=Mix::default();m.reset("abcdefghijk");
        let current=t("abcdefghijk","Asalayavale - Lyric Video | Khalifa");
        let old=t("history0001","Old melody");
        m.fill(m.generation,vec![current.clone(),t("alternate01","Asalayavale Official Video"),old.clone(),t("nexttrack01","A new love"),t("nexttrack02","Another melody")]);
        assert_eq!(m.choose(&current,&[old]).unwrap().id,"nexttrack01");
        let next=m.choose(&current,&[]).unwrap();m.consumed(&next);
        assert!(!m.tracks.iter().any(|t|t.id==next.id));
    }
    #[test] fn stale_prefetch_cannot_replace_new_seed_and_empty_mix_is_not_replay() {
        let mut m=Mix::default();m.reset("oldseed0001");let old=m.generation;
        m.reset("newseed0001");m.fill(old,vec![t("abcdefghijk","Wrong mix")]);
        assert!(m.tracks.is_empty());assert!(!m.fresh());
        m.fill(m.generation,vec![]);assert!(m.fresh());assert!(m.choose(&t("newseed0001","Seed"),&[]).is_none());
    }
}
