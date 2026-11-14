//! Decode/network work is never performed on Android's real-time audio callback.
//! A bounded PCM queue supplies ready samples; starvation yields silence, not a
//! blocking HTTP read. Position counts decoded samples, not starvation silence.
use rodio::{ChannelCount, SampleRate, Source};
use std::io;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering}, mpsc::{self, Receiver, TryRecvError}};
use std::time::Duration;
const CHUNK: usize = 4096;
const QUEUE: usize = 8;
static WORKERS: AtomicUsize = AtomicUsize::new(0);
struct Worker;
impl Drop for Worker { fn drop(&mut self) { WORKERS.fetch_sub(1, Ordering::AcqRel); } }
#[derive(Default)]
pub struct Progress {
    consumed: AtomicU64,
    pub starving: AtomicBool,
}
impl Progress {
    pub fn seconds(&self, channels: ChannelCount, rate: SampleRate) -> f64 {
        self.consumed.load(Ordering::Relaxed) as f64 / f64::from(channels.get()) / f64::from(rate.get())
    }
}
pub struct PcmSource {
    rx: Mutex<Receiver<Vec<f32>>>,
    chunk: Vec<f32>,
    offset: usize,
    channels: ChannelCount,
    rate: SampleRate,
    cancel: Arc<AtomicBool>,
    pub progress: Arc<Progress>,
}
impl PcmSource {
    pub fn prepare<S: Source + Send + 'static>(mut decoder: S, cancel: Arc<AtomicBool>) -> io::Result<Self> {
        let channels = decoder.channels(); let rate = decoder.sample_rate();
        let first: Vec<f32> = decoder.by_ref().take(CHUNK).collect();
        if first.is_empty() || cancel.load(Ordering::Acquire) { return Err(io::Error::other("audio_has_no_decoded_samples_or_cancelled")); }
        WORKERS.fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| (n < 2).then_some(n + 1))
            .map_err(|_| io::Error::other("audio_decoder_busy"))?;
        let worker = Worker;
        let (tx, rx) = mpsc::sync_channel(QUEUE);
        let flag = cancel.clone();
        std::thread::Builder::new().name("aysha-decode".into()).spawn(move || {
            let _worker = worker;
            while !flag.load(Ordering::Acquire) {
                let chunk: Vec<f32> = decoder.by_ref().take(CHUNK).collect();
                if chunk.is_empty() || flag.load(Ordering::Acquire) || tx.send(chunk).is_err() { return; }
            }
        })?;
        Ok(Self { rx:Mutex::new(rx), chunk:first, offset:0, channels, rate, cancel, progress:Arc::new(Progress::default()) })
    }
}
impl Iterator for PcmSource {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.cancel.load(Ordering::Acquire) { return None; }
        if self.offset >= self.chunk.len() {
            match self.rx.lock().unwrap().try_recv() {
                Ok(chunk) => { self.chunk = chunk; self.offset = 0; }
                Err(TryRecvError::Empty) => { self.progress.starving.store(true, Ordering::Release); return Some(0.0); }
                Err(TryRecvError::Disconnected) => return None,
            }
        }
        self.progress.starving.store(false, Ordering::Release);
        let sample = self.chunk[self.offset]; self.offset += 1;
        self.progress.consumed.fetch_add(1, Ordering::Relaxed);
        Some(sample)
    }
}
impl Source for PcmSource {
    fn current_span_len(&self) -> Option<usize> { None }
    fn channels(&self) -> ChannelCount { self.channels }
    fn sample_rate(&self) -> SampleRate { self.rate }
    fn total_duration(&self) -> Option<Duration> { None }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn starvation_never_blocks_audio_callback_or_advances_media_position() {
        let (_tx, rx) = mpsc::sync_channel(1);
        let progress = Arc::new(Progress::default());
        let mut source = PcmSource { rx:Mutex::new(rx), chunk:vec![0.25,0.5],offset:0,
            channels:ChannelCount::new(1).unwrap(),rate:SampleRate::new(44100).unwrap(),cancel:Arc::new(AtomicBool::new(false)),progress:progress.clone() };
        assert_eq!(source.next(), Some(0.25)); assert_eq!(source.next(), Some(0.5));
        for _ in 0..100 { assert_eq!(source.next(), Some(0.0)); }
        assert_eq!(progress.consumed.load(Ordering::Relaxed), 2);
        assert!(progress.starving.load(Ordering::Acquire));
        source.cancel.store(true, Ordering::Release); assert_eq!(source.next(), None);
    }
}
