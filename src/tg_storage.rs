//! User-approved Saved Messages archive. No bot, server, or account secret in logs.
//! Telegram byte ranges feed the same seekable Rust AAC decoder as YouTube ranges.
use crate::{player, range_media::{Block,Fetch,BLOCK}, tg_cache::TelegramCacheManager};
use grammers_client::{client::Client, media::{Downloadable, Media}, message::InputMessage};
use grammers_session::types::{PeerAuth,PeerId,PeerRef};
use std::{io,path::Path,sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}},time::Duration};
use tokio::io::AsyncWriteExt;

const MAX_TRACK:u64=64*1024*1024;
static PAUSED:AtomicBool=AtomicBool::new(false);
static LAST_ERROR:Mutex<Option<String>>=Mutex::new(None);
fn me()->PeerRef {PeerRef{id:PeerId::self_user(),auth:PeerAuth::default()}}
fn error(text:&str)->io::Error {io::Error::other(text)}
pub fn pause(value:bool) {PAUSED.store(value,Ordering::Release);}
pub fn warn(message:&str) {*LAST_ERROR.lock().unwrap()=Some(message.into());}
pub fn last_error()->Option<String> {LAST_ERROR.lock().unwrap().clone()}
fn exact_tag(text:&str,id:&str)->bool {text.lines().next()==Some(format!("#yt_{id}").as_str())}
struct TelegramFetch {client:Arc<Client>,media:Media,total:u64,runtime:tokio::runtime::Handle}
impl Fetch for TelegramFetch {
    fn fetch(&self,start:u64,end:u64)->io::Result<Block> {
        if PAUSED.load(Ordering::Acquire) {return Err(error("telegram_disconnected"));}
        let last=end.min(self.total-1);
        if start>last {return Err(error("telegram_range_bounds"));}
        // MTProto offsets must be aligned. At most two 64KiB chunks cover a request.
        let offset=(start%BLOCK as u64) as usize;
        let needed=(last-start+1) as usize;
        self.runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(25),async {
                let mut iter=self.client.iter_download(&self.media).chunk_size(BLOCK as i32).skip_chunks((start/BLOCK as u64) as i32);
                let mut bytes=Vec::with_capacity(BLOCK*2);
                while bytes.len()<offset+needed {
                    let chunk=iter.next().await.map_err(|e|{if let (Some(cache),Some(seconds))=(crate::cache_manager(),flood_wait(&e.to_string())) {let _=cache.on_flood_wait(seconds);}error("telegram_media_read_failed")})?.ok_or_else(||error("telegram_media_truncated"))?;
                    if chunk.is_empty() || bytes.len()+chunk.len()>BLOCK*2 {return Err(error("telegram_media_chunk_invalid"));}
                    bytes.extend_from_slice(&chunk);
                }
                Ok(Block{bytes:bytes[offset..offset+needed].to_vec(),total:self.total})
            }).await.map_err(|_|error("telegram_media_timeout"))?
        })
    }
}
fn saved_media(client:Arc<Client>,id:&str,message:&grammers_client::message::Message)->Option<player::Media> {
    if !exact_tag(message.text(),id) {return None;}
    let media=message.media()?;
    let size=media.size().filter(|size|*size>0 && (*size as u64)<=MAX_TRACK)?;
    let title=message.text().lines().nth(1).map(str::to_owned);
    Some(player::Media{video_id:id.into(),title,client:"TELEGRAM",itag:140,url:String::new(),user_agent:"",content_length:Some(size as u64),fetch:Some(Box::new(TelegramFetch{client,media,total:size as u64,runtime:tokio::runtime::Handle::current()}))})
}
pub async fn lookup(cache:&TelegramCacheManager,id:&str)->Result<Option<player::Media>,String> {
    if PAUSED.load(Ordering::Acquire) {return Ok(None);}
    let Some(client)=cache.telegram().authorized_client().await else {return Ok(None);};
    tokio::time::timeout(Duration::from_secs(15),async {
        if let Some(message_id)=cache.saved_message(id).ok().flatten() {
            let messages=client.get_messages_by_id(me(),&[message_id]).await.map_err(|e|{if let Some(seconds)=flood_wait(&e.to_string()) {let _=cache.on_flood_wait(seconds);} "Telegram message unavailable".to_string()})?;
            for message in messages.into_iter().flatten() {
                if let Some(media)=saved_media(client.clone(),id,&message) {return Ok(Some(media));}
            }
            let _=cache.forget_saved(id);
        }
        let mut messages=client.search_messages(me()).query(&format!("#yt_{id}"));
        // Search is fuzzy: require our exact first-line ID marker, in Saved Messages only.
        for _ in 0..8 {
            let Some(message)=messages.next().await.map_err(|e|{if let Some(seconds)=flood_wait(&e.to_string()) {let _=cache.on_flood_wait(seconds);} "Telegram search unavailable".to_string()})? else {break;};
            if let Some(media)=saved_media(client.clone(),id,&message) {
                let _=cache.remember_saved(id,message.id());return Ok(Some(media));
            }
        }
        Ok(None)
    }).await.map_err(|_|"Telegram lookup timed out".to_string())?
}

