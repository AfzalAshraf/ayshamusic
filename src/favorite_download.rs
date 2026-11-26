//! One sequential favorite download worker; no provider retry or identity rotation.
use crate::{android_catalog::Track, local_favorites::LocalFavorites};
use std::{io, path::{Path,PathBuf}, sync::Arc, time::Duration};
use tokio::io::AsyncWriteExt;
use futures_util::StreamExt;
const MAX_BYTES:u64=64*1024*1024;

fn temporary_path(cache:&Path,id:&str)->Option<PathBuf> {
    if !crate::youtube::valid_video_id(id){return None;}
    Some(cache.join("favorite-downloads").join(format!("{id}.part")))
}
fn valid_mp4_header(header:&[u8])->bool {header.len()>=12 && &header[4..8]==b"ftyp"}

pub async fn worker(db:Arc<LocalFavorites>,cache:PathBuf) {
    let root=cache.join("favorite-downloads");
    if std::fs::create_dir_all(&root).is_err(){return;}
    // Only our own incomplete staging files; published songs are never touched.
    if let Ok(entries)=std::fs::read_dir(&root){for entry in entries.flatten(){let path=entry.path();if path.extension().is_some_and(|v|v=="part"){let _=std::fs::remove_file(path);}}}
    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let Ok(Some(track))=db.next() else {continue;};
        let path=match temporary_path(&cache,&track.id){Some(path)=>path,None=>{db.finish(&track.id,false);continue;}};
        let result=tokio::time::timeout(Duration::from_secs(15*60),download(&db,&track,&path)).await;
        let success=matches!(result,Ok(Ok(())));
        let _=tokio::fs::remove_file(&path).await;
        db.finish(&track.id,success);
    }
}
async fn download(db:&Arc<LocalFavorites>,track:&Track,path:&Path)->io::Result<()> {
    let http=crate::http_client().ok_or_else(||io::Error::other("unavailable"))?;
    let visitor=crate::visitor(http).await;
    let audio=crate::resolve_default(http,&track.id,visitor.attached().then_some(visitor.value.as_str())).await.map_err(|_|io::Error::other("source unavailable"))?;
    if audio.content_length.is_some_and(|n|n==0 || n>MAX_BYTES){return Err(io::Error::other("size limit"));}
    if !db.active(&track.id){return Err(io::Error::other("cancelled"));}
    let client=crate::media_client().ok_or_else(||io::Error::other("unavailable"))?;
    let response=client.get(&audio.url).header(reqwest::header::USER_AGENT,audio.user_agent()).send().await.map_err(|_|io::Error::other("download failed"))?;
    if !response.status().is_success() || response.content_length().is_some_and(|n|n>MAX_BYTES) {return Err(io::Error::other("download rejected"));}
    let mut file=tokio::fs::File::create(path).await?;let mut stream=response.bytes_stream();let mut count=0u64;let mut last_report=0u64;let mut header=Vec::with_capacity(12);
    while let Some(chunk)=stream.next().await {
        let chunk=chunk.map_err(|_|io::Error::other("download interrupted"))?;
        count=count.checked_add(chunk.len() as u64).ok_or_else(||io::Error::other("size limit"))?;
        if count>MAX_BYTES || !db.active(&track.id){return Err(io::Error::other("cancelled or size limit"));}
        if header.len()<12 {header.extend_from_slice(&chunk[..chunk.len().min(12-header.len())]);}
        file.write_all(&chunk).await?;
        if count-last_report>=256*1024 {db.progress(&track.id,count,audio.content_length);last_report=count;}
    }
    if !valid_mp4_header(&header) || count==0 || audio.content_length.is_some_and(|n|n!=count){return Err(io::Error::other("incomplete audio"));}
    file.sync_all().await?;drop(file);
    if !db.active(&track.id){return Err(io::Error::other("cancelled"));}
    let path=path.to_owned();let item=track.clone();
    let published=tokio::task::spawn_blocking(move || {
        #[cfg(target_os="android")] {crate::android::publish_favorite(&path,&item.id,&item.title,&item.artist)}
        #[cfg(not(target_os="android"))] {let _=(path,item);false}
    }).await.unwrap_or(false);
    if !published{return Err(io::Error::other("publish failed"));}Ok(())
}
#[cfg(test)] mod tests {use super::*;
    #[test] fn paths_and_mp4_headers_are_bounded() {
        assert!(temporary_path(Path::new("/tmp"),"../../evil").is_none());
        assert!(temporary_path(Path::new("/tmp"),"abcdefghijk").unwrap().ends_with("favorite-downloads/abcdefghijk.part"));
        assert!(!valid_mp4_header(b"<html>blocked"));assert!(valid_mp4_header(b"\0\0\0\x18ftypM4A "));
    }
}
