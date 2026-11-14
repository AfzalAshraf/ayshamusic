//! Phone-native playback. A bounded seekable HTTP source lets the MP4 demuxer
//! read its index at either end of a file without downloading the whole song.
//! Device and decoder setup run on a dedicated thread, never a Tokio worker.
use crate::range_media::{HttpFetch, RangeReader};
use rodio::stream::DeviceSinkBuilder;
use rodio::{Decoder, Player as RodioPlayer, Source};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::Duration;
// Retained only for the opt-in main media-pump diagnostic, not native playback.
#[cfg(test)] use std::io::{self, Read, Seek, SeekFrom};
use tokio::sync::oneshot;

pub struct Media {
    pub video_id: String,
    pub title: Option<String>,
    pub client: &'static str,
    pub itag: u64,
    pub url: String,
    pub user_agent: &'static str,
    pub content_length: Option<u64>,
    pub fetch: Option<Box<dyn crate::range_media::Fetch>>,
}
#[derive(Default)]
struct Status {
    generation: u64,
    state: &'static str,
    video_id: Option<String>,
    title: Option<String>,
    volume: f32,
    position_secs: f64,
    duration_secs: Option<f64>,
    error: Option<String>,
}
type Ready = oneshot::Sender<Result<(), String>>;
enum Command {
    Play { media: Media, generation: u64, cancel: Arc<AtomicBool>, ready: Ready },
    Pause, Resume, Stop, Volume(f32),
}
#[derive(Clone)]
pub struct Player {
    tx: SyncSender<Command>,
    status: Arc<Mutex<Status>>,
    active: Arc<Mutex<Option<Arc<AtomicBool>>>>,
    epoch: Arc<AtomicU64>,
}
impl Player {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::sync_channel(4);
        let status = Arc::new(Mutex::new(Status { state: "idle", volume: 1.0, ..Status::default() }));
        let thread_status = status.clone();
        std::thread::Builder::new().name("aysha-audio".into())
            .spawn(move || audio_loop(rx, thread_status)).expect("audio thread spawn");
        Self { tx, status, active: Arc::new(Mutex::new(None)), epoch: Arc::new(AtomicU64::new(0)) }
    }
    pub fn play(&self, media: Media) -> Result<oneshot::Receiver<Result<(), String>>, String> {
        let generation = self.epoch.fetch_add(1, Ordering::AcqRel) + 1;
        let cancel = Arc::new(AtomicBool::new(false));
        if let Some(old) = self.active.lock().unwrap().replace(cancel.clone()) { old.store(true, Ordering::Release); }
        {
            let mut state = self.status.lock().unwrap();
            state.generation = generation;
            state.state = "buffering";
            state.video_id = Some(media.video_id.clone());
            state.title = media.title.clone();
            state.position_secs = 0.0;
            state.duration_secs = None;
            state.error = None;
        }
        let (ready, received) = oneshot::channel();
        if let Err(error) = self.send(Command::Play { media, generation, cancel, ready }) {
            set_error(&self.status, generation, error.clone());
            return Err(error);
        }
        Ok(received)
    }
    pub fn pause(&self) -> Result<(), String> { self.send(Command::Pause) }
    pub fn resume(&self) -> Result<(), String> { self.send(Command::Resume) }
    pub fn stop(&self) -> Result<(), String> {
        if let Some(old) = self.active.lock().unwrap().take() { old.store(true, Ordering::Release); }
        let generation = self.epoch.fetch_add(1, Ordering::AcqRel) + 1;
        {
            let mut state = self.status.lock().unwrap();
            state.generation = generation;
            state.state = "idle"; state.position_secs = 0.0; state.error = None;
        }
        self.send(Command::Stop)
    }
    pub fn set_volume(&self, volume: f32) -> Result<(), String> { self.send(Command::Volume(volume.clamp(0.0, 1.0))) }
    pub fn snapshot(&self) -> Value {
        let state = self.status.lock().unwrap();
        json!({"state":state.state,"video_id":state.video_id,"title":state.title,
            "volume":state.volume,"position_secs":state.position_secs,"duration_secs":state.duration_secs,"error":state.error})
    }
    fn send(&self, command: Command) -> Result<(), String> {
        self.tx.try_send(command).map_err(|error| match error {
            mpsc::TrySendError::Full(_) => "player_busy",
            mpsc::TrySendError::Disconnected(_) => "player_thread_unavailable",
        }.to_string())
    }
}
fn set_error(status: &Arc<Mutex<Status>>, generation: u64, message: String) {
    let mut state = status.lock().unwrap();
    if state.generation == generation { state.state = "error"; state.error = Some(message); }
}
fn audio_loop(rx: Receiver<Command>, status: Arc<Mutex<Status>>) {
    // Open lazily, keep alive, and allow another attempt after a device-open error.
    let mut device = None;
    let mut sink: Option<RodioPlayer> = None;
    let mut playing_generation = 0;
    let mut progress: Option<(Arc<crate::pcm_stream::Progress>, rodio::ChannelCount, rodio::SampleRate)> = None;
    let mut health: Option<Arc<Mutex<Option<String>>>> = None;
    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Command::Play { media, generation, cancel, ready }) => {
                if cancel.load(Ordering::Acquire) { let _ = ready.send(Err("playback_cancelled".into())); continue; }
                if let Some(old) = &sink { old.stop(); }
                let prepared = (|| -> Result<_, String> {
                    let fetch: Box<dyn crate::range_media::Fetch> = match media.fetch {
                        Some(fetch) => fetch,
                        None => Box::new(HttpFetch::new(media.url, media.user_agent).map_err(|e| e.to_string())?),
                    };
                    let reader = RangeReader::new(fetch, media.content_length, cancel.clone()).map_err(|e| e.to_string())?;
                    let len = reader.len();
                    let errors = reader.health();
                    let decoder = Decoder::builder().with_data(reader).with_byte_len(len)
                        .with_hint("m4a").with_seekable(true).build()
                        .map_err(|e| format!("audio_decode_failed: {e}"))?;
                    let duration = decoder.total_duration().map(|d| d.as_secs_f64());
                    let pcm = crate::pcm_stream::PcmSource::prepare(decoder, cancel.clone()).map_err(|e| e.to_string())?;
                    if let Some(error) = errors.lock().unwrap().clone() { return Err(error); }
                    if cancel.load(Ordering::Acquire) { return Err("playback_cancelled".into()); }
                    #[cfg(target_os = "android")]
                    if !crate::android::audio_context_ready() { return Err("Android audio context is not initialized; restart the app".into()); }
                    if device.is_none() {
                        device = Some(DeviceSinkBuilder::open_default_sink().map_err(|_| "audio_output_unavailable".to_string())?);
                        sink = Some(RodioPlayer::connect_new(device.as_ref().unwrap().mixer()));
                    }
                    Ok((pcm, duration, errors))
                })();
                match prepared {
                    Ok((pcm, duration, errors)) => {
                        let mut state = status.lock().unwrap();
                        if state.generation != generation || cancel.load(Ordering::Acquire) {
                            let _ = ready.send(Err("playback_cancelled".into())); continue;
                        }
                        state.duration_secs = duration;
                        progress = Some((pcm.progress.clone(), pcm.channels(), pcm.sample_rate()));
                        health = Some(errors);
                        let output = sink.as_ref().unwrap();
                        output.set_volume(state.volume); output.append(pcm); output.play();
                        playing_generation = generation;
                        state.state = "playing"; state.error = None;
                        // HTTP success now means decoder/output accepted the source,
                        // not merely that a URL was resolved. Audibility is separate.
                        let _ = ready.send(Ok(()));
                    }
                    Err(error) => { set_error(&status, generation, error.clone()); let _ = ready.send(Err(error)); }
                }
            }
            Ok(Command::Pause) => {
                if let Some(output) = &sink { output.pause(); }
                let mut state = status.lock().unwrap(); if matches!(state.state, "playing" | "buffering") { state.state = "paused"; }
            }
            Ok(Command::Resume) => {
                if let Some(output) = &sink {
                    let mut state = status.lock().unwrap();
                    if state.state == "paused" { output.play(); state.state = "playing"; }
                }
            }
            Ok(Command::Stop) => { if let Some(output) = &sink { output.stop(); } }
            Ok(Command::Volume(volume)) => {
                if let Some(output) = &sink { output.set_volume(volume); }
                status.lock().unwrap().volume = volume;
            }
            Err(RecvTimeoutError::Disconnected) => { if let Some(output) = &sink { output.stop(); } return; }
            Err(RecvTimeoutError::Timeout) => (),
        }
        let mut state = status.lock().unwrap();
        if state.generation == playing_generation && matches!(state.state, "playing" | "buffering" | "paused") {
            if let Some(output) = &sink {
                if let Some((samples, channels, rate)) = &progress {
                    state.position_secs = samples.seconds(*channels, *rate);
                    if state.state != "paused" {
                        state.state = if samples.starving.load(Ordering::Acquire) { "buffering" } else { "playing" };
                    }
                }
                if let Some(error) = health.as_ref().and_then(|h| h.lock().unwrap().clone()) {
                    state.state = "error"; state.error = Some(error); output.stop();
                } else if state.state != "paused" && output.empty() { state.state = "ended"; }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn commands_fail_fast_instead_of_blocking_a_runtime_worker() {
        let (tx, _rx) = mpsc::sync_channel(1);
        let player = Player { tx, status:Arc::new(Mutex::new(Status::default())), active:Arc::new(Mutex::new(None)),epoch:Arc::new(AtomicU64::new(0)) };
        player.pause().unwrap(); assert_eq!(player.resume().unwrap_err(), "player_busy");
    }
    #[test]
    fn cancelled_decode_cannot_overwrite_new_track_or_stop_state() {
        let status = Arc::new(Mutex::new(Status { generation: 2, state:"idle", ..Status::default() }));
        set_error(&status, 1, "old error".into());
        assert_eq!(status.lock().unwrap().state, "idle");
        set_error(&status, 2, "new error".into());
        assert_eq!(status.lock().unwrap().state, "error");
    }
}

#[cfg(test)] mod fifo_diagnostic {
use super::*;
const RING_CHUNKS: usize = 8;
/// Adapts the downloader's blocking channel into a `Read` for the decoder.
///
/// `rodio`'s decoder wants `Read + Seek + Send + Sync`; the mutex makes the
/// `Receiver` shareable and `seek` reports the stream's honest limitation.
struct ChannelReader {
    rx: Mutex<Receiver<io::Result<Vec<u8>>>>,
    chunk: Vec<u8>,
    offset: usize,
    done: bool,
}

impl ChannelReader {
    fn new(rx: Receiver<io::Result<Vec<u8>>>) -> Self {
        Self {
            rx: Mutex::new(rx),
            chunk: Vec::new(),
            offset: 0,
            done: false,
        }
    }
}

impl Read for ChannelReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            if self.offset < self.chunk.len() {
                let available = &self.chunk[self.offset..];
                let count = available.len().min(buf.len());
                buf[..count].copy_from_slice(&available[..count]);
                self.offset += count;
                return Ok(count);
            }
            if self.done {
                return Ok(0);
            }
            let received = self.rx.lock().unwrap().recv();
            match received {
                Ok(Ok(chunk)) => {
                    self.chunk = chunk;
                    self.offset = 0;
                }
                Ok(Err(error)) => {
                    self.done = true;
                    return Err(io::Error::other(error));
                }
                Err(_) => {
                    self.done = true;
                    return Ok(0);
                }
            }
        }
    }
}

