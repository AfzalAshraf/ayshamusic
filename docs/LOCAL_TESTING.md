# Test Aysha's Music Player on local Linux

Application code: `src/main.rs` (Rust API) and `src/index.html` (embedded browser UI).
Test code: `scripts/test_local.py` and `tests/test_local_smoke.py`.

These checks do not install, stop, restart or change any service. The optional stream check opens one downloader-backed request and closes it after a small sample. Until stream lifecycle cleanup is implemented, inspect for leftover yt-dlp processes after testing.

## 1. Build and offline tests

From the repository root on the Linux server:

```bash
git pull --ff-only origin arena/a850fb0d-ayshamusic &&
cargo test &&
cargo build --release &&
python3 -m unittest discover -s tests -p 'test_*.py' -v
```

Stop on failure; do not install an old binary after a failed build. No new Python dependencies are needed. Python is used as a test tool only; it does not replace the local Rust server.

## 2. Check the installed service and API

```bash
sudo systemctl status aysha-music --no-pager
sudo systemctl cat aysha-music
python3 scripts/test_local.py
```

The default URL is http://127.0.0.1:5050. For the same server on another approved address, pass `--base-url http://SERVER:5050`.

The smoke checker validates the actual root HTML, status fields and queue track schemas. No playback controls or queue mutations are issued. Zero exit status means the selected checks passed, not full production readiness.

## 3. Test a real audio stream

Use a YouTube track you are permitted to play:

```bash
python3 scripts/test_local.py --video-url 'https://www.youtube.com/watch?v=dQw4w9WgXcQ' --timeout 45
```

The checker reads at most a 4096-byte sample, does not print binary audio, checks the container against Content-Type, and exits nonzero on failure. It does not require downloading a full song.

**Known expected failure on current server:** WebM bytes served as audio/mpeg. The server also advertises byte ranges without implementing Range/206. The checker intentionally reports these rather than calling transport success end-to-end playback success. The byte-range check is conservative: it requires separate verification of seeking before trusting the advertised capability.

## 4. Capture evidence for remaining acceptance gates

```bash
sudo journalctl -u aysha-music -n 100 --no-pager
ps -eo pid,ppid,rss,comm | grep -E 'music-server|yt-dlp|mpv'
```

RSS is in KiB. A single snapshot is not a peak measurement. Measure during idle, browser streaming, Bluetooth playback, repeated skips and recovery; clarify whether the 20 MB budget includes child processes. Redact private details before sharing logs.

Inspect `ExecStart` in the installed unit before replacing binaries. The installed binary path is not assumed by these instructions.

## Manual acceptance matrix

Record PASS/FAIL, browser/device, and observed error for each:

- Search, select track, hear browser audio without Bluetooth.
- Pause/resume, stop, next/previous, volume and queue transitions.
- Empty queue fetches related tracks and continues without duplicate playback.
- Bluetooth speaker plays through mpv; disconnect behavior matches the approved fallback flow.
- Missing yt-dlp, unreachable YouTube and invalid track show useful errors.
- Repeated skips and closed tabs do not leave downloader processes or growing memory.
- Keyboard and mobile controls remain usable.
- Service recovers after restart/reboot with the approved persistence behavior.
- Access is restricted to the approved host/LAN/remote boundary.

Do not assign 5/5 until the relevant automated and manual gates are verified. Live stream, hardware and memory gates cannot be replaced with fixture tests.
