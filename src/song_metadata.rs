//! Conservative offline metadata cleanup. Never invent artist/album from a title.
use std::{io,path::Path,time::Duration};
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Song {pub title:String,pub artist:Option<String>,pub source:String}
fn bounded(text:&str,max:usize)->String {text.chars().filter(|c|!c.is_control() && !matches!(*c,'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}')).take(max).collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ")}
fn boilerplate(text:&str)->bool {
    let key=text.trim().to_ascii_lowercase();
    matches!(key.as_str(),"official video"|"official music video"|"official audio"|"lyric video"|"lyrics video"|"lyrics"|"lyrical video"|"official lyric video"|"audio"|"video song"|"4k"|"hd"|"1080p"|"full song")
}
pub fn clean(title:&str,source:&str)->Song {
    let raw=bounded(title,300);
    let mut cleaned=String::new();let mut chars=raw.chars().peekable();
    while let Some(c)=chars.next() {
        if c=='[' || c=='(' {
            let close=if c=='[' {']'}else{')'};let mut inside=String::new();let mut closed=false;
            for next in chars.by_ref() {if next==close {closed=true;break;}inside.push(next);}
            if !closed {cleaned.push(c);cleaned.push_str(&inside);}
            else if !boilerplate(&inside) {cleaned.push(c);cleaned.push_str(&inside);cleaned.push(close);}
        }else{cleaned.push(c);}
    }
    // Pipe suffixes generally carry film/cast/promotion, not reliable album data.
    let mut title=cleaned.split('|').next().unwrap_or(&cleaned).trim().to_owned();
    for separator in [" - "," – "," — "] {
        if let Some((head,tail))=title.rsplit_once(separator) {if boilerplate(tail){title=head.trim().into();}}
    }
    for suffix in [" official music video"," official video"," lyric video"," lyrics video"," lyrical video"," official audio"," video song"," full song"] {
        if title.to_ascii_lowercase().ends_with(suffix) {title.truncate(title.len()-suffix.len());title=title.trim().into();}
    }
    title=bounded(&title,180);
    if title.is_empty(){title=if raw.is_empty(){"Untitled song".into()}else{raw};}
    let source=bounded(source,180);
    // Only a structured Topic-channel suffix is used as artist evidence. Label
    // arbitrary uploader names as source, not performer (labels are not singers).
    let artist=source.strip_suffix(" - Topic").map(str::trim).filter(|s|!s.is_empty()).map(str::to_owned);
    Song{title,artist,source}
}
pub fn filename(song:&Song)->String {
    let name=song.artist.as_ref().map(|artist|format!("{artist} - {}",song.title)).unwrap_or_else(||song.title.clone());
    let mut safe=String::new();
    for c in name.chars() {
        if c.is_control() || matches!(c,'/'|'\\'|':'|'*'|'?'|'"'|'<'|'>'|'|') {safe.push(' ');} else {safe.push(c);}
    }
    let safe=safe.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut stem=String::new();for c in safe.trim_matches([' ','.']).chars(){if stem.len()+c.len_utf8()>180{break;}stem.push(c);}
    if stem.is_empty(){stem="Song".into();}
    format!("{}.m4a",stem.trim_end_matches([' ','.']))
}
/// Copy/tag/validate then atomic replacement. Never transcode or alter the source
/// cache pathname/index. A failed tag write leaves the playable original intact.
pub fn tag(path:&Path,song:&Song,cover:Option<&[u8]>)->io::Result<Duration> {
    let suffix=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
    let temp=path.with_extension(format!("tagging-{suffix}"));
    let result=(|| {
        std::fs::copy(path,&temp)?;
        let mut tag=mp4ameta::Tag::read_from_path(&temp).map_err(|_|io::Error::other("archive_tag_read_failed"))?;
        let duration=tag.duration();if duration.is_zero() || duration.as_secs()>604800{return Err(io::Error::other("archive_duration_invalid"));}
        tag.set_title(&song.title);
        if let Some(artist)=&song.artist {tag.set_artist(artist);}
        if let Some(cover)=cover.filter(|c|!c.is_empty() && c.len()<=200_000){tag.set_artwork(mp4ameta::Img::jpeg(cover.to_vec()));}
        tag.write_to_path(&temp).map_err(|_|io::Error::other("archive_tag_write_failed"))?;
        let check=mp4ameta::Tag::read_from_path(&temp).map_err(|_|io::Error::other("archive_tag_verify_failed"))?;
        if check.title()!=Some(song.title.as_str()) || check.duration()!=duration{return Err(io::Error::other("archive_tag_verify_failed"));}
        std::fs::File::open(&temp)?.sync_all()?;
        std::fs::rename(&temp,path)?;Ok(duration)
    })();
    let _=std::fs::remove_file(&temp);result
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn preserves_unicode_and_unknown_performer_instead_of_inventing_metadata(){
        let s=clean("சரவணன் இருக்க பயமேன் (Official Video) | இமான்","Sony Music Tamil");
        assert_eq!(s.title,"சரவணன் இருக்க பயமேன்");assert_eq!(s.artist,None);
        assert_eq!(filename(&s),"சரவணன் இருக்க பயமேன்.m4a");
        let s=clean("Song (Live at London) [Lyrics]","Singer - Topic");
        assert_eq!(s.title,"Song (Live at London)");assert_eq!(filename(&s),"Singer - Song (Live at London).m4a");
    }
    #[test] fn filenames_cannot_escape_directories_or_grow_without_bound(){
        let s=clean("../../Song: \\\"title\\\" [4K]\u{202e}","");let name=filename(&s);
        assert!(!name.contains('/'));assert!(!name.contains('\\'));assert!(!name.starts_with('.'));assert!(!name.contains('\u{202e}'));
        assert!(filename(&clean(&"அ".repeat(1000),"")).len()<=184);
        assert!(!clean("[Unreleased version] Song","").title.is_empty());
    }
    #[test] fn front_and_tail_mp4_metadata_roundtrip_preserves_decodability(){
        use rodio::Source;
        let Ok(root)=std::env::var("AYSHA_MEDIA_FIXTURES")else{return;};
        for kind in ["front","tail"] {
            let path=std::env::temp_dir().join(format!("aysha-tags-{}-{kind}.m4a",std::process::id()));
            std::fs::copy(Path::new(&root).join(format!("{kind}.m4a")),&path).unwrap();
            let song=clean("A melody [Official Video]","An artist - Topic");
            let cover=std::fs::read(Path::new(&root).join("cover.jpg")).unwrap();
            let duration=tag(&path,&song,Some(&cover)).unwrap();assert!(duration.as_secs()>=4);
            let t=mp4ameta::Tag::read_from_path(&path).unwrap();assert_eq!(t.title(),Some("A melody"));assert_eq!(t.artist(),Some("An artist"));assert_eq!(t.artwork().unwrap().data,cover.as_slice());
            let mut decoder=rodio::Decoder::try_from(std::fs::File::open(&path).unwrap()).unwrap();
            let rate=decoder.sample_rate().get() as usize;let channels=decoder.channels().get() as usize;
            assert!(decoder.by_ref().count()>=rate*channels*4,"tagged fixture must decode through the complete song");let _=std::fs::remove_file(path);
        }
    }
}
