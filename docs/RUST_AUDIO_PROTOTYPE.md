# First native Rust audio prototype

Status: implemented as a separate experimental command, not enabled in music-server. See the latest QA ledger for CI results and target-hardware limits. User approved trying the first audio-only prototype; this is not approval to replace production extraction, change listening semantics or enforce an unmeasured memory cap.

## What it does

`prototypes/rust-audio` is an isolated Cargo package using Rodio 0.21.1 with Symphonia-backed WAV/PCM, AAC/M4A, MP3, FLAC and Ogg/Vorbis decoding. It reads a **trusted local audio file**, decodes on demand and plays to Linux's default output. It never executes mpv, yt-dlp, Node, ffmpeg or a shell, and it does not access the network. This replaces the audio-only playback role in the experiment, not extraction of a YouTube page.

It provides pause, resume, volume, seek, status and stop through stdin, plus a wall-time limit (60 seconds by default, including pauses) and a decode-only headless mode. Memory lines show the prototype process's current RSS and high-water RSS from `/proc/self/status`, or explicitly report unavailable data. KiB values are not decimal MB.

The production API/UI/systemd service remains unchanged and still uses mpv/yt-dlp. The prototype is not built by the root `cargo build --release` or installed by `scripts/install_local.sh`. No physical output is opened by its headless decode mode. Do not confuse successful decoding/null-output playback with audible sound on the JBL.

## Build on the existing Ubuntu host

Run as `afine` in the existing branch checkout:

```bash
cd /home/afine/ayshamusic &&
git pull --ff-only origin arena/a850fb0d-ayshamusic
```

For this optional Linux audio build only, install the ALSA development headers and pkg-config if missing. These are build prerequisites, not a new music server:

```bash
sudo apt-get update &&
sudo apt-get install -y libasound2-dev pkg-config
```

Then test and build the **isolated** package (not the production installer):

```bash
cd /home/afine/ayshamusic &&
cargo test --manifest-path prototypes/rust-audio/Cargo.toml &&
cargo build --release --manifest-path prototypes/rust-audio/Cargo.toml
```

Executable: `prototypes/rust-audio/target/release/aysha-audio-prototype`.

## Prepare one audio file, separately from Rust playback

If you already have a trusted WAV, MP3 or AAC/M4A file <= 64 MiB, use that and skip extraction entirely.

Otherwise this example uses the **existing yt-dlp/Node tools** to prepare the song from the user's diagnostic logs. It is not a Rust extractor and its peak memory is separate from playback. Only use content you are permitted to access. Save outside the repository:

```bash
mkdir -p /home/afine/.local/share/aysha-audio-prototype &&
yt-dlp --js-runtimes node:/usr/bin/node --no-playlist --no-progress \
  --socket-timeout 15 --retries 1 --max-filesize 64M \
  -f 'bestaudio[ext=m4a][acodec^=mp4a]' \
  -o '/home/afine/.local/share/aysha-audio-prototype/test.%(ext)s' \
  -- 'https://www.youtube.com/watch?v=Jm7Satp7Zqk'
```

If no such format is available, preparation fails visibly: use another supported local file rather than renaming WebM as M4A. No transcoding/fallback is hidden. The downloader's size flag may depend on available size metadata; the prototype independently rejects files over 64 MiB. WebM/Opus format 251 is **not supported by this prototype**. This is a deliberate, limited decoder selection, not a drop-in mpv replacement.

## First check: headless decode

No audio device or speaker is touched:

```bash
cd /home/afine/ayshamusic &&
./prototypes/rust-audio/target/release/aysha-audio-prototype \
  --file /home/afine/.local/share/aysha-audio-prototype/test.m4a \
  --decode-only --seconds 10
```

Expected: `DECODE_OK samples=...` with a nonzero sample count and a memory report. This tests the requested decoded duration, not all bytes in the file or audible output. A missing/empty/oversized/unsupported file exits nonzero.

## Second check: actual Linux playback

Pause existing music first to avoid mixing two players. The prototype does not kill or control the production player. It uses the **current default Linux audio output**, not automatically the home Bluetooth speaker. Pair/select the JBL through the existing host audio settings first if that is the desired destination; this program does not change devices/pairing.

Run from the afine SSH shell; these are the same user-audio session paths used by the existing service:

```bash
cd /home/afine/ayshamusic &&
XDG_RUNTIME_DIR=/run/user/1000 \
PULSE_SERVER=unix:/run/user/1000/pulse/native \
./prototypes/rust-audio/target/release/aysha-audio-prototype \
  --file /home/afine/.local/share/aysha-audio-prototype/test.m4a \
  --volume 0.2 --seconds 60 --interactive
```

The ALSA backend/default configuration must route into the user's audio stack; setting Pulse environment variables alone does not create that route. If it cannot open the default output, the program exits with a diagnostic. Inspect `wpctl status` or `pactl get-default-sink` if installed; do not blindly reset pairing or make system-wide ALSA configuration changes.

Type one command per line, pressing Enter:

```text
status
pause
resume
volume 0.1
seek 10
stop
```

Volume must be finite and within 0..1. Seek is an integer in 0..3600 seconds, is rejected past a known track duration, and may fail for a particular decoder/container. Errors are printed without silently falling back to mpv. Stdin command lines/queue are bounded; EOF in interactive mode stops the program. The wall-time limit includes paused time. This foreground prototype is not designed to keep playing after SSH disconnects; the existing systemd player is independent of this test.

## Measurements and acceptance

Send the prototype's terminal output and tell us:

1. Did sound come from the intended output?
2. Did pause/resume/volume/stop work? Did seek work for this file?
3. What `VmRSS`/`VmHWM` values were printed during actual output?

For a fair comparison with mpv, use the same prepared file, the same output and comparable duration/volume; do not compare Rust playing a local file against mpv extracting a YouTube page. If `/usr/bin/time` is installed, `time -v` can provide an additional peak-RSS report for each separate experiment. Do not run both simultaneously for a single-player comparison.

The 32 KiB reader, one on-demand source, bounded command queue, 64 MiB file limit and maximum wall time avoid whole-track buffering/unbounded work in our controller. Decoder/driver internals can still allocate their own memory; this is **not a 100 MB hard limit or proof of the aggregate service budget**. Use only trusted files for this first prototype. Page cache, yt-dlp/Node preparation, shared audio services, other listeners and future tunnel processes require separate aggregate accounting. Multi-device sessions, resource admission and iPhone browser streaming are not changed.

## CI and rollback

The separate CI job installs audio build headers plus ffmpeg **for fixture generation only**, runs prototype unit tests, builds the executable, decodes generated WAV/AAC-M4A/MP3, rejects bad/missing/oversized inputs, and exercises the playback branch against ALSA's null output. It does not access real YouTube media or a physical speaker. The production Rust/frontend/browser jobs continue separately.

To stop experimenting: type `stop` or press Ctrl+C, and resume music in the existing player. No service rollback/reinstallation is needed because production was not switched. Prepared test audio can be removed manually from the directory above; generated builds are ignored by Git. Do not promote this prototype until target output/reconnect tests, error behavior and measured memory satisfy the agreed scope.
