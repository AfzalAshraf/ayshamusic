//! Media3 variant: Rust owns media acquisition/Telegram; Kotlin owns audio only.
//! ExoPlayer reads a token-scoped loopback range endpoint, never account secrets
//! or signed provider URLs. Rust variant cannot accept platform playback events.
use crate::{player::Media,range_media::{Fetch,HttpFetch,PreparedFetch,BLOCK}};
use axum::{body::{Body,Bytes},extract::Path,http::{header,HeaderMap,Method,StatusCode},response::{IntoResponse,Response},Json};
use serde::Deserialize;
use serde_json::{json,Value};
use std::{io::{self,Read},sync::{Arc,Mutex,atomic::{AtomicUsize,Ordering}}};
use tokio::sync::oneshot;

static PREPARING:AtomicUsize=AtomicUsize::new(0);
struct Worker;
impl Drop for Worker {fn drop(&mut self){PREPARING.fetch_sub(1,Ordering::AcqRel);}}
type Request=(u64,u64,std::sync::mpsc::SyncSender<io::Result<crate::range_media::Block>>);
struct Input {requests:std::sync::mpsc::SyncSender<Request>,len:u64}
impl Input {
    fn fetch(&self,start:u64,end:u64)->io::Result<crate::range_media::Block> {
        let (tx,rx)=std::sync::mpsc::sync_channel(1);
        self.requests.try_send((start,end,tx)).map_err(|_|io::Error::other("media3_input_worker_busy_or_closed"))?;
        rx.recv_timeout(std::time::Duration::from_secs(15)).map_err(|_|io::Error::other("media3_input_worker_timeout"))?
    }
}
struct Active {
    token:String,revision:u64,wants_play:bool,id:String,title:Option<String>,input:Option<Arc<Input>>,
    state:String,error:Option<String>,position:f64,duration:f64,
    ready:Option<oneshot::Sender<Result<(),String>>>,
}
struct Inner {active:Option<Active>,volume:f32}
#[derive(Clone)] pub struct Player {inner:Arc<Mutex<Inner>>,dispatch:Arc<dyn Fn(Value)->Result<(),String>+Send+Sync>}
fn command(value:Value)->Result<(),String> {
    #[cfg(target_os="android")] {crate::android::platform_command(&value.to_string())}
    #[cfg(not(target_os="android"))] {let _=value;Err("Media3 audio requires Android".into())}
}
fn token()->io::Result<String> {let mut bytes=[0u8;20];std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;Ok(bytes.iter().map(|b|format!("{b:02x}")).collect())}
impl Player {
    pub fn new()->Self {Self{inner:Arc::new(Mutex::new(Inner{active:None,volume:1.0})),dispatch:Arc::new(command)}}
    fn emit(&self,value:Value)->Result<(),String>{(self.dispatch)(value)}
    pub fn play(&self,media:Media)->Result<oneshot::Receiver<Result<(),String>>,String> {
        PREPARING.fetch_update(Ordering::AcqRel,Ordering::Acquire,|n|(n<2).then_some(n+1)).map_err(|_|"media3_prepare_busy")?;
        let worker=Worker;
        let token=token().map_err(|_|"media3_token_unavailable")?;
        let (ready,rx)=oneshot::channel();
        {
            let mut inner=self.inner.lock().unwrap();
            if let Some(mut old)=inner.active.take() {
                if let Some(ready)=old.ready.take(){let _=ready.send(Err("playback_cancelled".into()));}
                self.emit(json!({"op":"stop","token":old.token}))?;
            }
            inner.active=Some(Active{token:token.clone(),revision:0,wants_play:true,id:media.video_id.clone(),title:media.title.clone(),input:None,state:"buffering".into(),error:None,position:0.0,duration:0.0,ready:Some(ready)});
        }
        let player=self.clone();
        let failed=token.clone();
        let spawned=std::thread::Builder::new().name("aysha-media3-input".into()).spawn(move || {
            let _worker=worker;
            let result=(||->Result<(),String>{
                let fetch:Box<dyn Fetch>=match media.fetch {Some(fetch)=>fetch,None=>Box::new(HttpFetch::new(media.url,media.user_agent).map_err(|e|e.to_string())?)};
                let fetch=PreparedFetch::new(fetch,media.content_length).map_err(|e|e.to_string())?;
                let total=fetch.len();
                let (requests,rx)=std::sync::mpsc::sync_channel::<Request>(2);
                let volume={
                    let mut inner=player.inner.lock().unwrap();
                    let active=inner.active.as_mut().filter(|a|a.token==token).ok_or("playback_cancelled")?;
                    active.input=Some(Arc::new(Input{requests,len:total}));inner.volume
                };
                // Check again under the same operation mutex used by Stop. A late
                // prepare cannot issue a fresh Play after Stop has cleared it.
                let inner=player.inner.lock().unwrap();
                let active=inner.active.as_ref().filter(|a|a.token==token).ok_or("playback_cancelled")?;
                player.emit(json!({"op":"play","token":token,"revision":active.revision,"play_when_ready":active.wants_play,"url":format!("http://127.0.0.1:{}/api/platform/media/{}",crate::api_port(),token),"volume":volume}))?;
                drop(inner);
                // This worker owns and drops the blocking client outside Tokio.
                while let Ok((start,end,reply))=rx.recv() {
                    let _=reply.send(fetch.fetch(start,end));
                }
                Ok(())
            })();
            if let Err(error)=result {player.fail(&token,&format!("media3_input_failed: {error}; stage=prepare"));}
        });
        if spawned.is_err() {self.fail(&failed,"media3_input_thread_unavailable");return Err("media3_input_thread_unavailable".into());}
        Ok(rx)
    }
    fn control(&self,op:&str)->Result<(),String> {
        let mut inner=self.inner.lock().unwrap();
        let Some(active)=inner.active.as_mut() else {return Ok(());};
        if active.error.is_some(){return Err("playback_failed_select_track_again".into());}
        active.revision=active.revision.wrapping_add(1);
        active.wants_play=op!="pause";
        self.emit(json!({"op":op,"token":active.token,"revision":active.revision}))?;
        if op=="pause" {active.state="paused".into();}
        Ok(())
    }
    pub fn pause(&self)->Result<(),String>{self.control("pause")}
    pub fn resume(&self)->Result<(),String>{self.control("resume")}
    pub fn seek(&self,id:&str,position:f64)->Result<(),String> {
        let mut inner=self.inner.lock().unwrap();
        let a=inner.active.as_mut().filter(|a|a.id==id).ok_or("Song changed; try seeking again")?;
        if !position.is_finite() || position<0.0 || !a.duration.is_finite() || a.duration<=0.0 || position>a.duration || a.error.is_some() || !["playing","paused","buffering"].contains(&a.state.as_str()) {return Err("Seek position unavailable for this song".into());}
        a.revision=a.revision.wrapping_add(1);
        self.emit(json!({"op":"seek","token":a.token,"revision":a.revision,"position":position}))?;
        a.position=position;Ok(())
    }
    pub fn stop(&self)->Result<(),String>{
        let mut inner=self.inner.lock().unwrap();
        if let Some(mut active)=inner.active.take() {
            if let Some(ready)=active.ready.take(){let _=ready.send(Err("playback_cancelled".into()));}
            self.emit(json!({"op":"stop","token":active.token}))?;
        } Ok(())
    }
    pub fn set_volume(&self,volume:f32)->Result<(),String>{
        let mut inner=self.inner.lock().unwrap();inner.volume=if volume.is_finite(){volume.clamp(0.0,1.0)}else{1.0};
        if let Some(active)=&inner.active {self.emit(json!({"op":"volume","token":active.token,"volume":inner.volume}))?;}Ok(())
    }
    pub fn snapshot(&self)->Value {
        let inner=self.inner.lock().unwrap();
        match &inner.active {
            Some(a)=>json!({"state":a.state,"video_id":a.id,"title":a.title,"volume":inner.volume,"position_secs":a.position,"duration_secs":a.duration,"error":a.error}),
            None=>json!({"state":"idle","video_id":null,"title":null,"volume":inner.volume,"position_secs":0.0,"duration_secs":0.0,"error":null}),
        }
    }
    fn fail(&self,token:&str,error:&str) {
        let mut inner=self.inner.lock().unwrap();
        if let Some(a)=inner.active.as_mut().filter(|a|a.token==token) {
            a.state="error".into();a.input=None;
            let _=self.emit(json!({"op":"stop","token":a.token}));
            // Preserve the underlying Rust range failure, not only ExoPlayer's
            // later generic HTTP/source error.
            if a.error.is_none() {a.error=Some(error.chars().take(240).collect());}
            if let Some(ready)=a.ready.take(){let _=ready.send(Err(a.error.clone().unwrap_or_else(||"media3_failed".into())));}
        }
    }
    fn input(&self,token:&str)->Option<Arc<Input>> {self.inner.lock().unwrap().active.as_ref().filter(|a|a.token==token).and_then(|a|a.input.clone())}
    fn event(&self,event:Event)->bool {
        if !["playing","paused","buffering","ended","error"].contains(&event.state.as_str()) || !event.position.is_finite() || !event.duration.is_finite() || event.position<0.0 || event.duration<0.0 || event.position>604800.0 || event.duration>604800.0 {return false;}
        if !self.inner.lock().unwrap().active.as_ref().is_some_and(|a|a.token==event.token && a.revision==event.revision){return false;}
        if event.state=="error" {if self.input(&event.token).is_none(){return false;}self.fail(&event.token,event.error.as_deref().unwrap_or("Media3 playback failed"));return true;}
        let mut inner=self.inner.lock().unwrap();
        let Some(a)=inner.active.as_mut().filter(|a|a.token==event.token && a.revision==event.revision) else{return false;};
        if a.error.is_some(){return false;}
        a.state=event.state;a.position=event.position;a.duration=event.duration;
        if event.ready {if let Some(ready)=a.ready.take(){let _=ready.send(Ok(()));}}
        true
    }
}
#[derive(Deserialize)] pub struct Event {token:String,#[serde(default)]revision:u64,state:String,#[serde(default)]position:f64,#[serde(default)]duration:f64,#[serde(default)]ready:bool,#[serde(default)]error:Option<String>}
fn running()->Option<&'static Player> {crate::started_player().and_then(|p|p.platform())}
pub async fn receive_event(Json(event):Json<Event>)->StatusCode {
    if running().is_some_and(|p|p.event(event)){StatusCode::NO_CONTENT}else{StatusCode::CONFLICT}
}
fn bounds(header:Option<&str>,len:u64)->Result<(u64,u64,bool),()> {
    if len==0{return Err(());}
    let Some(raw)=header else{return Ok((0,len-1,false));};
    let (first,last)=raw.strip_prefix("bytes=").ok_or(())?.split_once('-').ok_or(())?;
    if first.is_empty() {let suffix=last.parse::<u64>().map_err(|_|())?;if suffix==0{return Err(());}return Ok((len.saturating_sub(suffix),len-1,true));}
    let start=first.parse::<u64>().map_err(|_|())?;
    let end=if last.is_empty(){len-1}else{last.parse::<u64>().map_err(|_|())?.min(len-1)};
    if start>=len || end<start{return Err(());}Ok((start,end,true))
}
pub async fn media(Path(token):Path<String>,method:Method,headers:HeaderMap)->Response {
    let Some(player)=running().cloned() else{return StatusCode::NOT_FOUND.into_response();};
    media_for(player,token,method,headers).await
}
async fn media_for(player:Player,token:String,method:Method,headers:HeaderMap)->Response {
    let Some(input)=player.input(&token) else{return StatusCode::GONE.into_response();};
    let range=headers.get(header::RANGE).and_then(|v|v.to_str().ok());
    let Ok((start,end,partial))=bounds(range,input.len) else{return (StatusCode::RANGE_NOT_SATISFIABLE,[(header::CONTENT_RANGE,format!("bytes */{}",input.len))]).into_response();};
    let length=end-start+1;let total=input.len;
    let body=if method==Method::HEAD {Body::empty()}else{
        Body::from_stream(futures_util::stream::try_unfold((start,input,player,token),move |(position,input,player,token)|async move {
            if position>end{return Ok::<_,io::Error>(None);}
            if player.input(&token).is_none(){return Err(io::Error::other("playback_cancelled"));}
            let last=(position+BLOCK as u64-1).min(end);
            let source=input.clone();
            let result=tokio::task::spawn_blocking(move ||source.fetch(position,last)).await.map_err(|_|io::Error::other("media3_input_worker_failed"))?;
            let block=match result {Ok(block)=>block,Err(error)=>{player.fail(&token,&format!("media3_input_failed: {error}; stage=stream; offset={position}"));return Err(error);}};
            if block.total!=input.len || block.bytes.len() as u64!=last-position+1 {player.fail(&token,"media3_input_range_mismatch");return Err(io::Error::other("media3_input_range_mismatch"));}
            Ok(Some((Bytes::from(block.bytes),(last+1,input,player,token))))
        }))
    };
    let mut response=Response::builder().status(if partial{StatusCode::PARTIAL_CONTENT}else{StatusCode::OK})
        .header(header::CONTENT_TYPE,"audio/mp4").header(header::ACCEPT_RANGES,"bytes")
        .header(header::CACHE_CONTROL,"no-store").header(header::CONTENT_LENGTH,length);
    if partial {response=response.header(header::CONTENT_RANGE,format!("bytes {start}-{end}/{total}"));}
    response.body(body).unwrap_or_else(|_|StatusCode::INTERNAL_SERVER_ERROR.into_response())
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn media3_http_ranges_cover_open_suffix_and_invalid_ranges(){
        assert_eq!(bounds(None,100),Ok((0,99,false)));assert_eq!(bounds(Some("bytes=50-"),100),Ok((50,99,true)));
        assert_eq!(bounds(Some("bytes=-10"),100),Ok((90,99,true)));assert_eq!(bounds(Some("bytes=99-200"),100),Ok((99,99,true)));
        for raw in ["bytes=100-","bytes=20-10","bytes=0-1,3-4","bytes=-0","bad"] {assert!(bounds(Some(raw),100).is_err());}
    }
    #[test] fn stopped_tokens_and_stale_events_cannot_reactivate_playback(){
        let player=Player::new();let event=Event{revision:0,token:"old".into(),state:"playing".into(),position:0.0,duration:1.0,ready:true,error:None};
        assert!(!player.event(event));assert!(player.input("old").is_none());assert_eq!(player.snapshot()["state"],"idle");
    }
}

