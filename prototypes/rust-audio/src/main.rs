//! Opt-in local-file playback experiment. No networking, extraction or subprocesses.
use rodio::{Decoder, OutputStreamBuilder, Sink, Source};
use std::{
    error::Error,
    fs::{self, File},
    io::{self, BufReader, Read},
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_COMMAND_BYTES: usize = 128;
const HELP: &str = "Aysha Rust audio prototype (not the production server)\n\
Usage: aysha-audio-prototype --file PATH [--volume 0..1] [--seconds 1..3600]\n\
       [--decode-only] [--interactive]\n\
Defaults: volume 0.2, wall-time limit 60 seconds; local regular files <= 64 MiB.\n\
Formats: WAV/PCM, AAC in M4A/MP4, MP3, FLAC, Ogg/Vorbis. WebM/Opus is not enabled.\n\
--decode-only: decode a bounded duration without opening an audio device.\n\
--interactive: stdin commands pause, resume, volume N, seek SECONDS, status, stop.\n\
EOF in interactive mode stops playback. No mpv/yt-dlp or automatic fallback.\n\
Memory lines report THIS PROCESS only, not the music service or whole host.";

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug, PartialEq)]
struct Options {
    file: PathBuf,
    volume: f32,
    seconds: u64,
    decode_only: bool,
    interactive: bool,
}

fn volume(value: &str) -> std::result::Result<f32, &'static str> {
    let v: f32 = value.parse().map_err(|_| "Volume must be a number in 0..1")?;
    if !v.is_finite() || !(0.0..=1.0).contains(&v) {
        return Err("Volume must be finite and in 0..1");
    }
    Ok(v)
}

fn parse_options(args: impl IntoIterator<Item = String>) -> Result<Option<Options>> {
    let mut args = args.into_iter();
    let (mut file, mut vol, mut seconds) = (None, 0.2, 60);
    let (mut decode_only, mut interactive) = (false, false);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(None),
            "--file" => {
                if file.is_some() { return Err("Specify --file only once".into()); }
                file = Some(PathBuf::from(args.next().ok_or("Missing --file value")?));
            }
            "--volume" => vol = volume(&args.next().ok_or("Missing --volume value")?)?,
            "--seconds" => {
                seconds = args.next().ok_or("Missing --seconds value")?.parse::<u64>()?;
                if !(1..=3600).contains(&seconds) { return Err("Seconds must be in 1..3600".into()); }
            }
            "--decode-only" => decode_only = true,
            "--interactive" => interactive = true,
            _ => return Err("Unknown argument; use --help".into()),
        }
    }
    if decode_only && interactive { return Err("--interactive cannot be combined with --decode-only".into()); }
    Ok(Some(Options { file: file.ok_or("A local --file is required")?, volume: vol, seconds, decode_only, interactive }))
}

fn open_file(path: &PathBuf) -> Result<File> {
    let file = File::open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() { return Err("Input must be a regular file".into()); }
    if meta.len() == 0 || meta.len() > MAX_FILE_BYTES {
        return Err("Input must contain 1 byte to 64 MiB; use a trusted prepared audio file".into());
    }
    Ok(file)
}

