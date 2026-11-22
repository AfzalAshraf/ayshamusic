//! The two APKs share acquisition, Telegram and UI. Only decode/output differs.
use crate::player::Media;
use serde_json::Value;
use tokio::sync::oneshot;

pub enum Player { Rust(crate::player::Player), Media3(crate::platform_audio::Player) }
impl Player {
    pub fn spawn()->Self {
        if crate::audio_engine_name()=="media3" {Self::Media3(crate::platform_audio::Player::new())}
        else {Self::Rust(crate::player::Player::spawn())}
    }
    pub fn platform(&self)->Option<&crate::platform_audio::Player> {match self {Self::Media3(p)=>Some(p),_=>None}}
    pub fn play(&self,media:Media)->Result<oneshot::Receiver<Result<(),String>>,String> {match self {Self::Rust(p)=>p.play(media),Self::Media3(p)=>p.play(media)}}
    pub fn pause(&self)->Result<(),String> {match self {Self::Rust(p)=>p.pause(),Self::Media3(p)=>p.pause()}}
    pub fn resume(&self)->Result<(),String> {match self {Self::Rust(p)=>p.resume(),Self::Media3(p)=>p.resume()}}
    pub fn stop(&self)->Result<(),String> {match self {Self::Rust(p)=>p.stop(),Self::Media3(p)=>p.stop()}}
    pub fn set_volume(&self,v:f32)->Result<(),String> {match self {Self::Rust(p)=>p.set_volume(v),Self::Media3(p)=>p.set_volume(v)}}
    pub fn snapshot(&self)->Value {
        let mut value=match self {Self::Rust(p)=>p.snapshot(),Self::Media3(p)=>p.snapshot()};
        value["audio_engine"]=serde_json::json!(match self {Self::Rust(_)=>"rust",Self::Media3(_)=>"media3"});value
    }
}