#[cfg(test)] mod integration_tests {
    use super::*;
    use http_body_util::BodyExt;
    fn fixture(fail:bool)->(Player,oneshot::Receiver<Result<(),String>>) {fixture_size(fail,200000)}
    fn fixture_size(fail:bool,total:u64)->(Player,oneshot::Receiver<Result<(),String>>) {
        let player=Player::new();
        let (requests,rx)=std::sync::mpsc::sync_channel::<Request>(2);
        std::thread::spawn(move ||while let Ok((start,end,reply))=rx.recv(){
            let result=if fail {Err(io::Error::other("fixture_upstream_403"))} else {
                Ok(crate::range_media::Block{bytes:(start..=end).map(|n|(n%251) as u8).collect(),total})
            };let _=reply.send(result);
        });
        let (ready,waiting)=oneshot::channel();
        player.inner.lock().unwrap().active=Some(Active{token:"fixture".into(),revision:0,wants_play:true,id:"dQw4w9WgXcQ".into(),title:None,
            input:Some(Arc::new(Input{requests,len:total})),state:"buffering".into(),error:None,position:0.0,duration:0.0,ready:Some(ready)});
        (player,waiting)
    }
    #[tokio::test] async fn media3_loopback_reads_actual_ranges_head_suffix_and_revokes_stopped_tokens(){
        let (player,_)=fixture(false);
        for (range,start,end) in [("bytes=0-99999",0,99999),("bytes=150000-",150000,199999),("bytes=-100",199900,199999)] {
            let mut headers=HeaderMap::new();headers.insert(header::RANGE,range.parse().unwrap());
            let response=media_for(player.clone(),"fixture".into(),Method::GET,headers).await;
            assert_eq!(response.status(),StatusCode::PARTIAL_CONTENT);
            assert_eq!(response.headers()[header::CONTENT_RANGE],format!("bytes {start}-{end}/200000"));
            let bytes=response.into_body().collect().await.unwrap().to_bytes();
            assert_eq!(bytes.as_ref(),(start..=end).map(|n|(n%251) as u8).collect::<Vec<_>>());
        }
        let response=media_for(player.clone(),"fixture".into(),Method::HEAD,HeaderMap::new()).await;
        assert_eq!(response.headers()[header::CONTENT_LENGTH],"200000");
        assert!(response.into_body().collect().await.unwrap().to_bytes().is_empty());
        player.inner.lock().unwrap().active=None;
        assert_eq!(media_for(player,"fixture".into(),Method::GET,HeaderMap::new()).await.status(),StatusCode::GONE);
    }
    #[tokio::test] async fn long_loopback_body_streams_every_block_and_late_seek_without_whole_file_buffering(){
        let total=10*1024*1024+123;
        let (player,_)=fixture_size(false,total);
        let response=media_for(player.clone(),"fixture".into(),Method::GET,HeaderMap::new()).await;
        assert_eq!(response.headers()[header::CONTENT_LENGTH],total.to_string());
        let mut body=response.into_body();let mut position=0u64;let mut frames=0;
        while let Some(frame)=body.frame().await {
            let frame=frame.unwrap();if let Ok(bytes)=frame.into_data(){
                assert!(bytes.len()<=BLOCK);
                assert!(bytes.iter().enumerate().all(|(i,b)|*b==((position+i as u64)%251) as u8));
                position+=bytes.len() as u64;frames+=1;
            }
        }
        assert_eq!(position,total);assert!(frames>150);
        let mut headers=HeaderMap::new();headers.insert(header::RANGE,format!("bytes={}-",total-100).parse().unwrap());
        let response=media_for(player,"fixture".into(),Method::GET,headers).await;
        assert_eq!(response.status(),StatusCode::PARTIAL_CONTENT);
        assert_eq!(response.into_body().collect().await.unwrap().to_bytes().len(),100);
    }
    #[tokio::test] async fn media3_reports_underlying_input_failure_and_rejects_stale_callbacks(){
        let (player,ready)=fixture(true);
        assert!(!player.event(Event{revision:0,token:"wrong".into(),state:"playing".into(),position:1.0,duration:2.0,ready:true,error:None}));
        let response=media_for(player.clone(),"fixture".into(),Method::GET,HeaderMap::new()).await;
        assert!(response.into_body().collect().await.is_err());
        assert!(ready.await.unwrap().unwrap_err().contains("fixture_upstream_403"));
        assert_eq!(player.snapshot()["state"],"error");
        assert!(!player.event(Event{revision:0,token:"fixture".into(),state:"playing".into(),position:1.0,duration:2.0,ready:true,error:None}));
        assert!(player.snapshot()["error"].as_str().unwrap().contains("fixture_upstream_403"));
    }
    #[tokio::test] async fn media3_ready_event_acknowledges_preparation_but_not_wrong_tokens(){
        let (player,mut ready)=fixture(false);
        assert!(matches!(ready.try_recv(),Err(oneshot::error::TryRecvError::Empty)));
        assert!(player.event(Event{revision:0,token:"fixture".into(),state:"playing".into(),position:0.0,duration:5.0,ready:true,error:None}));
        assert!(ready.await.unwrap().is_ok());
        assert_eq!(player.snapshot()["state"],"playing");
    }
    fn recorded(player:&mut Player)->Arc<Mutex<Vec<Value>>> {
        let log=Arc::new(Mutex::new(Vec::new()));let sink=log.clone();
        player.dispatch=Arc::new(move |value|{sink.lock().unwrap().push(value);Ok(())});log
    }
    struct Rejected;
    impl Fetch for Rejected {fn fetch(&self,_:u64,_:u64)->io::Result<crate::range_media::Block>{Err(io::Error::other("media_range_http_403"))}}
    #[tokio::test] async fn failed_replacement_stops_old_output_before_fetch_and_does_not_leave_orphan_audio(){
        let (mut player,_)=fixture(false);let log=recorded(&mut player);
        let ready=player.play(Media{video_id:"dQw4w9WgXcQ".into(),title:None,client:"fixture",itag:140,url:String::new(),user_agent:"",content_length:None,fetch:Some(Box::new(Rejected)),#[cfg(target_os="windows")] source:None}).unwrap();
        let result=tokio::time::timeout(std::time::Duration::from_secs(2),ready).await.unwrap().unwrap();
        assert!(result.unwrap_err().contains("403"));
        let commands=log.lock().unwrap();
        assert_eq!(commands[0]["op"],"stop");assert_eq!(commands[0]["token"],"fixture");
        assert!(commands.iter().all(|c|c["op"]!="play"));
        assert_eq!(commands.last().unwrap()["op"],"stop");
        assert_eq!(player.snapshot()["state"],"error");
        let token=player.inner.lock().unwrap().active.as_ref().unwrap().token.clone();assert!(player.input(&token).is_none());
    }
    #[tokio::test] async fn seek_preserves_pause_rejects_stale_song_and_invalid_positions() {
        let (mut player,_)=fixture(false);let log=recorded(&mut player);
        assert!(player.event(Event{revision:0,token:"fixture".into(),state:"paused".into(),position:1.0,duration:5.0,ready:true,error:None}));
        let id=player.snapshot()["video_id"].as_str().unwrap().to_owned();
        for position in [f64::NAN,f64::INFINITY,-1.0,6.0] {assert!(player.seek(&id,position).is_err());}
        assert!(player.seek("anotherSong",2.0).is_err());assert!(log.lock().unwrap().is_empty());
        player.seek(&id,3.0).unwrap();assert_eq!(player.snapshot()["state"],"paused");
        assert_eq!(player.snapshot()["position_secs"],3.0);
        assert!(!player.event(Event{revision:0,token:"fixture".into(),state:"playing".into(),position:1.0,duration:5.0,ready:true,error:None}));
        assert_eq!(log.lock().unwrap()[0]["op"],"seek");assert_eq!(log.lock().unwrap()[0]["revision"],1);
    }
    #[tokio::test] async fn pause_resume_revisions_ignore_delayed_playing_and_error_events(){
        let (mut player,_)=fixture(false);let log=recorded(&mut player);
        assert!(player.event(Event{revision:0,token:"fixture".into(),state:"playing".into(),position:1.0,duration:5.0,ready:true,error:None}));
        player.pause().unwrap();assert_eq!(player.snapshot()["state"],"paused");
        for state in ["playing","error"] {
            assert!(!player.event(Event{revision:0,token:"fixture".into(),state:state.into(),position:1.0,duration:5.0,ready:true,error:None}));
        }
        assert_eq!(player.snapshot()["state"],"paused");player.resume().unwrap();
        assert!(player.event(Event{revision:2,token:"fixture".into(),state:"playing".into(),position:2.0,duration:5.0,ready:true,error:None}));
        assert_eq!(player.snapshot()["state"],"playing");player.stop().unwrap();
        assert_eq!(player.snapshot()["state"],"idle");
        assert!(!player.event(Event{revision:2,token:"fixture".into(),state:"playing".into(),position:3.0,duration:5.0,ready:true,error:None}));
        let commands=log.lock().unwrap();assert_eq!(commands[0]["op"],"pause");assert_eq!(commands[0]["revision"],1);
        assert_eq!(commands[1]["op"],"resume");assert_eq!(commands[1]["revision"],2);assert_eq!(commands[2]["op"],"stop");
    }

    struct Waiting(std::sync::Mutex<std::sync::mpsc::Receiver<()>>);
    impl Fetch for Waiting {
        fn fetch(&self,_:u64,_:u64)->io::Result<crate::range_media::Block>{
            self.0.lock().unwrap().recv_timeout(std::time::Duration::from_secs(2)).map_err(|_|io::Error::other("fixture_timeout"))?;
            Ok(crate::range_media::Block{bytes:vec![0;100],total:100})
        }
    }
    #[tokio::test] async fn pause_before_input_ready_is_retained_in_the_later_platform_play_command(){
        let mut player=Player::new();
        let (played,received)=oneshot::channel();let played=Mutex::new(Some(played));
        player.dispatch=Arc::new(move |value|{
            if value["op"]=="play" {if let Some(reply)=played.lock().unwrap().take(){let _=reply.send(value);}}Ok(())
        });
        let (release,waiting)=std::sync::mpsc::channel();
        let _ready=player.play(Media{video_id:"dQw4w9WgXcQ".into(),title:None,client:"fixture",itag:140,url:String::new(),user_agent:"",content_length:None,fetch:Some(Box::new(Waiting(Mutex::new(waiting)))),#[cfg(target_os="windows")] source:None}).unwrap();
        player.pause().unwrap();release.send(()).unwrap();
        let command=tokio::time::timeout(std::time::Duration::from_secs(2),received).await.unwrap().unwrap();
        assert_eq!(command["revision"],1);assert_eq!(command["play_when_ready"],false);
        player.stop().unwrap();
    }

}
