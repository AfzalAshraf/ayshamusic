//! Non-recursive, bounded scan of a folder explicitly chosen by the user.
use serde_json::{json,Value};
use std::path::Path;

pub fn scan(folder:&Path)->Result<Value,String> {
    let entries=std::fs::read_dir(folder).map_err(|_|"local_folder_unavailable".to_owned())?;
    let mut tracks=Vec::new();
    let mut scanned=0usize;
    let mut truncated=false;
    for entry in entries {
        if scanned>=5000 {truncated=true;break;}
        scanned+=1;
        let Ok(entry)=entry else {continue};
        let Ok(kind)=entry.file_type() else {continue};
        if !kind.is_file(){continue;}
        let path=entry.path();
        if !path.extension().and_then(|s|s.to_str()).is_some_and(|s|["m4a","mp3","wav","flac","ogg","webm"].iter().any(|ext|s.eq_ignore_ascii_case(ext))){continue;}
        let Some(path_text)=path.to_str() else {continue};
        let name=path.file_stem().and_then(|s|s.to_str()).unwrap_or("Local song").chars().filter(|c|!c.is_control()).take(200).collect::<String>();
        if tracks.len()>=500 {truncated=true;break;}
        tracks.push(json!({"name":name,"path":path_text}));
    }
    tracks.sort_by_key(|v|v["name"].as_str().unwrap_or("").to_lowercase());
    Ok(json!({"items":tracks,"truncated":truncated}))
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn scan_only_chosen_folders_supported_extensions() {
        let dir=std::env::temp_dir().join(format!("aysha-library-test-{}",std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("B.M4A"),b"x").unwrap();
        std::fs::write(dir.join("a.m4a"),b"x").unwrap();
        std::fs::write(dir.join("c.MP3"),b"x").unwrap();
        std::fs::write(dir.join("d.wav"),b"x").unwrap();
        std::fs::write(dir.join("e.flac"),b"x").unwrap();
        std::fs::write(dir.join("f.ogg"),b"x").unwrap();
        std::fs::write(dir.join("g.webm"),b"x").unwrap();
        std::fs::write(dir.join("ignore.mp4"),b"x").unwrap();
        std::fs::create_dir(dir.join("nested.m4a")).unwrap();
        let list=scan(&dir).unwrap();let items=list["items"].as_array().unwrap();
        assert_eq!(items.len(),7);assert_eq!(items[0]["name"],"a");assert_eq!(items[1]["name"],"B");
        std::fs::remove_dir_all(&dir).unwrap();assert!(scan(&dir).is_err());
    }
}