#[derive(Debug, PartialEq)]
enum Command {
    Pause,
    Resume,
    Stop,
    Status,
    Volume(f32),
    Seek(Duration),
    Invalid(&'static str),
}

fn parse_command(line: &str) -> Command {
    let mut words = line.split_whitespace();
    let action = words.next().unwrap_or("");
    let arg = words.next();
    if words.next().is_some() { return Command::Invalid("Too many command arguments"); }
    match (action, arg) {
        ("pause", None) => Command::Pause,
        ("resume", None) => Command::Resume,
        ("stop", None) => Command::Stop,
        ("status", None) => Command::Status,
        ("volume", Some(value)) => match volume(value) {
            Ok(value) => Command::Volume(value),
            Err(error) => Command::Invalid(error),
        },
        ("seek", Some(value)) => match value.parse::<u64>() {
            Ok(value) if value <= 3600 => Command::Seek(Duration::from_secs(value)),
            _ => Command::Invalid("Seek must be an integer in 0..3600 seconds"),
        },
        _ => Command::Invalid("Use pause, resume, volume N, seek SECONDS, status or stop"),
    }
}

// Fixed-size line accumulation and a bounded queue; pasted input cannot create an
// unbounded command backlog. The stdin thread exits with the CLI process.
fn read_commands(input: impl Read, mut send: impl FnMut(Command) -> bool) {
    let mut line = Vec::with_capacity(MAX_COMMAND_BYTES);
    let mut oversized = false;
    for byte in input.bytes() {
        let Ok(byte) = byte else { break; };
        if byte == b'\n' {
            let command = if oversized {
                Command::Invalid("Command exceeds 128 bytes")
            } else {
                parse_command(std::str::from_utf8(&line).unwrap_or(""))
            };
            if !send(command) { return; }
            line.clear();
            oversized = false;
        } else if line.len() < MAX_COMMAND_BYTES {
            line.push(byte);
        } else {
            oversized = true;
        }
    }
    if !oversized && !line.is_empty() {
        if !send(parse_command(std::str::from_utf8(&line).unwrap_or(""))) { return; }
    }
    send(Command::Stop);
}

fn stdin_commands() -> Receiver<Command> {
    let (tx, rx) = mpsc::sync_channel(4);
    thread::spawn(move || read_commands(io::stdin().lock(), |command| tx.send(command).is_ok()));
    rx
}

fn memory_report(stage: &str) {
    match fs::read_to_string("/proc/self/status") {
        Ok(status) => {
            let rss = status.lines().find(|l| l.starts_with("VmRSS:")).unwrap_or("VmRSS: unavailable");
            let peak = status.lines().find(|l| l.starts_with("VmHWM:")).unwrap_or("VmHWM: unavailable");
            println!("MEMORY {stage} (prototype process only): {rss}; {peak}");
        }
        Err(_) => println!("MEMORY {stage}: /proc/self/status unavailable; no memory measurement"),
    }
}

fn run(options: Options) -> Result<()> {
    memory_report("startup");
    // BufReader and on-demand Source decoding: no whole-file collect or buffered().
    let file = open_file(&options.file)?;
    let mut builder = Decoder::builder()
        .with_byte_len(file.metadata()?.len())
        .with_seekable(true)
        .with_data(BufReader::with_capacity(32 * 1024, file));
    if let Some(extension) = options.file.extension().and_then(|value| value.to_str()) {
        builder = builder.with_hint(extension);
    }
    let source = builder.build()?;
    let (channels, rate) = (source.channels(), source.sample_rate());
    let duration = source.total_duration();
    println!("DECODE channels={channels} sample_rate={rate} duration={duration:?}");
    let limit = Duration::from_secs(options.seconds);
    if options.decode_only {
        let count = source.take_duration(limit).count();
        if count == 0 { return Err("Decoder produced no audio samples".into()); }
        println!("DECODE_OK samples={count} requested_duration_limit={}s (no audio device opened)", options.seconds);
        memory_report("decode-finished");
        return Ok(());
    }
    // Keep the stream alive until the sink stops. Uses the default OS output,
    // never pairs a device or changes the user's default output automatically.
    let stream = OutputStreamBuilder::open_default_stream()
        .map_err(|e| format!("Cannot open default audio output: {e}. Check the afine audio session and default sink."))?;
    let sink = Sink::connect_new(stream.mixer());
    sink.set_volume(options.volume);
    sink.append(source);
    let commands = options.interactive.then(stdin_commands);
    println!("PLAYING default Linux output; volume={}; wall-time limit={}s", options.volume, options.seconds);
    if options.interactive { println!("Controls: pause | resume | volume 0.2 | seek 10 | status | stop"); }
    let started = Instant::now();
    let mut last_memory = Instant::now();
    while !sink.empty() && started.elapsed() < limit {
        if let Some(rx) = &commands {
            // At most four queued commands are consumed on each tick.
            for command in rx.try_iter().take(4) {
                match command {
                    Command::Pause => { sink.pause(); println!("PAUSED"); }
                    Command::Resume => { sink.play(); println!("RESUMED"); }
                    Command::Volume(value) => { sink.set_volume(value); println!("VOLUME {value}"); }
                    Command::Seek(pos) => {
                        if duration.is_some_and(|duration| pos > duration) {
                            eprintln!("Seek exceeds known track duration");
                        } else if let Err(error) = sink.try_seek(pos) {
                            eprintln!("Seek failed: {error}");
                        } else { println!("SEEK {}s", pos.as_secs()); }
                    }
                    Command::Status => println!("STATUS paused={} position={:?} volume={}", sink.is_paused(), sink.get_pos(), sink.volume()),
                    Command::Stop => { sink.stop(); println!("STOPPED by command/EOF"); }
                    Command::Invalid(error) => eprintln!("Command error: {error}"),
                }
            }
        }
        if last_memory.elapsed() >= Duration::from_secs(5) {
            memory_report("playing");
            last_memory = Instant::now();
        }
        thread::sleep(Duration::from_millis(50));
    }
    if started.elapsed() >= limit { println!("STOPPED at wall-time limit (includes pauses)"); }
    else if sink.empty() { println!("FINISHED/STOPPED (does not independently prove audible output)"); }
    sink.stop();
    drop(sink);
    drop(stream);
    memory_report("playback-finished");
    Ok(())
}

fn main() {
    let result = match parse_options(std::env::args().skip(1)) {
        Ok(None) => { println!("{HELP}"); return; }
        Ok(Some(options)) => run(options),
        Err(error) => Err(error),
    };
    if let Err(error) = result {
        eprintln!("Prototype error: {error}");
        memory_report("error");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(args: &[&str]) -> Result<Option<Options>> {
        parse_options(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn cli_defaults_and_help() {
        let o = options(&["--file", "track.m4a"]).unwrap().unwrap();
        assert_eq!(o.volume, 0.2);
        assert_eq!(o.seconds, 60);
        assert!(!o.decode_only && !o.interactive);
        assert!(options(&["--help"]).unwrap().is_none());
    }

    #[test]
    fn rejects_invalid_options() {
        for args in [vec![], vec!["--file"], vec!["--file", "x", "--seconds", "0"],
            vec!["--file", "x", "--seconds", "3601"], vec!["--file", "x", "--volume", "NaN"],
            vec!["--file", "x", "--volume", "inf"], vec!["--file", "x", "--volume", "-1"],
            vec!["--file", "x", "--unknown"], vec!["--file", "x", "--file", "y"],
            vec!["--file", "x", "--decode-only", "--interactive"]] {
            assert!(options(&args).is_err(), "accepted {args:?}");
        }
    }

    #[test]
    fn validates_commands() {
        assert_eq!(parse_command("pause"), Command::Pause);
        assert_eq!(parse_command("resume"), Command::Resume);
        assert_eq!(parse_command("stop"), Command::Stop);
        assert_eq!(parse_command("status"), Command::Status);
        assert_eq!(parse_command("volume 0.3"), Command::Volume(0.3));
        assert_eq!(parse_command("seek 10"), Command::Seek(Duration::from_secs(10)));
        for command in ["volume NaN", "seek -1", "seek 3601", "pause extra", "", "volume 0.2 extra"] {
            assert!(matches!(parse_command(command), Command::Invalid(_)));
        }
    }

    #[test]
    fn input_lines_are_bounded_and_eof_stops() {
        let input = format!("{}\npause\nresume", "x".repeat(100_000));
        let mut commands = Vec::new();
        read_commands(input.as_bytes(), |command| { commands.push(command); true });
        assert_eq!(commands, vec![Command::Invalid("Command exceeds 128 bytes"), Command::Pause, Command::Resume, Command::Stop]);
    }

    #[test]
    fn decodes_pcm_fixture_without_audio_hardware() {
        let frames = 800_u32;
        let len = frames * 2;
        let mut wav = Vec::new();
        wav.extend(b"RIFF"); wav.extend((36 + len).to_le_bytes()); wav.extend(b"WAVEfmt ");
        wav.extend(16_u32.to_le_bytes()); wav.extend(1_u16.to_le_bytes()); wav.extend(1_u16.to_le_bytes());
        wav.extend(8000_u32.to_le_bytes()); wav.extend(16000_u32.to_le_bytes());
        wav.extend(2_u16.to_le_bytes()); wav.extend(16_u16.to_le_bytes());
        wav.extend(b"data"); wav.extend(len.to_le_bytes()); wav.resize(44 + len as usize, 0);
        let decoder = Decoder::try_from(io::Cursor::new(wav)).unwrap();
        assert_eq!(decoder.channels(), 1);
        assert_eq!(decoder.sample_rate(), 8000);
        assert_eq!(decoder.count(), frames as usize);
    }

    #[test]
    fn invalid_media_is_not_reported_as_decoded() {
        assert!(Decoder::try_from(io::Cursor::new(b"This is not audio".to_vec())).is_err());
    }
}
