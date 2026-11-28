//! User-approved Saved Messages archive. No bot, server, or account secret in logs.
//! Telegram byte ranges feed the same seekable Rust AAC decoder as YouTube ranges.
use crate::{player, range_media::{Block,Fetch,BLOCK}, tg_cache::TelegramCacheManager};
use grammers_client::{client::Client, media::{Downloadable, Media, Attribute}, message::InputMessage};
use grammers_session::types::{PeerAuth,PeerId,PeerRef};
use std::{io,path::Path,sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}},time::Duration};
use tokio::io::AsyncWriteExt;
use rodio::Source;

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
    Some(player::Media{video_id:id.into(),title,client:"TELEGRAM",itag:140,url:String::new(),user_agent:"",content_length:Some(size as u64),fetch:Some(Box::new(TelegramFetch{client,media,total:size as u64,runtime:tokio::runtime::Handle::current()})),#[cfg(target_os="windows")] source:None})
}
pub async fn lookup(cache:&TelegramCacheManager,id:&str)->Result<Option<player::Media>,String> {
    if PAUSED.load(Ordering::Acquire) {return Ok(None);}
    tokio::time::timeout(Duration::from_secs(15),async {
        let Some(client)=cache.telegram().authorized_client().await else {return Ok(None);};
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
        if let Ok(entries)=std::fs::read_dir(root) {for entry in entries.flatten() {if entry.path().extension().is_some_and(|ext|ext=="part" || ext.to_string_lossy().starts_with("tagging-")) {let _=std::fs::remove_file(entry.path());}}}
    }
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        if PAUSED.load(Ordering::Acquire) || !cache.telegram().is_configured() {continue;}
        let Ok(Some(client))=tokio::time::timeout(Duration::from_secs(20),cache.telegram().authorized_client()).await else {continue;};
        match cache.seconds_until_allowed() {Ok(0)=>(),_=>continue}
        let Ok(Some(item))=cache.take_next_pending() else {continue;};
        let cancel=crate::archive_progress::begin(item.id);
        if !cache.save_active(item.id).unwrap_or(false) {crate::archive_progress::clear(item.id);continue;}
        let operation=tokio::time::timeout(Duration::from_secs(15*60),async {
            // Prevent duplicate Saved Messages after restart/retry.
            if lookup(&cache,&item.video_id).await.map_err(io::Error::other)?.is_some() {return Ok((false,true));}
            let path=Path::new(&item.file_path);
            if !path.is_file() {download(&cache,&item.video_id,path,item.id).await?;}
            crate::archive_progress::stage(item.id,"validating",None);
            if PAUSED.load(Ordering::Acquire) {return Err(error("telegram_disconnected"));}
            // A complete, decodable AAC file is required before any upload.
            let validate=path.to_owned();
            let decoded_duration=tokio::task::spawn_blocking(move || {
                let mut decoder=rodio::Decoder::try_from(std::fs::File::open(validate)?).map_err(|_|error("archive_audio_not_decodable"))?;
                let duration=decoder.total_duration();
                decoder.next().ok_or_else(||error("archive_audio_empty"))?;Ok::<_,io::Error>(duration)
            }).await.map_err(|_|error("archive_validation_failed"))??;
            crate::archive_progress::stage(item.id,"tagging",None);
            let song=crate::song_metadata::clean(&item.title,&item.artist);
            let cover=cover(&item.video_id).await;
            let size=tokio::fs::metadata(path).await?.len();
            // Enrichment must not regress working uploads of a valid container
            // variant. Tag a copy and fall back to Telegram audio attributes.
            let room=cache.make_room(size.saturating_add(512*1024)).unwrap_or(false);
            let tagged=if room {
                let tagging=path.to_owned();let metadata=song.clone();let picture=cover.clone();
                tokio::task::spawn_blocking(move ||crate::song_metadata::tag(&tagging,&metadata,picture.as_deref())).await.ok().and_then(Result::ok)
            }else{None};
            let tags_applied=tagged.is_some();
            let duration=tagged.or(decoded_duration).filter(|d|d.as_secs()<=604800).unwrap_or(Duration::ZERO);
            let file=tokio::fs::File::open(path).await?;
            let len=file.metadata().await?.len();
            if len==0 || len>MAX_TRACK {return Err(error("archive_track_size_limit"));}
            crate::archive_progress::stage(item.id,"uploading",Some(len));
            let mut reader=crate::archive_progress::UploadReader{inner:file,id:item.id,count:0};
            let uploaded=client.upload_stream(&mut reader,len as usize,crate::song_metadata::filename(&song)).await?;
            let caption=if song.source.is_empty(){format!("#yt_{}\n{}",item.video_id,song.title)}else{format!("#yt_{}\n{}\nSource: {}",item.video_id,song.title,song.source)};
            let mut message=InputMessage::new().text(caption).document(uploaded).mime_type("audio/mp4")
                .attribute(Attribute::Audio{duration,title:Some(song.title.clone()),performer:song.artist.clone()});
            if let Some(bytes)=cover {
                crate::archive_progress::stage(item.id,"artwork",None);
                let size=bytes.len();let mut stream=std::io::Cursor::new(bytes);
                let thumb=client.upload_stream(&mut stream,size,"cover.jpg".into()).await?;
                message=message.thumbnail(thumb);
            }
            if PAUSED.load(Ordering::Acquire) {return Err(error("telegram_disconnected"));}
            crate::archive_progress::finalizing(&cache,item.id)?;
            let message=client.send_message(me(),message).await.map_err(|e|error(&e.to_string()))?;
            cache.remember_saved(&item.video_id,message.id()).map_err(|_|error("archive_index_failed"))?;
            Ok::<_,io::Error>((true,tags_applied))
        });
        let result=tokio::select! {
            biased;
            _=cancel.notified()=>None,
            result=operation=>Some(result.unwrap_or_else(|_|Err(error("archive_operation_timeout")))),
        };
        crate::archive_progress::clear(item.id);
        if result.is_none() {
            let _=tokio::fs::remove_file(Path::new(&item.file_path).with_extension("part")).await;
            continue;
        }
        match result.unwrap() {
            Ok((uploaded,tags_applied))=>{if uploaded {let _=cache.record_upload(&item.video_id);}let _=cache.mark_done(item.id);*LAST_ERROR.lock().unwrap()=if tags_applied {None}else{Some("A song was saved as audio, but its embedded tags could not be updated. Playback and the original file were preserved.".into())};}
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
async fn download(cache:&TelegramCacheManager,id:&str,path:&Path,item:i64)->io::Result<()> {
    use futures_util::StreamExt;
    let http=crate::http_client().ok_or_else(||error("archive_http_unavailable"))?;
    crate::archive_progress::stage(item,"resolving",None);
    let visitor=crate::visitor(http).await;
    let audio=crate::resolve_default(http,id,visitor.attached().then_some(visitor.value.as_str())).await.map_err(|_|error("archive_resolution_failed"))?;
    if audio.content_length.is_some_and(|n|n>MAX_TRACK) {return Err(error("archive_track_size_limit"));}
    let media=crate::media_client().ok_or_else(||error("archive_media_unavailable"))?;
    if !cache.make_room(audio.content_length.unwrap_or(MAX_TRACK)).map_err(|_|error("archive_cache_unavailable"))? {return Err(error("archive_cache_full"));}
    let tmp=path.with_extension("part");
    // Overwrite an old failed partial, if present; successful final files are atomic.
    let _=tokio::fs::remove_file(&tmp).await;
    let result=async {
        let response=media.get(&audio.url).header(reqwest::header::USER_AGENT,audio.user_agent()).send().await.map_err(|_|error("archive_media_failed"))?;
        if !response.status().is_success() {return Err(error("archive_media_rejected"));}
        crate::archive_progress::stage(item,"downloading",audio.content_length);
        let mut file=tokio::fs::File::create(&tmp).await?;
        let mut stream=response.bytes_stream();let mut count=0u64;
        let baseline=cache.cache_bytes().map_err(|_|error("archive_cache_unavailable"))?;
        while let Some(chunk)=stream.next().await {
            let chunk=chunk.map_err(|_|error("archive_stream_failed"))?;
            count+=chunk.len() as u64;
            if count>MAX_TRACK || baseline+count>crate::tg_cache::DEFAULT_MAX_CACHE_BYTES {return Err(error("archive_cache_full"));}
            if PAUSED.load(Ordering::Acquire) {return Err(error("telegram_disconnected"));}
            file.write_all(&chunk).await?;
            crate::archive_progress::bytes(item,count);
        }
        if count==0 || audio.content_length.is_some_and(|n|n!=count) {return Err(error("archive_incomplete"));}
        file.sync_all().await?;drop(file);tokio::fs::rename(&tmp,path).await?;Ok(())
    }.await;
    if result.is_err() {let _=tokio::fs::remove_file(&tmp).await;}
    result
}
/// Optional art must never block audio indefinitely or fetch a caller URL.
async fn cover(id:&str)->Option<Vec<u8>> {
    if !crate::youtube::valid_video_id(id){return None;}
    #[cfg(not(target_os="android"))] {let _=id;None}
    #[cfg(target_os="android")] {
        tokio::time::timeout(Duration::from_secs(6),async {
            use futures_util::StreamExt;
            let response=crate::http_client()?.get(format!("https://img.youtube.com/vi/{id}/hqdefault.jpg")).send().await.ok()?;
            if !response.status().is_success() || response.content_length().is_some_and(|n|n>1024*1024){return None;}
            let mut chunks=response.bytes_stream();let mut bytes=Vec::new();
            while let Some(chunk)=chunks.next().await {let chunk=chunk.ok()?;if bytes.len()+chunk.len()>1024*1024{return None;}bytes.extend_from_slice(&chunk);}
            tokio::task::spawn_blocking(move ||crate::android::square_cover(&bytes)).await.ok().flatten()
        }).await.ok().flatten()
    }
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
