#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod diagnostics;
mod local_library;

use axum::{body::{to_bytes, Body}, http::{Method, Request}};
use eframe::egui::{self, Color32, RichText};
use serde_json::{json, Value};
use std::{sync::mpsc, time::Duration};
use tower::ServiceExt;

#[derive(Clone)]
struct RequestSpec { kind: &'static str, method: Method, path: String, body: Option<Value> }
struct ResultMessage { kind: &'static str, result: Result<Value, String>, http_status: Option<u16> }

fn worker() -> Result<(tokio::sync::mpsc::UnboundedSender<RequestSpec>, mpsc::Receiver<ResultMessage>), String> {
    let root = std::env::var_os("LOCALAPPDATA").ok_or("Windows local app data is unavailable")?;
    let root = std::path::PathBuf::from(root).join("AyshaMusicRustDesktop");
    let app=ayshamusic::windows_desktop_router(root.join("data").to_string_lossy().into_owned(),root.join("cache").to_string_lossy().into_owned()).map_err(str::to_owned)?;
    let (tx,mut requests)=tokio::sync::mpsc::unbounded_channel::<RequestSpec>();
    let (results,rx)=mpsc::channel::<ResultMessage>();
    std::thread::Builder::new().name("aysha-desktop-backend".into()).spawn(move || {
        let Ok(runtime)=tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build() else {let _=results.send(ResultMessage{kind:"error",result:Err("Could not start Rust backend".into()),http_status:None});return;};
        let generation=std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        runtime.block_on(async move { while let Some(request)=requests.recv().await {
            use std::sync::atomic::Ordering;
            let changes_playback=request.kind=="play" || request.kind=="local-play" || (request.kind=="control" && request.path=="/api/stop");
            let seq=if changes_playback {generation.fetch_add(1,Ordering::SeqCst)+1} else {generation.load(Ordering::SeqCst)};
            let generation=generation.clone();
            let app=app.clone();let results=results.clone();
            tokio::spawn(async move {
                let kind=request.kind;
                let mut http_status=None;
                let response=async {
                    if kind=="local-scan" {
                        let folder=request.body.as_ref().and_then(|v|v["path"].as_str()).ok_or("local_folder_unavailable")?;
                        let path=std::path::PathBuf::from(folder);
                        return tokio::task::spawn_blocking(move || local_library::scan(&path)).await.map_err(|_|"local_folder_unavailable".to_owned())?;
                    }
                    if kind=="local-play" {
                        let path=request.body.as_ref().and_then(|v|v["path"].as_str()).ok_or("No file selected")?;
                        ayshamusic::windows_play_local_file(std::path::PathBuf::from(path),generation.clone(),seq).await?;
                        return Ok(json!({"status":"playing"}));
                    }
                    let bytes=request.body.map(|v|v.to_string()).unwrap_or_default();
                    let req=Request::builder().method(request.method).uri(request.path).header("content-type","application/json").body(Body::from(bytes)).map_err(|_|"Invalid desktop request".to_owned())?;
                    let response=app.oneshot(req).await.map_err(|_|"Desktop backend unavailable".to_owned())?;
                    let status=response.status();http_status=Some(status.as_u16());let bytes=to_bytes(response.into_body(),256*1024).await.map_err(|_|"Backend response too large".to_owned())?;
                    let value:Value=serde_json::from_slice(&bytes).map_err(|_|"Invalid backend response".to_owned())?;
                    if !status.is_success() || value["status"]=="error" || value["status"]=="failed" {return Err(value["detail"].as_str().or_else(||value["error"].as_str()).unwrap_or("Request failed").chars().take(8192).collect());}
                    Ok(value)
                }.await;
                if !matches!(kind,"play"|"local-play") || generation.load(Ordering::SeqCst)==seq {
                    let _=results.send(ResultMessage{kind,result:response,http_status});
                }
            });
        }});
    }).map_err(|_|"Could not start desktop worker".to_owned())?;
    Ok((tx,rx))
}

fn playback_error(error:&str)->String {
    if error=="local_folder_unavailable" {"Could not read that folder. Choose another folder or check access permissions.".into()}
    else if error.contains("media_range_http_403") {"The provider refused this audio (403). Online playback is unavailable for this request; a local M4A file may still play.".into()}
    else {error.to_owned()}
}

struct Aysha {
    tx:tokio::sync::mpsc::UnboundedSender<RequestSpec>,rx:mpsc::Receiver<ResultMessage>,
    query:String,results:Vec<Value>, status:String, player:Value,searching:bool, starting:bool,
    events:Vec<diagnostics::Event>, show_report:bool, last_player_error:String,
    local_songs:Vec<Value>,scanning:bool,local_notice:String,
}
impl Aysha {
    fn record_error(&mut self,kind:&'static str,error:&str,status:Option<u16>) {
        let event=diagnostics::event(kind,error,status);
        if self.events.len()==10 {self.events.remove(0);}
        self.events.push(event);
    }
    fn send(&self,kind:&'static str,method:Method,path:String,body:Option<Value>) {let _=self.tx.send(RequestSpec{kind,method,path,body});}
    fn search(&mut self) {
        let q=self.query.trim();if q.is_empty() || q.chars().count()>200 {self.status="Enter a search of 1–200 characters.".into();return;}
        let query=url::form_urlencoded::Serializer::new(String::new()).append_pair("q",q).finish();
        self.searching=true;self.status="Searching…".into();self.send("search",Method::GET,format!("/api/search?{query}"),None);
    }
    fn play_local(&mut self,path:&str) {
        if self.starting{return;}
        self.starting=true;self.status="Opening local song…".into();
        self.send("local-play",Method::POST,String::new(),Some(json!({"path":path})));
    }
    fn play(&mut self,track:Value) {
        if self.starting {return;}
        let Some(id)=track["id"].as_str() else{return};
        self.send("play",Method::POST,"/api/play".into(),Some(json!({"video_id":id,"track":track})));
        self.starting=true;self.status="Preparing song…".into();
    }
}
impl eframe::App for Aysha {
    fn update(&mut self,ctx:&egui::Context,_frame:&mut eframe::Frame) {
        while let Ok(msg)=self.rx.try_recv() {
            match msg.result {
                Ok(value)=> match msg.kind {
                    "search"=>{self.searching=false;self.results=value.as_array().cloned().unwrap_or_default();self.status=if self.results.is_empty(){"No songs found."}else{"Choose a song."}.into();},
                    "local-scan"=>{self.scanning=false;self.local_songs=value["items"].as_array().cloned().unwrap_or_default();self.local_notice=if value["truncated"]==true {"Showing up to 500 songs from the first 5,000 entries.".into()} else {format!("{} local M4A songs found.",self.local_songs.len())};},
                    "play"|"local-play"=>{self.starting=false;self.status="Playing requested song.".into();},
                    "player"=>{
                        let error=value["error"].as_str().unwrap_or("").to_owned();
                        if !error.is_empty() && error!=self.last_player_error { self.record_error("player",&error,None); }
                        self.last_player_error=error;
                        self.player=value;
                    },
                    _=>{},
                },
                Err(error)=>{if msg.kind=="local-scan"{self.scanning=false;}if msg.kind=="search"{self.searching=false;}if msg.kind=="play" || msg.kind=="local-play"{self.starting=false;}self.record_error(msg.kind,&error,msg.http_status);self.status=playback_error(&error);},
            }
        }
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(Color32::from_rgb(17,12,18)).inner_margin(28.0)).show(ctx,|ui| {
            ui.heading(RichText::new("Aysha Music").size(30.0).color(Color32::from_rgb(249,112,135)));
            ui.label("Made with ❤️ for Aysha — with love, from Afzal");
            ui.add_space(16.0);
            ui.label(RichText::new("Windows · early Rust desktop candidate").color(Color32::LIGHT_GRAY));
            ui.horizontal(|ui| {
                let text=ui.add(egui::TextEdit::singleline(&mut self.query).hint_text("Search music").desired_width(330.0));
                let enter=text.lost_focus() && ui.input(|i|i.key_pressed(egui::Key::Enter));
                if ui.add_enabled(!self.searching,egui::Button::new("Search")).clicked() || enter {self.search();}
            });
            ui.horizontal(|ui| {
                if ui.add_enabled(!self.starting,egui::Button::new("Open M4A file…")).clicked() {
                    if let Some(path)=rfd::FileDialog::new().add_filter("M4A audio", &["m4a"]).pick_file() {
                        if let Some(path)=path.to_str() {
                            self.play_local(path);
                        } else {self.status="Could not read this file name.".into();}
                    }
                }
                if ui.add_enabled(!self.scanning,egui::Button::new("Choose music folder…")).clicked() {
                    if let Some(folder)=rfd::FileDialog::new().pick_folder() {
                        if let Some(path)=folder.to_str() {
                            self.scanning=true;self.local_notice="Reading folder…".into();
                            self.send("local-scan",Method::POST,String::new(),Some(json!({"path":path})));
                        } else {self.local_notice="Could not read the folder name.".into();}
                    }
                }
            });
            ui.label(&self.status);
            ui.horizontal(|ui| {
                if ui.button("View error report").clicked() {self.show_report=true;}
                if ui.button("Copy error report").clicked() {ctx.copy_text(diagnostics::report(&self.events,&self.player));self.status="Report sent to clipboard. Review it before sharing.".into();}
                if ui.button("Clear errors").clicked() {self.events.clear();}
            });
            ui.separator();
            let song=self.player["track"]["title"].as_str().or_else(||self.player["title"].as_str()).unwrap_or("Choose a song");
            ui.heading(song);
            let state=self.player["state"].as_str().unwrap_or("idle");
            let seconds=self.player["position_secs"].as_f64().unwrap_or(0.0).max(0.0) as u64;
            ui.label(format!("{} · {}:{:02}",state,seconds/60,seconds%60));
            ui.horizontal(|ui| {
                for (label,route) in [("Previous","/api/prev"),("Pause","/api/pause"),("Resume","/api/resume"),("Next","/api/next"),("Stop","/api/stop")] {
                    if ui.button(label).clicked() {if route=="/api/stop" {self.starting=false;self.status="Stopped.".into();}self.send("control",Method::POST,route.into(),Some(json!({})));}
                }
            });
            ui.separator();ui.heading("Local music folder");
            ui.label(if self.local_notice.is_empty(){"Choose a folder containing saved M4A songs. The folder is not saved between sessions."}else{&self.local_notice});
            let mut local_selected=None;
            egui::ScrollArea::vertical().id_salt("local-library").max_height(150.0).show(ui,|ui| {
                for track in &self.local_songs {ui.horizontal(|ui| {
                    if ui.add_enabled(!self.starting,egui::Button::new("Play")).clicked(){local_selected=track["path"].as_str().map(str::to_owned);}
                    ui.label(track["name"].as_str().unwrap_or("Local song"));
                });}
            });
            if let Some(path)=local_selected {self.play_local(&path);}
            ui.separator();ui.heading("Online search results");
            let mut selected=None;
            egui::ScrollArea::vertical().id_salt("online-results").show(ui,|ui| {for (index,track) in self.results.iter().enumerate() {
                let title=track["title"].as_str().unwrap_or("YouTube track");let artist=track["artist"].as_str().unwrap_or("");
                ui.horizontal(|ui|{if ui.add_enabled(!self.starting,egui::Button::new("Play")).clicked(){selected=Some(index);}ui.label(format!("{title}  ·  {artist}"));});
            }});
            if let Some(index)=selected {self.play(self.results[index].clone());}
        });
        if self.show_report {
            let mut open=true;
            egui::Window::new("Diagnostic report").open(&mut open).resizable(true).show(ctx,|ui| {
                ui.label("Review this report before sharing. Raw provider messages and personal data are excluded.");
                let mut report=diagnostics::report(&self.events,&self.player);
                ui.add(egui::TextEdit::multiline(&mut report).desired_width(620.0).desired_rows(16).interactive(false));
                if ui.button("Copy report").clicked(){ctx.copy_text(report);}
            });
            self.show_report=open;
        }
        ctx.request_repaint_after(Duration::from_millis(250));
    }
}

fn main() -> eframe::Result {
    let (tx,rx)=match worker() {Ok(pair)=>pair,Err(error)=>{eprintln!("{error}");return Ok(());}};
    // Poll in a dedicated thread: GUI repaint frequency does not control audio.
    let status=tx.clone();std::thread::spawn(move || loop {
        if status.send(RequestSpec{kind:"player",method:Method::GET,path:"/api/player".into(),body:None}).is_err(){break;}
        std::thread::sleep(Duration::from_secs(1));
    });
    let app=Aysha{tx,rx,query:String::new(),results:vec![],status:"Search for a song to begin.".into(),player:json!({}),searching:false,starting:false,events:Vec::new(),show_report:false,last_player_error:String::new(),local_songs:Vec::new(),scanning:false,local_notice:String::new()};
    let options=eframe::NativeOptions{viewport:egui::ViewportBuilder::default().with_inner_size([880.0,680.0]).with_min_inner_size([480.0,420.0]),..Default::default()};
    eframe::run_native("Aysha Music",options,Box::new(|_|Ok(Box::new(app))))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn provider_403_has_clear_message_and_other_errors_are_preserved() {
        assert!(playback_error("audio_decode_failed; input: media_range_http_403").contains("provider refused"));
        assert_eq!(playback_error("audio_output_unavailable"),"audio_output_unavailable");
    }
    #[tokio::test]
    async fn embedded_backend_rejects_empty_search_and_never_opens_audio() {
        let root=std::env::temp_dir().join(format!("aysha-windows-test-{}",std::process::id()));
        let app=ayshamusic::windows_desktop_router(root.join("data").to_string_lossy().into_owned(),root.join("cache").to_string_lossy().into_owned()).unwrap();
        let request=Request::builder().uri("/api/search?q=").body(Body::empty()).unwrap();
        let response=app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(),axum::http::StatusCode::BAD_REQUEST);
        let request=Request::builder().uri("/api/player").body(Body::empty()).unwrap();
        let response=app.oneshot(request).await.unwrap();
        assert_eq!(response.status(),axum::http::StatusCode::OK);
        let bytes=to_bytes(response.into_body(),4096).await.unwrap();
        let status:Value=serde_json::from_slice(&bytes).unwrap();
        assert_eq!(status["state"],"idle");
    }
}
