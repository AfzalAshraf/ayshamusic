#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use axum::{body::{to_bytes, Body}, http::{Method, Request}};
use eframe::egui::{self, Color32, RichText};
use serde_json::{json, Value};
use std::{sync::mpsc, time::Duration};
use tower::ServiceExt;

#[derive(Clone)]
struct RequestSpec { kind: &'static str, method: Method, path: String, body: Option<Value> }
struct ResultMessage { kind: &'static str, result: Result<Value, String> }

fn worker() -> Result<(tokio::sync::mpsc::UnboundedSender<RequestSpec>, mpsc::Receiver<ResultMessage>), String> {
    let root = std::env::var_os("LOCALAPPDATA").ok_or("Windows local app data is unavailable")?;
    let root = std::path::PathBuf::from(root).join("AyshaMusicRustDesktop");
    let app=ayshamusic::windows_desktop_router(root.join("data").to_string_lossy().into_owned(),root.join("cache").to_string_lossy().into_owned()).map_err(str::to_owned)?;
    let (tx,mut requests)=tokio::sync::mpsc::unbounded_channel::<RequestSpec>();
    let (results,rx)=mpsc::channel::<ResultMessage>();
    std::thread::Builder::new().name("aysha-desktop-backend".into()).spawn(move || {
        let Ok(runtime)=tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build() else {let _=results.send(ResultMessage{kind:"error",result:Err("Could not start Rust backend".into())});return;};
        runtime.block_on(async move { while let Some(request)=requests.recv().await {
            let app=app.clone();let results=results.clone();
            tokio::spawn(async move {
                let kind=request.kind;
                let response=async {
                    let bytes=request.body.map(|v|v.to_string()).unwrap_or_default();
                    let req=Request::builder().method(request.method).uri(request.path).header("content-type","application/json").body(Body::from(bytes)).map_err(|_|"Invalid desktop request".to_owned())?;
                    let response=app.oneshot(req).await.map_err(|_|"Desktop backend unavailable".to_owned())?;
                    let status=response.status();let bytes=to_bytes(response.into_body(),256*1024).await.map_err(|_|"Backend response too large".to_owned())?;
                    let value:Value=serde_json::from_slice(&bytes).map_err(|_|"Invalid backend response".to_owned())?;
                    if !status.is_success() || value["status"]=="error" || value["status"]=="failed" {return Err(value["detail"].as_str().or_else(||value["error"].as_str()).unwrap_or("Request failed").chars().take(300).collect());}
                    Ok(value)
                }.await;
                let _=results.send(ResultMessage{kind,result:response});
            });
        }});
    }).map_err(|_|"Could not start desktop worker".to_owned())?;
    Ok((tx,rx))
}

struct Aysha {
    tx:tokio::sync::mpsc::UnboundedSender<RequestSpec>,rx:mpsc::Receiver<ResultMessage>,
    query:String,results:Vec<Value>, status:String, player:Value,searching:bool, starting:bool,
}
impl Aysha {
    fn send(&self,kind:&'static str,method:Method,path:String,body:Option<Value>) {let _=self.tx.send(RequestSpec{kind,method,path,body});}
    fn search(&mut self) {
        let q=self.query.trim();if q.is_empty() || q.chars().count()>200 {self.status="Enter a search of 1–200 characters.".into();return;}
        let query=url::form_urlencoded::Serializer::new(String::new()).append_pair("q",q).finish();
        self.searching=true;self.status="Searching…".into();self.send("search",Method::GET,format!("/api/search?{query}"),None);
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
                    "play"=>{self.starting=false;self.status="Playing requested song.".into();},
                    "player"=>self.player=value,
                    _=>{},
                },
                Err(error)=>{if msg.kind=="search"{self.searching=false;}if msg.kind=="play"{self.starting=false;}self.status=error;},
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
            ui.label(&self.status);
            ui.separator();
            let song=self.player["track"]["title"].as_str().or_else(||self.player["title"].as_str()).unwrap_or("Choose a song");
            ui.heading(song);
            let state=self.player["state"].as_str().unwrap_or("idle");
            let seconds=self.player["position_secs"].as_f64().unwrap_or(0.0).max(0.0) as u64;
            ui.label(format!("{} · {}:{:02}",state,seconds/60,seconds%60));
            ui.horizontal(|ui| {
                for (label,route) in [("Previous","/api/prev"),("Pause","/api/pause"),("Resume","/api/resume"),("Next","/api/next"),("Stop","/api/stop")] {
                    if ui.button(label).clicked() {self.send("control",Method::POST,route.into(),Some(json!({})));}
                }
            });
            ui.separator();ui.heading("Search results");
            let mut selected=None;
            egui::ScrollArea::vertical().show(ui,|ui| {for (index,track) in self.results.iter().enumerate() {
                let title=track["title"].as_str().unwrap_or("YouTube track");let artist=track["artist"].as_str().unwrap_or("");
                ui.horizontal(|ui|{if ui.add_enabled(!self.starting,egui::Button::new("Play")).clicked(){selected=Some(index);}ui.label(format!("{title}  ·  {artist}"));});
            }});
            if let Some(index)=selected {self.play(self.results[index].clone());}
        });
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
    let app=Aysha{tx,rx,query:String::new(),results:vec![],status:"Search for a song to begin.".into(),player:json!({}),searching:false,starting:false};
    let options=eframe::NativeOptions{viewport:egui::ViewportBuilder::default().with_inner_size([880.0,680.0]).with_min_inner_size([480.0,420.0]),..Default::default()};
    eframe::run_native("Aysha Music",options,Box::new(|_|Ok(Box::new(app))))
}

#[cfg(test)]
mod tests {
    use super::*;
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
