#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod diagnostics;
mod downloader;
mod local_library;
mod local_audio;
mod opus_source;
mod library_settings;

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
    let library_db=root.join("data").join("windows-library-settings.sqlite3");
    let app=ayshamusic::windows_desktop_router(root.join("data").to_string_lossy().into_owned(),root.join("cache").to_string_lossy().into_owned()).map_err(str::to_owned)?;
    let (tx,mut requests)=tokio::sync::mpsc::unbounded_channel::<RequestSpec>();
    let (results,rx)=mpsc::channel::<ResultMessage>();
    std::thread::Builder::new().name("aysha-desktop-backend".into()).spawn(move || {
        let Ok(runtime)=tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build() else {let _=results.send(ResultMessage{kind:"error",result:Err("Could not start Rust backend".into()),http_status:None});return;};
        let generation=std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let provider_blocked=std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        runtime.block_on(async move { while let Some(request)=requests.recv().await {
            use std::sync::atomic::Ordering;
            let changes_playback=request.kind=="play" || request.kind=="local-play" || (request.kind=="control" && request.path=="/api/stop");
            let seq=if changes_playback {generation.fetch_add(1,Ordering::SeqCst)+1} else {generation.load(Ordering::SeqCst)};
            let generation=generation.clone();let provider_blocked=provider_blocked.clone();let library_db=library_db.clone();
            let app=app.clone();let results=results.clone();
            tokio::spawn(async move {
                let kind=request.kind;
                let mut http_status=None;
                let response=async {
                    if kind=="play" {
                        if provider_blocked.load(Ordering::SeqCst) {return Err("provider_sign_in_or_bot_challenge".into());}
                        let id=request.body.as_ref().and_then(|v|v["video_id"].as_str()).ok_or("invalid_video_id")?;
                        let bytes=downloader::fetch(id,generation.clone(),seq).await?;
                        if bytes.len()>=4 && bytes[..4]==[0x1a,0x45,0xdf,0xa3] {
                            let (source,health)=tokio::task::spawn_blocking(move || opus_source::OpusSource::new(bytes)).await.map_err(|_|"opus_decoder_unavailable".to_owned())??;
                            ayshamusic::windows_play_decoded_source(Box::new(source),health,generation.clone(),seq).await?;
                        } else {ayshamusic::windows_play_audio_bytes(bytes,generation.clone(),seq).await?;}
                        return Ok(json!({"status":"playing"}));
                    }
                    if kind=="local-scan" || kind=="local-restore" || kind=="local-forget" {
                        let folder=request.body.as_ref().and_then(|v|v["path"].as_str()).map(str::to_owned);
                        let save=request.body.as_ref().and_then(|v|v["save"].as_bool()).unwrap_or(false);
                        return tokio::task::spawn_blocking(move || {
                            if kind=="local-forget" {library_settings::forget(&library_db)?;return Ok(json!({"forgotten":true}));}
                            let folder=if kind=="local-restore" {library_settings::load(&library_db)?} else {folder};
                            let Some(folder)=folder else {return Ok(json!({"items":[],"folder":null}));};
                            let mut result=match local_library::scan(std::path::Path::new(&folder)) {
                                Ok(value)=>value,
                                Err(_) if kind=="local-restore"=>return Ok(json!({"items":[],"folder":folder,"scan_error":true})),
                                Err(error)=>return Err(error),
                            };
                            let saved=if save {library_settings::save(&library_db,&folder).is_ok()} else {true};
                            result["folder"]=json!(folder);result["save_failed"]=json!(!saved);
                            Ok(result)
                        }).await.map_err(|_|"local_settings_unavailable".to_owned())?;
                    }
                    if kind=="local-play" {
                        let path=request.body.as_ref().and_then(|v|v["path"].as_str()).ok_or("No file selected")?;
                        let path=std::path::PathBuf::from(path);
                        if path.extension().and_then(|v|v.to_str()).is_some_and(|v|v.eq_ignore_ascii_case("m4a")) {
                            ayshamusic::windows_play_local_file(path,generation.clone(),seq).await?;
                        } else {
                            let (source,health)=tokio::task::spawn_blocking(move || local_audio::prepare(&path)).await.map_err(|_|"local_audio_open_failed".to_owned())??;
                            ayshamusic::windows_play_decoded_source(source,health,generation.clone(),seq).await?;
                        }
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
                if let Err(ref error)=response {
                    if kind=="play" && (online_challenge(error)) {provider_blocked.store(true,Ordering::SeqCst);}
                }
                if !matches!(kind,"play"|"local-play") || generation.load(Ordering::SeqCst)==seq {
                    let _=results.send(ResultMessage{kind,result:response,http_status});
                }
            });
        }});
    }).map_err(|_|"Could not start desktop worker".to_owned())?;
    Ok((tx,rx))
}

fn online_challenge(error:&str)->bool {
    error.split(", ").any(|code| matches!(code,"ytdlp_sign_in_or_bot_challenge"|"provider_sign_in_or_bot_challenge"))
}

fn playback_error(error:&str)->String {
    if online_challenge(error) {"The provider requires sign-in or a bot check. Online playback has stopped; no bypass was attempted.".into()}
    else if error=="ytdlp_missing" {"yt-dlp.exe is missing. Extract the complete ZIP and keep both executables together.".into()}
    else if error=="ytdlp_js_runtime_missing" {"deno.exe is missing. Extract the complete ZIP and keep all executable files together.".into()}
    else if error.contains("ytdlp_images_only") {"The online source returned images but no audio. Nothing was downloaded; local files can still play.".into()}
    else if error.starts_with("ytdlp_") {format!("Online audio could not be prepared ({error}). View the error report for details.")}
    else if error.starts_with("local_settings_") {"Could not read or save the music folder setting. Check app storage access.".into()}
    else if error=="local_audio_decode_failed" || error=="local_audio_unsupported" || error.starts_with("opus_") {"This file's audio format or codec is not supported, or the file is damaged.".into()}
    else if error=="local_audio_size_invalid" {"This audio file is empty or exceeds the supported size limit.".into()}
    else if error=="local_audio_open_failed" {"Could not open the selected audio file.".into()}
    else if error=="local_folder_unavailable" {"Could not read that folder. Choose another folder or check access permissions.".into()}
    else if error.contains("media_range_http_403") {"The provider refused this audio (403). Online playback is unavailable for this request; a local M4A file may still play.".into()}
    else {error.to_owned()}
}

struct Aysha {
    tx:tokio::sync::mpsc::UnboundedSender<RequestSpec>,rx:mpsc::Receiver<ResultMessage>,
    query:String,results:Vec<Value>, status:String, player:Value,searching:bool, starting:bool,
    events:Vec<diagnostics::Event>, show_report:bool, last_player_error:String,
    local_songs:Vec<Value>,scanning:bool,local_notice:String,local_folder:Option<String>,provider_blocked:bool,
    page:Page, pending_title:String, now_playing:String,
}
#[derive(Clone,Copy,PartialEq,Eq)]
enum Page { Home, Search, Library, Settings }
const ROSE:Color32=Color32::from_rgb(248,105,130);
const MUTED:Color32=Color32::from_rgb(169,158,169);
fn clock(seconds:f64)->String {
    let seconds=if seconds.is_finite() && seconds>=0.0 {seconds as u64}else{0};
    format!("{}:{:02}",seconds/60,seconds%60)
}
fn navigation(ui:&mut egui::Ui,page:&mut Page) {
    for (name,target) in [("Home",Page::Home),("Search",Page::Search),("Your library",Page::Library),("Settings",Page::Settings)] {
        if ui.selectable_label(*page==target,name).clicked(){*page=target;}
    }
}
fn configure_style(ctx:&egui::Context) {
    let mut style=(*ctx.style()).clone();
    style.visuals=egui::Visuals::dark();
    style.visuals.override_text_color=Some(Color32::from_rgb(245,239,243));
    style.visuals.panel_fill=Color32::from_rgb(17,14,19);
    style.visuals.window_fill=Color32::from_rgb(31,25,32);
    style.visuals.widgets.noninteractive.bg_fill=Color32::from_rgb(33,27,34);
    style.visuals.widgets.inactive.bg_fill=Color32::from_rgb(42,34,43);
    style.visuals.widgets.hovered.bg_fill=Color32::from_rgb(75,52,63);
    style.visuals.widgets.active.bg_fill=Color32::from_rgb(113,54,71);
    style.visuals.selection.bg_fill=Color32::from_rgb(150,55,82);
    style.spacing.item_spacing=egui::vec2(12.0,10.0);
    style.spacing.button_padding=egui::vec2(14.0,9.0);
    ctx.set_style(style);
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
        self.pending_title=std::path::Path::new(path).file_stem().and_then(|v|v.to_str()).unwrap_or("Local song").chars().take(200).collect();
        self.starting=true;self.status="Opening local song…".into();
        self.send("local-play",Method::POST,String::new(),Some(json!({"path":path})));
    }
    fn play(&mut self,track:Value) {
        if self.starting || self.provider_blocked {return;}
        let Some(id)=track["id"].as_str() else{return};
        self.pending_title=track["title"].as_str().unwrap_or("Online song").chars().take(200).collect();
        self.send("play",Method::POST,"/api/play".into(),Some(json!({"video_id":id,"track":track})));
        self.starting=true;self.status="Preparing audio with yt-dlp… This uses data and may take time.".into();
    }
}
impl eframe::App for Aysha {
    fn update(&mut self,ctx:&egui::Context,_frame:&mut eframe::Frame) {
        while let Ok(msg)=self.rx.try_recv() {
            match msg.result {
                Ok(value)=> match msg.kind {
                    "search"=>{self.searching=false;self.results=value.as_array().cloned().unwrap_or_default();self.status=if self.results.is_empty(){"No songs found."}else{"Choose a song."}.into();},
                    "local-scan"|"local-restore"=>{
                        self.scanning=false;self.local_songs=value["items"].as_array().cloned().unwrap_or_default();
                        self.local_folder=value["folder"].as_str().map(str::to_owned);
                        self.local_notice=if value["scan_error"]==true {"Saved folder is unavailable. Reconnect it, refresh, choose another, or forget it.".into()} else if value["save_failed"]==true {"Folder opened, but could not be remembered. Check app storage access.".into()} else if self.local_folder.is_none(){"Choose a music folder to browse your saved songs.".into()} else if value["truncated"]==true {"Showing up to 500 songs from the first 5,000 entries.".into()} else {format!("{} local audio files found.",self.local_songs.len())};
                    },
                    "local-forget"=>{self.scanning=false;self.local_songs.clear();self.local_folder=None;self.local_notice="Folder forgotten. Your music files were not deleted.".into();},
                    "play"|"local-play"=>{self.starting=false;self.now_playing=self.pending_title.clone();self.status="Playing requested song.".into();},
                    "player"=>{
                        let error=value["error"].as_str().unwrap_or("").to_owned();
                        if !error.is_empty() && error!=self.last_player_error { self.record_error("player",&error,None); }
                        self.last_player_error=error;
                        self.player=value;
                    },
                    _=>{},
                },
                Err(error)=>{if msg.kind=="play" && (online_challenge(&error)) {self.provider_blocked=true;}if matches!(msg.kind,"local-scan"|"local-restore"|"local-forget"){self.scanning=false;self.local_notice=playback_error(&error);}if msg.kind=="search"{self.searching=false;}if msg.kind=="play" || msg.kind=="local-play"{self.starting=false;self.now_playing.clear();}self.record_error(msg.kind,&error,msg.http_status);self.status=playback_error(&error);},
            }
        }
        let wide=ctx.screen_rect().width()>=700.0;
        egui::TopBottomPanel::top("topbar").frame(egui::Frame::NONE.fill(Color32::from_rgb(24,19,25)).inner_margin(egui::Margin::symmetric(24,16))).show(ctx,|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("♪").size(27.0).color(ROSE));
                ui.label(RichText::new("AYSHA").size(21.0).strong());
                ui.label(RichText::new("MUSIC").size(12.0).color(ROSE).strong());
                ui.add_space(24.0);
                if !wide { navigation(ui,&mut self.page); }
                else { ui.label(RichText::new("YOUR MUSIC, YOUR MOMENT").size(11.0).color(MUTED)); }
            });
        });
        egui::TopBottomPanel::bottom("player-bar").frame(egui::Frame::NONE.fill(Color32::from_rgb(29,23,30)).inner_margin(egui::Margin::symmetric(20,12))).show(ctx,|ui| {
            let state=self.player["state"].as_str().unwrap_or("idle").to_owned();
            let position=self.player["position_secs"].as_f64().unwrap_or(0.0).max(0.0);
            let duration=self.player["duration_secs"].as_f64().filter(|d|d.is_finite() && *d>0.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("♪").size(26.0).color(ROSE));
                ui.vertical(|ui| {
                    ui.label(RichText::new(if self.now_playing.is_empty(){"Nothing playing"}else{&self.now_playing}).strong().size(15.0));
                    ui.label(RichText::new(if self.starting {"Preparing audio"} else {state.as_str()}).size(11.0).color(MUTED));
                });
                ui.add_space(20.0);
                let (label,route)=if state=="playing" {("Pause","/api/pause")}else{("Play","/api/resume")};
                if ui.add_enabled(!self.starting && (state=="playing" || state=="paused"),egui::Button::new(label)).clicked(){self.send("control",Method::POST,route.into(),Some(json!({})));}
                if ui.add_enabled(self.starting || state=="playing" || state=="paused",egui::Button::new("Stop")).clicked(){
                    self.starting=false;self.now_playing.clear();self.status="Stopped.".into();self.send("control",Method::POST,"/api/stop".into(),Some(json!({})));
                }
                ui.label(RichText::new(format!("{} / {}",clock(position),duration.map(clock).unwrap_or_else(||"--:--".into()))).color(MUTED).size(12.0));
            });
            let fraction=duration.map(|d|(position/d).clamp(0.0,1.0) as f32).unwrap_or(0.0);
            ui.add(egui::ProgressBar::new(fraction).desired_width(ui.available_width()).fill(ROSE));
        });
        if wide {
            egui::SidePanel::left("navigation").resizable(false).exact_width(190.0).frame(egui::Frame::NONE.fill(Color32::from_rgb(22,18,23)).inner_margin(egui::Margin::symmetric(18,24))).show(ctx,|ui| {
                ui.label(RichText::new("DISCOVER").size(10.0).color(MUTED).strong());ui.add_space(8.0);
                ui.vertical(|ui|navigation(ui,&mut self.page));
                ui.add_space(28.0);ui.separator();ui.add_space(12.0);
                ui.label(RichText::new("Made with ❤️ for Aysha — with love, from Afzal").size(12.0).color(MUTED));
            });
        }
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(Color32::from_rgb(17,14,19)).inner_margin(egui::Margin::symmetric(28,24))).show(ctx,|ui| {
            egui::ScrollArea::vertical().id_salt("main-content").show(ui,|ui| {
                match self.page {
                    Page::Home=>{
                        ui.label(RichText::new("A LITTLE SPACE FOR YOUR MUSIC").size(11.0).color(ROSE).strong());
                        ui.add_space(8.0);ui.label(RichText::new("Music feels better here.").size(32.0).strong());
                        ui.label(RichText::new("Search for something you love, or listen to the music already on your PC.").color(MUTED).size(15.0));
                        ui.add_space(20.0);
                        egui::Frame::NONE.fill(Color32::from_rgb(51,31,41)).corner_radius(14).inner_margin(20).show(ui,|ui| {
                            ui.set_min_width((ui.available_width()-40.0).max(100.0));
                            ui.label(RichText::new("YOUR MUSIC, READY WHEN YOU ARE").size(11.0).color(ROSE).strong());
                            ui.add_space(8.0);ui.label(RichText::new("Make yourself at home.").size(22.0).strong());
                            ui.label(RichText::new("Your saved audio lives in Your library.").color(MUTED));ui.add_space(10.0);
                            ui.horizontal_wrapped(|ui|{
                                if ui.button("Open your library").clicked(){self.page=Page::Library;}
                                if ui.button("Search music").clicked(){self.page=Page::Search;}
                            });
                        });
                        ui.add_space(22.0);ui.label(RichText::new("Your library").size(20.0).strong());
                        ui.label(RichText::new(&self.local_notice).color(MUTED));
                        ui.add_space(12.0);ui.label(RichText::new(&self.status).color(MUTED));
                    },
                    Page::Search=>{
                        ui.label(RichText::new("DISCOVER").size(11.0).color(ROSE).strong());
                        ui.label(RichText::new("Search music").size(30.0).strong());
                        ui.label(RichText::new("Find a song or artist.").color(MUTED));ui.add_space(12.0);
                        ui.horizontal(|ui|{
                            let text=ui.add(egui::TextEdit::singleline(&mut self.query).hint_text("Search songs and artists").desired_width((ui.available_width()-110.0).clamp(100.0,500.0)));
                            let enter=text.lost_focus() && ui.input(|i|i.key_pressed(egui::Key::Enter));
                            if ui.add_enabled(!self.searching,egui::Button::new(if self.searching {"Searching…"}else{"Search"})).clicked() || (enter && !self.searching) {self.search();}
                        });
                        ui.add_space(16.0);ui.label(RichText::new(&self.status).color(MUTED));
                        if self.provider_blocked {ui.label(RichText::new("The provider requested sign-in or a bot check. Online playback is paused for this session.").color(ROSE));}
                        ui.add_space(14.0);
                        if !self.results.is_empty() {ui.label(RichText::new("Songs").size(21.0).strong());}
                        let mut selected=None;
                        for (index,track) in self.results.iter().enumerate() {
                            let title=track["title"].as_str().unwrap_or("Online track");let artist=track["artist"].as_str().unwrap_or("");
                            egui::Frame::NONE.fill(Color32::from_rgb(31,26,32)).corner_radius(8).inner_margin(12).show(ui,|ui| {
                                ui.set_min_width((ui.available_width()-24.0).max(100.0));
                                ui.horizontal_wrapped(|ui| {
                                    if ui.add_enabled(!self.starting && !self.provider_blocked,egui::Button::new("Play")).clicked(){selected=Some(index);}
                                    ui.vertical(|ui|{ui.label(RichText::new(title).strong());if !artist.is_empty(){ui.label(RichText::new(artist).color(MUTED).size(12.0));}});
                                });
                            });
                        }
                        if let Some(index)=selected {self.play(self.results[index].clone());}
                    },
                    Page::Library=>{
                        ui.label(RichText::new("YOUR COLLECTION").size(11.0).color(ROSE).strong());
                        ui.label(RichText::new("Your library").size(30.0).strong());
                        ui.label(RichText::new("Play audio files saved on this PC. Your files stay where they are.").color(MUTED));ui.add_space(16.0);
                        ui.horizontal_wrapped(|ui| {
                            if ui.add_enabled(!self.starting,egui::Button::new("Open audio file…")).clicked() {
                                if let Some(path)=rfd::FileDialog::new().add_filter("Supported audio", &["m4a", "mp3", "wav", "flac", "ogg", "webm"]).pick_file() {
                                    if let Some(path)=path.to_str(){self.play_local(path);} else {self.status="Could not read this file name.".into();}
                                }
                            }
                            if ui.add_enabled(!self.scanning,egui::Button::new("Choose music folder…")).clicked() {
                                if let Some(folder)=rfd::FileDialog::new().pick_folder() {
                                    if let Some(path)=folder.to_str() {self.scanning=true;self.local_notice="Reading folder…".into();self.send("local-scan",Method::POST,String::new(),Some(json!({"path":path,"save":true})));}
                                    else {self.local_notice="Could not read the folder name.".into();}
                                }
                            }
                            if ui.add_enabled(!self.scanning && self.local_folder.is_some(),egui::Button::new("Refresh")).clicked() {
                                if let Some(path)=self.local_folder.clone(){self.scanning=true;self.local_notice="Reading folder…".into();self.send("local-scan",Method::POST,String::new(),Some(json!({"path":path,"save":false})));}
                            }
                            if ui.add_enabled(!self.scanning && self.local_folder.is_some(),egui::Button::new("Forget folder")).clicked(){self.scanning=true;self.send("local-forget",Method::POST,String::new(),None);}
                        });
                        ui.add_space(10.0);ui.label(RichText::new(&self.local_notice).color(MUTED));ui.label(RichText::new(&self.status).color(MUTED));ui.add_space(12.0);
                        let mut selected=None;
                        for (index,track) in self.local_songs.iter().enumerate() {
                            egui::Frame::NONE.fill(Color32::from_rgb(31,26,32)).corner_radius(8).inner_margin(12).show(ui,|ui|{
                                ui.set_min_width((ui.available_width()-24.0).max(100.0));
                                ui.horizontal_wrapped(|ui|{
                                    if ui.add_enabled(!self.starting,egui::Button::new("Play")).clicked(){selected=track["path"].as_str().map(str::to_owned);}
                                    ui.label(RichText::new(format!("{:02}",index+1)).color(MUTED));ui.label(RichText::new(track["name"].as_str().unwrap_or("Local song")).strong());
                                });
                            });
                        }
                        if let Some(path)=selected{self.play_local(&path);}
                    },
                    Page::Settings=>{
                        ui.label(RichText::new("YOUR APP").size(11.0).color(ROSE).strong());
                        ui.label(RichText::new("Settings").size(30.0).strong());ui.add_space(8.0);
                        ui.label(RichText::new("Diagnostics").size(20.0).strong());
                        ui.label(RichText::new("Reports stay on this device unless you choose to copy them.").color(MUTED));
                        ui.horizontal_wrapped(|ui|{
                            if ui.button("View error report").clicked(){self.show_report=true;}
                            if ui.button("Copy error report").clicked(){ctx.copy_text(diagnostics::report(&self.events,&self.player));self.status="Report sent to clipboard. Review it before sharing.".into();}
                            if ui.button("Clear errors").clicked(){self.events.clear();}
                        });
                        ui.add_space(18.0);ui.label(RichText::new("About").size(20.0).strong());
                        ui.label(RichText::new(format!("Aysha Music for Windows · v{}",env!("CARGO_PKG_VERSION"))).color(MUTED));
                        ui.label(RichText::new("Early candidate · online playback may be unavailable").color(MUTED));
                        ui.add_space(18.0);ui.label("Made with ❤️ for Aysha — with love, from Afzal");
                    },
                }
            });
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
    let _=tx.send(RequestSpec{kind:"local-restore",method:Method::GET,path:String::new(),body:None});
    let app=Aysha{tx,rx,query:String::new(),results:vec![],status:"Search for a song to begin.".into(),player:json!({}),searching:false,starting:false,events:Vec::new(),show_report:false,last_player_error:String::new(),local_songs:Vec::new(),scanning:true,local_notice:"Loading saved music folder…".into(),local_folder:None,provider_blocked:false,page:Page::Home,pending_title:String::new(),now_playing:String::new()};
    let options=eframe::NativeOptions{viewport:egui::ViewportBuilder::default().with_inner_size([1120.0,760.0]).with_min_inner_size([580.0,480.0]),..Default::default()};
    eframe::run_native("Aysha Music",options,Box::new(|cc|{configure_style(&cc.egui_ctx);Ok(Box::new(app))}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn challenge_is_terminal_but_format_error_is_not_challenge() {
        assert!(online_challenge("ytdlp_sign_in_or_bot_challenge"));
        assert!(online_challenge("provider_sign_in_or_bot_challenge"));
        assert!(!online_challenge("ytdlp_format_unavailable"));
    }
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