impl Seek for ChannelReader {
    fn seek(&mut self, _position: SeekFrom) -> io::Result<u64> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "live stream is not seekable",
        ))
    }
}

/// Creates the bounded chunk channel used by `ChannelReader`.
pub fn stream_channel() -> (SyncSender<io::Result<Vec<u8>>>, Receiver<io::Result<Vec<u8>>>) {
    mpsc::sync_channel(RING_CHUNKS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reader_streams_chunks_and_reports_eof() {
        let (tx, rx) = stream_channel();
        tx.send(Ok(b"abc".to_vec())).unwrap();
        tx.send(Ok(b"defg".to_vec())).unwrap();
        drop(tx);

        let mut reader = ChannelReader::new(rx);
        let mut out = Vec::new();
        let mut buf = [0u8; 2];
        loop {
            let read = reader.read(&mut buf).unwrap();
            if read == 0 {
                break;
            }
            out.extend_from_slice(&buf[..read]);
        }
        assert_eq!(out, b"abcdefg");
    }

    #[test]
    fn reader_propagates_download_errors() {
        let (tx, rx) = stream_channel();
        tx.send(Ok(b"ab".to_vec())).unwrap();
        tx.send(Err(io::Error::other("HTTP 403"))).unwrap();
        drop(tx);

        let mut reader = ChannelReader::new(rx);
        let mut buf = [0u8; 8];
        assert_eq!(reader.read(&mut buf).unwrap(), 2);
        let error = reader.read(&mut buf).expect_err("download error");
        assert!(error.to_string().contains("403"));
    }
}

}
#[cfg(test)] pub use fifo_diagnostic::stream_channel;
