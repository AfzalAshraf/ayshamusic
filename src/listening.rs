//! Bounded on-phone listening preferences. No telemetry/upload or invented plays.
//! Only actual player progress and explicit likes/skips update the local model.
use crate::android_catalog::Track;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::{self, OpenOptions}, io::{Read, Write}, path::PathBuf};

const LIMIT: usize = 300;
#[derive(Clone, Default, Serialize, Deserialize)]
struct Entry { id:String, title:String, artist:String, seconds:f64, skips:u32, completions:u32, liked:bool }
#[derive(Default, Serialize, Deserialize)]
struct Model { entries:Vec<Entry> }
struct Session { id:String, position:f64, heard:f64, duration:f64, completed:bool }
pub struct Listening { model:Model, path:Option<PathBuf>, active:Option<Session>, dirty:u8, pub durable:bool }
#[derive(Clone, Default)]
pub struct Preferences { pub styles:HashMap<&'static str,f64>, pub channels:HashMap<String,f64>, pub observations:usize }
fn style(track:&Track)->Option<&'static str> {
    let text=format!("{} {}",track.title,track.artist).to_lowercase();
    // Metadata cues only: do not pretend that a label/channel is an audio genre.
    [ ("devotional", &["devotional","bhajan","qawwali","worship"][..]),
      ("dance", &["dance","party","club","edm","workout"][..]),
      ("romantic", &["romantic","love song","pyaar","pyar","ishq","mohabbat"][..]),
      ("melody", &["melody","melodious","soothing","soft song","acoustic"][..]),
      ("instrumental", &["instrumental","piano","lofi","lo-fi","ambient"][..]) ]
      .into_iter().find(|(_,cues)|cues.iter().any(|cue|text.contains(cue))).map(|(s,_)|s)
}
impl Preferences {
    pub fn query(&self)->String {
        if let Some((style,_))=self.styles.iter().filter(|(_,w)|**w>0.0).max_by(|a,b|a.1.total_cmp(b.1).then_with(||a.0.cmp(b.0))) {
            return format!("{style} songs music");
        }
        if let Some((channel,_))=self.channels.iter().filter(|(_,w)|**w>0.0).max_by(|a,b|a.1.total_cmp(b.1).then_with(||a.0.cmp(b.0))) {
            return format!("{} songs",channel.chars().take(100).collect::<String>());
        }
        "music songs discovery".into()
    }
    pub fn score(&self,track:&Track)->f64 {
        style(track).and_then(|s|self.styles.get(s)).copied().unwrap_or(0.0)
          +self.channels.get(&track.artist.to_lowercase()).copied().unwrap_or(0.0)*0.5
    }
}
impl Listening {
    pub fn open(path:Option<PathBuf>)->Self {
        let model=path.as_ref().and_then(|p| {
            let mut bytes=Vec::new(); fs::File::open(p).ok()?.take(1024*1024+1).read_to_end(&mut bytes).ok()?;
            if bytes.len()>1024*1024 {return None;}
            let model:Model=serde_json::from_slice(&bytes).ok()?;
            (model.entries.len()<=LIMIT && model.entries.iter().all(|e|crate::youtube::valid_video_id(&e.id)&&e.title.chars().count()<=300&&e.artist.chars().count()<=150&&e.seconds.is_finite()&&(0.0..=1e9).contains(&e.seconds))).then_some(model)
        }).unwrap_or_default();
        let durable=path.as_ref().is_some_and(|p|p.exists()); Self {model,path,active:None,dirty:0,durable}
    }
    fn entry(&mut self,track:&Track)->&mut Entry {
        if let Some(index)=self.model.entries.iter().position(|e|e.id==track.id) {return &mut self.model.entries[index];}
        if self.model.entries.len()>=LIMIT {self.model.entries.remove(0);}
        self.model.entries.push(Entry{id:track.id.clone(),title:track.title.chars().take(300).collect(),artist:track.artist.chars().take(150).collect(),..Entry::default()});
        self.model.entries.last_mut().unwrap()
    }
    /// Called by a Rust timer with the app's actual player snapshot, never UI time.
    pub fn observe(&mut self,track:&Track,player:&serde_json::Value) {
        if player["video_id"].as_str()!=Some(track.id.as_str()) || player["error"].as_str().is_some() {return;}
        let position=player["position_secs"].as_f64().filter(|p|p.is_finite()&&*p>=0.0).unwrap_or(0.0);
        let duration=player["duration_secs"].as_f64().filter(|d|d.is_finite()&&*d>0.0).unwrap_or(0.0);
        if player["state"]!="playing" {
            if player["state"]=="paused" { if let Some(session)=&mut self.active {session.position=position;} }
            else {self.active=None;}
            if self.dirty>0 {self.flush();}
            return;
        }
        if self.active.as_ref().map(|s|s.id.as_str())!=Some(track.id.as_str()) {
            // Baseline only: reload/seek/stale polling must not fabricate past listening.
            self.active=Some(Session{id:track.id.clone(),position,heard:0.0,duration,completed:false}); return;
        }
        let session=self.active.as_mut().unwrap();
        let delta=position-session.position; session.position=position;
        if !(0.0..=12.0).contains(&delta) {return;}
        session.heard+=delta;
        let complete=!session.completed && duration>0.0 && session.heard>=duration*0.8;
        session.completed|=complete;
        if delta>0.0 {let e=self.entry(track);e.seconds=(e.seconds+delta).min(1e9);if complete {e.completions=e.completions.saturating_add(1);}self.changed(false);}
    }
    pub fn skip(&mut self,track:&Track) {
        if let Some(session)=&self.active {
            if session.id==track.id && session.heard>0.0 && session.heard<30.0_f64.min(if session.duration>0.0 {session.duration*0.25}else{30.0}) {
                let e=self.entry(track);e.skips=e.skips.saturating_add(1); self.changed(true);
            }
        }
        self.active=None;
    }
    pub fn like(&mut self,track:&Track,value:bool) {self.entry(track).liked=value;self.changed(true);}
    pub fn preferences(&self)->Preferences {
        let mut out=Preferences{observations:self.model.entries.len(),..Preferences::default()};
        for e in &self.model.entries {
            let weight=(e.seconds/30.0).min(20.0)+f64::from(e.completions)*4.0+if e.liked {8.0}else{0.0}-f64::from(e.skips)*3.0;
            if weight<=0.0 {continue;}
            let track=Track{id:e.id.clone(),title:e.title.clone(),artist:e.artist.clone(),thumbnail:String::new(),url:String::new()};
            if let Some(s)=style(&track) {*out.styles.entry(s).or_default()+=weight;}
            if !e.artist.trim().is_empty() {*out.channels.entry(e.artist.to_lowercase()).or_default()+=weight;}
        } out
    }
    pub fn reset(&mut self)->bool {self.model=Model::default();self.active=None;self.changed(true);self.path.is_none()||self.durable}
    fn changed(&mut self,force:bool) {self.dirty=self.dirty.saturating_add(1);if force||self.dirty>=6 {self.flush();}}
    pub fn flush(&mut self) {
        let Some(path)=&self.path else {self.durable=false;return;};
        let result=(||->std::io::Result<()> {
            let bytes=serde_json::to_vec(&self.model)?;
            let tmp=path.with_extension("tmp");
            let mut options=OpenOptions::new();options.create(true).truncate(true).write(true);
            #[cfg(unix)] {use std::os::unix::fs::OpenOptionsExt;options.mode(0o600);}
            let mut file=options.open(&tmp)?;file.write_all(&bytes)?;file.sync_all()?;fs::rename(tmp,path)?;Ok(())
        })();self.durable=result.is_ok();if self.durable {self.dirty=0;}
    }
}
#[cfg(test)]
mod tests {
    use super::*;use serde_json::json;
    fn track()->Track {Track{id:"abcdefghijk".into(),title:"Romantic original".into(),artist:"Artist".into(),thumbnail:String::new(),url:String::new()}}
    fn tick(position:f64,state:&str)->serde_json::Value {json!({"video_id":"abcdefghijk","state":state,"position_secs":position,"duration_secs":100.0})}
    #[test] fn only_playback_progress_learns_not_failed_plays_or_pauses_or_seeks() {
        let mut model=Listening::open(None);let t=track();
        model.observe(&t,&json!({"video_id":t.id,"state":"error","error":"no stream","position_secs":0}));assert_eq!(model.preferences().observations,0);
        model.observe(&t,&tick(0.0,"playing"));model.observe(&t,&tick(5.0,"playing"));
        model.observe(&t,&tick(50.0,"playing"));model.observe(&t,&tick(55.0,"paused"));model.observe(&t,&tick(55.0,"playing"));
        assert_eq!(model.model.entries[0].seconds,5.0);
        assert_eq!(model.preferences().query(),"romantic songs music");
        model.skip(&t);assert_eq!(model.model.entries[0].skips,1);assert!(model.preferences().styles.is_empty());
        model.like(&t,true);assert!(!model.preferences().styles.is_empty());
    }
    #[test] fn profile_persists_privately_and_reset_removes_learned_preferences() {
        let path=std::env::temp_dir().join(format!("aysha-listen-{}-{}.json",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let mut model=Listening::open(Some(path.clone()));model.like(&track(),true);assert!(model.durable);
        let mut loaded=Listening::open(Some(path.clone()));assert_eq!(loaded.preferences().observations,1);
        #[cfg(unix)] {use std::os::unix::fs::PermissionsExt;assert_eq!(fs::metadata(&path).unwrap().permissions().mode()&0o777,0o600);}
        loaded.reset();assert_eq!(Listening::open(Some(path.clone())).preferences().observations,0);let _=fs::remove_file(path);
    }
    #[test] fn no_history_is_honest_discovery_not_a_forced_mood() {assert_eq!(Listening::open(None).preferences().query(),"music songs discovery");}
}
