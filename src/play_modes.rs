//! Bounded, validated playback preferences. No effect on playback clock or archives.
use serde::{Deserialize,Serialize};
use std::path::Path;
#[derive(Debug,Clone,Copy,Default,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="snake_case")]
pub enum Repeat { #[default] Off, All, One }
#[derive(Debug,Clone,Copy,Default,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Modes { pub shuffle:bool, pub repeat:Repeat }
impl Modes {
    pub fn open(path:Option<&Path>)->Self {
        path.and_then(|p|std::fs::metadata(p).ok().filter(|m|m.len()<=2048).and_then(|_|std::fs::read(p).ok()))
            .and_then(|b|serde_json::from_slice(&b).ok()).unwrap_or_default()
    }
    pub fn save(&self,path:Option<&Path>)->std::io::Result<()> {
        if let Some(path)=path {
            let temp=path.with_extension("pending");
            let result=(|| {std::fs::write(&temp,serde_json::to_vec(self).map_err(std::io::Error::other)?)?;std::fs::File::open(&temp)?.sync_all()?;std::fs::rename(&temp,path)})();
            if result.is_err(){let _=std::fs::remove_file(temp);}result
        }else{Ok(())}
    }
}
pub fn pick_index(len:usize,shuffle:bool,entropy:usize)->usize {if shuffle && len>1 {entropy%len}else{0}}
/// Non-security random choice using the standard library's independently seeded hasher.
pub fn entropy()->usize {
    use std::hash::BuildHasher;
    std::collections::hash_map::RandomState::new().hash_one(std::time::SystemTime::now()) as usize
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn modes_reject_unknown_and_invalid_values_and_default_safely(){
        assert_eq!(Modes::open(None),Modes::default());
        for bad in [r#"{"shuffle":true,"repeat":"forever"}"#,r#"{"shuffle":"yes","repeat":"off"}"#,r#"{"shuffle":true,"repeat":"all","url":"http://evil"}"#]{assert!(serde_json::from_str::<Modes>(bad).is_err());}
        let path=std::env::temp_dir().join(format!("aysha-modes-{}.json",std::process::id()));
        let m=Modes{shuffle:true,repeat:Repeat::One};m.save(Some(&path)).unwrap();assert_eq!(Modes::open(Some(&path)),m);
        std::fs::write(&path,"bad").unwrap();assert_eq!(Modes::open(Some(&path)),Modes::default());let _=std::fs::remove_file(path);
    }
    #[test] fn ordered_and_shuffled_selection_is_bounded(){for i in 0..100 {assert_eq!(pick_index(4,false,i),0);assert!(pick_index(4,true,i)<4);}assert_eq!(pick_index(0,true,5),0);}
}
