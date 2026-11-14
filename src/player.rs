//! Audio playback: a dedicated rodio/cpal thread fed by a bounded in-memory
//! ring buffer that is filled from the `googlevideo` HTTP stream.
//!
//! Only decoded audio is buffered (8 chunks of ~16 KiB = ~128 KiB), so RSS stays
//! flat no matter how long the track is. The audio thread owns the cpal output
//! stream; the HTTP layer only sends commands.

use rodio::stream::DeviceSinkBuilder;
use rodio::{Decoder, Player as RodioPlayer};
use serde_json::{json, Value};
use std::io::{self, Read, Seek, SeekFrom};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Number of 16 KiB chunks buffered ahead of the decoder (~128 KiB).
const RING_CHUNKS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayState {
    Idle,
    Playing,
    Paused,
    Error,
}

impl PlayState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Playing => "playing",
            Self::Paused => "paused",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Default)]
struct Status {
    state: Option<PlayState>,
    video_id: Option<String>,
    title: Option<String>,
    volume: f32,
    position_secs: f64,
    error: Option<String>,
    client: Option<String>,
    itag: Option<u64>,
}

enum Command {
    Play {
        video_id: String,
        title: Option<String>,
        client: String,
        itag: u64,
        audio: Receiver<io::Result<Vec<u8>>>,
    },
    Pause,
    Resume,
    Stop,
    Volume(f32),
}

/// Cheap, cloneable handle to the playback thread.
#[derive(Clone)]
pub struct Player {
    tx: SyncSender<Command>,
    status: Arc<Mutex<Status>>,
}

impl Player {
    /// Starts the playback thread. The device is opened there, so a missing
    /// output (desktop CI) surfaces as an `error` status instead of a panic.
    pub fn spawn() -> Player {
        let (tx, rx) = mpsc::sync_channel(4);
        let status = Arc::new(Mutex::new(Status {
            state: Some(PlayState::Idle),
            volume: 1.0,
            ..Status::default()
        }));
        let thread_status = Arc::clone(&status);

        std::thread::Builder::new()
            .name("ayshamusic-audio".into())
            .spawn(move || audio_loop(rx, thread_status))
            .expect("failed to spawn ayshamusic audio thread");

        Self { tx, status }
    }

    /// Starts playing a track from a byte stream, replacing whatever is playing.
    pub fn play(
        &self,
        video_id: &str,
        title: Option<String>,
        client: &str,
        itag: u64,
        audio: Receiver<io::Result<Vec<u8>>>,
    ) -> Result<(), String> {
        // Reserve the status before handing over, so `/api/player` never shows
        // the previous track while the new one is being set up.
        {
            let mut status = self.status.lock().unwrap();
            status.video_id = Some(video_id.to_string());
            status.title = title.clone();
            status.client = Some(client.to_string());
            status.itag = Some(itag);
            status.position_secs = 0.0;
            status.error = None;
        }
        self.send(Command::Play {
            video_id: video_id.to_string(),
            title,
            client: client.to_string(),
            itag,
            audio,
        })
    }

    pub fn pause(&self) -> Result<(), String> {
        self.send(Command::Pause)
    }

    pub fn resume(&self) -> Result<(), String> {
        self.send(Command::Resume)
    }

    pub fn stop(&self) -> Result<(), String> {
        self.send(Command::Stop)
    }

    pub fn set_volume(&self, volume: f32) -> Result<(), String> {
        self.send(Command::Volume(volume.clamp(0.0, 1.0)))
    }

    pub fn snapshot(&self) -> Value {
        let status = self.status.lock().unwrap();
        json!({
            "state": status.state.unwrap_or(PlayState::Idle).as_str(),
            "video_id": status.video_id,
            "title": status.title,
            "volume": status.volume,
            "position_secs": status.position_secs,
            "error": status.error,
            "client": status.client,
            "itag": status.itag,
        })
    }

    fn send(&self, command: Command) -> Result<(), String> {
        self.tx
            .send(command)
            .map_err(|_| "player thread is not running".to_string())
    }
}

fn audio_loop(rx: Receiver<Command>, status: Arc<Mutex<Status>>) {
    // Keep the device sink alive for the whole thread; dropping it stops audio.
    let device = match DeviceSinkBuilder::open_default_sink() {
        Ok(device) => device,
        Err(error) => {
            let mut status = status.lock().unwrap();
            status.state = Some(PlayState::Error);
            status.error = Some(format!("audio output unavailable: {error}"));
            return;
        }
    };
    let sink = RodioPlayer::connect_new(device.mixer());

    loop {
        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(Command::Play {
                video_id,
                title,
                client,
                itag,
                audio,
            }) => {
                sink.stop();
                let source = match Decoder::builder()
                    .with_data(ChannelReader::new(audio))
                    .with_hint("m4a")
                    .with_seekable(false)
                    .build()
                {
                    Ok(source) => source,
                    Err(error) => {
                        set_error(&status, format!("audio decode failed: {error}"));
                        continue;
                    }
                };
                let volume = status.lock().unwrap().volume;
                sink.set_volume(volume);
                sink.append(source);
                sink.play();

                let mut status = status.lock().unwrap();
                status.state = Some(PlayState::Playing);
                status.video_id = Some(video_id);
                status.title = title;
                status.client = Some(client);
                status.itag = Some(itag);
                status.position_secs = 0.0;
                status.error = None;
            }
            Ok(Command::Pause) => {
                sink.pause();
                let mut status = status.lock().unwrap();
                if status.state == Some(PlayState::Playing) {
                    status.state = Some(PlayState::Paused);
                }
            }
            Ok(Command::Resume) => {
                sink.play();
                let mut status = status.lock().unwrap();
                if status.state == Some(PlayState::Paused) {
                    status.state = Some(PlayState::Playing);
                }
            }
            Ok(Command::Stop) => {
                sink.stop();
                let mut status = status.lock().unwrap();
                status.state = Some(PlayState::Idle);
                status.position_secs = 0.0;
            }
            Ok(Command::Volume(volume)) => {
                sink.set_volume(volume);
                status.lock().unwrap().volume = volume;
            }
            Err(RecvTimeoutError::Disconnected) => {
                sink.stop();
                return;
            }
            Err(RecvTimeoutError::Timeout) => {}
        }

        // Track ended on its own (decoder hit EOF / stream finished).
        let mut status = status.lock().unwrap();
        if status.state == Some(PlayState::Playing) {
            status.position_secs = sink.get_pos().as_secs_f64();
            if sink.empty() {
                status.state = Some(PlayState::Idle);
                status.position_secs = 0.0;
            }
        } else if status.state == Some(PlayState::Paused) {
            status.position_secs = sink.get_pos().as_secs_f64();
        }
    }
}

fn set_error(status: &Arc<Mutex<Status>>, message: String) {
    let mut status = status.lock().unwrap();
    status.state = Some(PlayState::Error);
    status.error = Some(message);
}

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
