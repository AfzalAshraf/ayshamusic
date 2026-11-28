//! Decode files the user selected; no file is modified, uploaded or converted.
use std::{fs::File,io::Read,path::Path,sync::{Arc,Mutex}};
use rodio::Source;

type Health=Arc<Mutex<Option<String>>>;
const MAX_LOCAL_FILE:u64=512*1024*1024;
const MAX_WEBM_FILE:u64=64*1024*1024;

pub fn prepare(path:&Path)->Result<(Box<dyn Source+Send>,Health),String> {
    let extension=path.extension().and_then(|v|v.to_str()).unwrap_or("").to_ascii_lowercase();
    if !matches!(extension.as_str(),"mp3"|"wav"|"flac"|"ogg"|"webm") {return Err("local_audio_unsupported".into());}
    let file=File::open(path).map_err(|_|"local_audio_open_failed".to_owned())?;
    let length=file.metadata().map_err(|_|"local_audio_open_failed".to_owned())?.len();
    if length==0 || length>MAX_LOCAL_FILE {return Err("local_audio_size_invalid".into());}
    if extension=="webm" {
        if length>MAX_WEBM_FILE {return Err("local_audio_size_invalid".into());}
        let mut bytes=Vec::new();
        file.take(MAX_WEBM_FILE+1).read_to_end(&mut bytes).map_err(|_|"local_audio_open_failed".to_owned())?;
        if bytes.len() as u64>MAX_WEBM_FILE {return Err("local_audio_size_invalid".into());}
        let (source,health)=crate::opus_source::OpusSource::new(bytes)?;
        return Ok((Box::new(source),health));
    }
    let mut decoder=rodio::Decoder::try_from(file).map_err(|_|"local_audio_decode_failed".to_owned())?;
    // Reject malformed or unsupported files before reporting playback as ready.
    let first=decoder.next().ok_or("local_audio_decode_failed")?;
    let channels=decoder.channels();let rate=decoder.sample_rate();let duration=decoder.total_duration();
    let source=FirstSample{first:Some(first),decoder,channels,rate,duration};
    Ok((Box::new(source),Arc::new(Mutex::new(None))))
}
struct FirstSample<R:Source> {first:Option<f32>,decoder:R,channels:rodio::ChannelCount,rate:rodio::SampleRate,duration:Option<std::time::Duration>}
impl<R:Source> Iterator for FirstSample<R> {type Item=f32;fn next(&mut self)->Option<f32>{self.first.take().or_else(||self.decoder.next())}}
impl<R:Source> Source for FirstSample<R> {
    fn current_span_len(&self)->Option<usize>{None}
    fn channels(&self)->rodio::ChannelCount{self.channels}
    fn sample_rate(&self)->rodio::SampleRate{self.rate}
    fn total_duration(&self)->Option<std::time::Duration>{self.duration}
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn m4a_fixture_decodes_without_conversion() {
        let path=std::env::temp_dir().join(format!("aysha-m4a-decoder-{}.m4a",std::process::id()));
        std::fs::write(&path,include_bytes!("../fixtures/tone.m4a")).unwrap();
        let decoder=rodio::Decoder::try_from(File::open(&path).unwrap()).unwrap();
        let samples:Vec<f32>=decoder.collect();assert!(samples.len()>1000);assert!(samples.iter().any(|v|v.abs()>0.01));
        std::fs::remove_file(path).unwrap();
    }
    #[test] fn decode_real_offline_local_audio_and_reject_invalid_content() {
        for (name,bytes) in [("mp3",include_bytes!("../fixtures/tone.mp3").as_slice()),("wav",include_bytes!("../fixtures/tone.wav").as_slice()),("flac",include_bytes!("../fixtures/tone.flac").as_slice()),("ogg",include_bytes!("../fixtures/tone.ogg").as_slice()),("webm",include_bytes!("../fixtures/tone-opus.webm").as_slice())] {
            let dir=std::env::temp_dir().join(format!("aysha-audio-fixture-{}",std::process::id()));std::fs::create_dir_all(&dir).unwrap();
            let path=dir.join(format!("tone.{name}"));std::fs::write(&path,bytes).unwrap();
            let (source,health)=prepare(&path).unwrap_or_else(|error|panic!("{name}: {error}"));
            let samples:Vec<f32>=source.collect();assert!(samples.len()>1000,"{name}");assert!(samples.iter().any(|s|s.abs()>0.01),"{name}");assert!(health.lock().unwrap().is_none());
            std::fs::write(&path,b"not a valid audio file").unwrap();assert!(prepare(&path).is_err(),"{name}");
            std::fs::remove_file(&path).unwrap();let _=std::fs::remove_dir(&dir);
        }
    }
}
