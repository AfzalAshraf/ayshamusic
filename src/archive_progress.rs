//! Ephemeral transfer telemetry. Persistent queue state remains in SQLite.
use serde::Serialize;
use std::{sync::{Arc,Mutex},pin::Pin,task::{Context,Poll}};
use tokio::{io::{AsyncRead,ReadBuf},sync::Notify};
#[derive(Clone,Serialize)]
pub struct Progress {pub id:i64,pub stage:&'static str,pub bytes:u64,pub total:Option<u64>}
struct Active {value:Progress,cancel:Arc<Notify>}
static ACTIVE:Mutex<Option<Active>>=Mutex::new(None);
pub fn begin(id:i64)->Arc<Notify> {
    let cancel=Arc::new(Notify::new());
    *ACTIVE.lock().unwrap()=Some(Active{value:Progress{id,stage:"checking",bytes:0,total:None},cancel:cancel.clone()});cancel
}
pub fn snapshot()->Option<Progress> {ACTIVE.lock().unwrap().as_ref().map(|a|a.value.clone())}
pub fn stage(id:i64,stage:&'static str,total:Option<u64>) {
    if let Some(a)=ACTIVE.lock().unwrap().as_mut().filter(|a|a.value.id==id) {a.value.stage=stage;a.value.bytes=0;a.value.total=total;}
}
pub fn bytes(id:i64,count:u64) {
    if let Some(a)=ACTIVE.lock().unwrap().as_mut().filter(|a|a.value.id==id) {a.value.bytes=count;}
}
pub fn clear(id:i64) {let mut a=ACTIVE.lock().unwrap();if a.as_ref().is_some_and(|a|a.value.id==id){*a=None;}}
/// Serializes the cancel/final-send boundary. Never claims to undo a sent message.
pub fn cancel(cache:&crate::tg_cache::TelegramCacheManager,id:i64)->Result<bool,String> {
    let active=ACTIVE.lock().unwrap();
    if active.as_ref().is_some_and(|a|a.value.id==id && a.value.stage=="finalizing") {return Err("This song is being committed to Saved Messages. It can no longer be cancelled here.".into());}
    let changed=cache.cancel_save(id).map_err(|_|"Could not cancel this save".to_string())?;
    if changed {if let Some(a)=active.as_ref().filter(|a|a.value.id==id){a.cancel.notify_one();}}
    Ok(changed)
}
pub fn finalizing(cache:&crate::tg_cache::TelegramCacheManager,id:i64)->std::io::Result<()> {
    let mut active=ACTIVE.lock().unwrap();
    if !cache.save_active(id).unwrap_or(false){return Err(std::io::Error::other("archive_cancelled"));}
    if let Some(a)=active.as_mut().filter(|a|a.value.id==id){a.value.stage="finalizing";}
    Ok(())
}
/// grammers exposes AsyncRead, not server-acknowledged part callbacks. Report
/// actual bytes read into its uploader; completion is separate from this count.
pub struct UploadReader<R> {pub inner:R,pub id:i64,pub count:u64}
impl<R:AsyncRead+Unpin> AsyncRead for UploadReader<R> {
    fn poll_read(mut self:Pin<&mut Self>,cx:&mut Context<'_>,buf:&mut ReadBuf<'_>)->Poll<std::io::Result<()>> {
        let before=buf.filled().len();
        let result=Pin::new(&mut self.inner).poll_read(cx,buf);
        if matches!(result,Poll::Ready(Ok(()))) {self.count+=(buf.filled().len()-before) as u64;bytes(self.id,self.count);}
        result
    }
}
#[cfg(test)] mod tests {
    use super::*;use tokio::io::AsyncReadExt;
    #[tokio::test] async fn uploader_reports_real_reads_without_claiming_remote_completion() {
        let id=987654;
        let mut r=UploadReader{inner:&b"hello"[..],id,count:0};let mut out=vec![];
        r.read_to_end(&mut out).await.unwrap();
        assert_eq!(r.count,5);assert_eq!(out,b"hello");
    }
}
