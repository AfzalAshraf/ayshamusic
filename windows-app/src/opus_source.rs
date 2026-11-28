//! Windows-only WebM/Opus decoding, without transcoding the provider's audio.
//! The compressed input remains bounded; only one decoded packet is held here.
use std::{io::Cursor,sync::{Arc,Mutex},time::Duration};
use rodio::{ChannelCount,SampleRate,Source};
use symphonia::core::{audio::SampleBuffer,codecs::{Decoder,DecoderOptions,CODEC_TYPE_OPUS},errors::Error,formats::{FormatOptions,FormatReader},io::MediaSourceStream,meta::MetadataOptions,probe::Hint};
use symphonia_adapter_libopus::OpusDecoder;

pub struct OpusSource {
    format:Box<dyn FormatReader>, decoder:OpusDecoder, track_id:u32,
    samples:Vec<f32>, index:usize, channels:ChannelCount, rate:SampleRate,
    duration:Option<Duration>, health:Arc<Mutex<Option<String>>>, ended:bool,
}
impl OpusSource {
    pub fn new(bytes:Vec<u8>)->Result<(Self,Arc<Mutex<Option<String>>>),String> {
        if bytes.len()<4 || bytes.len()>64*1024*1024 || bytes[..4]!=[0x1a,0x45,0xdf,0xa3] {return Err("opus_container_invalid".into());}
        let stream=MediaSourceStream::new(Box::new(Cursor::new(bytes)),Default::default());
        let mut hint=Hint::new();hint.with_extension("webm");
        let mut format=symphonia::default::get_probe().format(&hint,stream,&FormatOptions::default(),&MetadataOptions::default()).map_err(|_|"opus_container_invalid".to_owned())?.format;
        let track=format.tracks().iter().find(|t|t.codec_params.codec==CODEC_TYPE_OPUS).ok_or("opus_track_missing")?;
        let track_id=track.id;let params=track.codec_params.clone();
        let duration=params.time_base.zip(params.n_frames).map(|(base,n)|Duration::from(base.calc_time(n))).filter(|d|!d.is_zero());
        let mut decoder=OpusDecoder::try_new(&params,&DecoderOptions::default()).map_err(|_|"opus_decoder_unavailable".to_owned())?;
        // Decode one packet to obtain the actual signal format and reject bad files.
        let first=loop {
            let packet=format.next_packet().map_err(|_|"opus_decode_failed".to_owned())?;
            if packet.track_id()!=track_id {continue;}
            let audio=decoder.decode(&packet).map_err(|_|"opus_decode_failed".to_owned())?;
            let spec=*audio.spec();
            let count=spec.channels.count();
            if count==0 || count>8 || spec.rate==0 || spec.rate>192000 {return Err("opus_audio_parameters_invalid".into());}
            let mut buffer=SampleBuffer::<f32>::new(audio.capacity() as u64,spec);
            buffer.copy_interleaved_ref(audio);
            if !buffer.samples().is_empty(){break (spec,buffer.samples().to_vec());}
        };
        let channels=ChannelCount::new(first.0.channels.count() as u16).ok_or("opus_audio_parameters_invalid")?;
        let rate=SampleRate::new(first.0.rate).ok_or("opus_audio_parameters_invalid")?;
        let health=Arc::new(Mutex::new(None));
        Ok((Self{format,decoder,track_id,samples:first.1,index:0,channels,rate,duration,health:health.clone(),ended:false},health))
    }
    fn next_packet(&mut self)->bool {
        loop {
            let packet=match self.format.next_packet() {
                Ok(packet)=>packet,
                Err(Error::IoError(error)) if error.kind()==std::io::ErrorKind::UnexpectedEof=>{self.ended=true;return false;},
                Err(_)=>{*self.health.lock().unwrap()=Some("opus_read_failed".into());self.ended=true;return false;},
            };
            if packet.track_id()!=self.track_id {continue;}
            let audio=match self.decoder.decode(&packet) {Ok(audio)=>audio,Err(_)=>{*self.health.lock().unwrap()=Some("opus_decode_failed".into());self.ended=true;return false;}};
            let spec=*audio.spec();
            if spec.rate!=self.rate.get() || spec.channels.count()!=usize::from(self.channels.get()) {*self.health.lock().unwrap()=Some("opus_audio_parameters_changed".into());self.ended=true;return false;}
            let mut buffer=SampleBuffer::<f32>::new(audio.capacity() as u64,spec);buffer.copy_interleaved_ref(audio);
            self.samples=buffer.samples().to_vec();self.index=0;
            if !self.samples.is_empty(){return true;}
        }
    }
}
impl Iterator for OpusSource {
    type Item=f32;
    fn next(&mut self)->Option<f32> {
        if self.index>=self.samples.len() && (self.ended || !self.next_packet()){return None;}
        let value=self.samples[self.index];self.index+=1;Some(value)
    }
}
impl Source for OpusSource {
    fn current_span_len(&self)->Option<usize>{None}
    fn channels(&self)->ChannelCount{self.channels}
    fn sample_rate(&self)->SampleRate{self.rate}
    fn total_duration(&self)->Option<Duration>{self.duration}
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn real_offline_webm_opus_decodes_audio_and_rejects_bad_files() {
        let bytes=include_bytes!("../fixtures/tone-opus.webm").to_vec();
        let (source,health)=OpusSource::new(bytes).unwrap();
        assert_eq!(source.sample_rate().get(),48000);
        let samples:Vec<_>=source.collect();assert!(samples.len()>40000);assert!(samples.iter().any(|s|s.abs()>0.01));assert!(health.lock().unwrap().is_none());
        assert!(OpusSource::new(vec![0,1,2,3]).is_err());
    }
}