/// Sequential persistent worker: complete files even after a skip, then upload.
/// No partial file is published, and Telegram's FloodWait always delays retries.
pub async fn worker(cache:Arc<TelegramCacheManager>) {
    // Remove incomplete files left by a killed/timed-out download; never publish them.
    if let Some(root)=cache.cached_path("cleanup").parent() {
        if let Ok(entries)=std::fs::read_dir(root) {for entry in entries.flatten() {if entry.path().extension().is_some_and(|ext|ext=="part") {let _=std::fs::remove_file(entry.path());}}}
    }
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        if PAUSED.load(Ordering::Acquire) || !cache.telegram().is_configured() {continue;}
        let Some(client)=cache.telegram().authorized_client().await else {continue;};
        match cache.seconds_until_allowed() {Ok(0)=>(),_=>continue}
        let Ok(Some(item))=cache.take_next_pending() else {continue;};
        let result=tokio::time::timeout(Duration::from_secs(15*60),async {
            // Prevent duplicate Saved Messages after restart/retry.
            if lookup(&cache,&item.video_id).await.map_err(io::Error::other)?.is_some() {return Ok(());}
            let path=Path::new(&item.file_path);
            if !path.is_file() {download(&cache,&item.video_id,path).await?;}
            if PAUSED.load(Ordering::Acquire) {return Err(error("telegram_disconnected"));}
            // A complete, decodable AAC file is required before any upload.
            let validate=path.to_owned();
            tokio::task::spawn_blocking(move || {
                let mut decoder=rodio::Decoder::try_from(std::fs::File::open(validate)?).map_err(|_|error("archive_audio_not_decodable"))?;
                decoder.next().ok_or_else(||error("archive_audio_empty"))?;Ok::<_,io::Error>(())
            }).await.map_err(|_|error("archive_validation_failed"))??;
            let mut file=tokio::fs::File::open(path).await?;
            let len=file.metadata().await?.len();
            if len==0 || len>MAX_TRACK {return Err(error("archive_track_size_limit"));}
            let uploaded=client.upload_stream(&mut file,len as usize,format!("{}.m4a",item.video_id)).await?;
            if PAUSED.load(Ordering::Acquire) {return Err(error("telegram_disconnected"));}
            let message=client.send_message(me(),InputMessage::new().text(&item.caption).file(uploaded).mime_type("audio/mp4")).await.map_err(|e|error(&e.to_string()))?;
            cache.remember_saved(&item.video_id,message.id()).map_err(|_|error("archive_index_failed"))?;
            Ok::<_,io::Error>(())
        }).await.unwrap_or_else(|_|Err(error("archive_operation_timeout")));
        match result {
            Ok(())=>{let _=cache.record_upload(&item.video_id);let _=cache.mark_done(item.id);*LAST_ERROR.lock().unwrap()=None;}
            Err(e)=>{
                if let Some(seconds)=flood_wait(&e.to_string()) {let _=cache.on_flood_wait(seconds);}
                // Do not expose Telegram responses, phone numbers, tokens or signed URLs.
                let _=cache.mark_failed(item.id,"archive_failed");
                *LAST_ERROR.lock().unwrap()=Some("A song could not be saved. Check Telegram, storage and the upload queue; playback is independent.".into());
            }
        }
        let _=cache.evict_lru();
    }
}
async fn download(cache:&TelegramCacheManager,id:&str,path:&Path)->io::Result<()> {
    use futures_util::StreamExt;
    let http=crate::http_client().ok_or_else(||error("archive_http_unavailable"))?;
    let visitor=crate::visitor(http).await;
    let audio=crate::youtube::resolve_audio(http,id,visitor.attached().then_some(visitor.value.as_str())).await.map_err(|_|error("archive_resolution_failed"))?;
    if audio.content_length.is_some_and(|n|n>MAX_TRACK) {return Err(error("archive_track_size_limit"));}
    let media=crate::media_client().ok_or_else(||error("archive_media_unavailable"))?;
    if !cache.make_room(audio.content_length.unwrap_or(MAX_TRACK)).map_err(|_|error("archive_cache_unavailable"))? {return Err(error("archive_cache_full"));}
    let tmp=path.with_extension("part");
    // Overwrite an old failed partial, if present; successful final files are atomic.
    let _=tokio::fs::remove_file(&tmp).await;
    let result=async {
        let response=media.get(&audio.url).header(reqwest::header::USER_AGENT,crate::youtube::ANDROID.user_agent).send().await.map_err(|_|error("archive_media_failed"))?;
        if !response.status().is_success() {return Err(error("archive_media_rejected"));}
        let mut file=tokio::fs::File::create(&tmp).await?;
        let mut stream=response.bytes_stream();let mut count=0u64;
        let baseline=cache.cache_bytes().map_err(|_|error("archive_cache_unavailable"))?;
        while let Some(chunk)=stream.next().await {
            let chunk=chunk.map_err(|_|error("archive_stream_failed"))?;
            count+=chunk.len() as u64;
            if count>MAX_TRACK || baseline+count>crate::tg_cache::DEFAULT_MAX_CACHE_BYTES {return Err(error("archive_cache_full"));}
            if PAUSED.load(Ordering::Acquire) {return Err(error("telegram_disconnected"));}
            file.write_all(&chunk).await?;
        }
        if count==0 || audio.content_length.is_some_and(|n|n!=count) {return Err(error("archive_incomplete"));}
        file.sync_all().await?;drop(file);tokio::fs::rename(&tmp,path).await?;Ok(())
    }.await;
    if result.is_err() {let _=tokio::fs::remove_file(&tmp).await;}
    result
}
fn flood_wait(text:&str)->Option<u64> {
    let value=text.split("FLOOD_WAIT").nth(1)?.trim_start_matches(|c:char|!c.is_ascii_digit());
    value.chars().take_while(char::is_ascii_digit).collect::<String>().parse::<u64>().ok().map(|s|s.min(86400))
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn saved_message_match_is_exact_not_fuzzy_or_another_song() {
        assert!(exact_tag("#yt_abcdefghijk\nSong","abcdefghijk"));
        assert!(!exact_tag("#yt_abcdefghijkX\nSong","abcdefghijk"));
        assert!(!exact_tag("Other text #yt_abcdefghijk","abcdefghijk"));
    }
    #[test] fn flood_wait_has_a_real_duration_and_is_bounded() {
        assert_eq!(flood_wait("FLOOD_WAIT_42"),Some(42));assert_eq!(flood_wait("FLOOD_WAIT (value: 61)"),Some(61));assert_eq!(flood_wait("network unavailable"),None);
    }
}
