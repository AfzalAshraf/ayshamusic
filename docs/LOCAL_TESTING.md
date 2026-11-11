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

**Regression being fixed:** The previous server labelled WebM bytes as audio/mpeg and advertised unimplemented byte ranges. The new media module detects the container and advertises Accept-Ranges: none. Build and run the new Rust binary before retesting; rebuilding alone does not replace the currently running service binary. The checker intentionally reports mismatches rather than calling transport success end-to-end playback success. The byte-range check is conservative: it requires separate verification of seeking before trusting the advertised capability.

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


## Playback and Bluetooth regression tests

```bash
cargo test
node --test tests/frontend.test.cjs
bash -n scripts/diagnose_local.sh
bash scripts/diagnose_local.sh
```

Node is needed only to run frontend regression tests, not to run the player.

To test the fresh binary without guessing your systemd installation path, first build successfully, then stop the service and run the new executable in the foreground as your normal audio-session user:

```bash
cargo test && cargo build --release && sudo systemctl stop aysha-music && ./target/release/music-server
```

Leave that terminal open. In a second terminal, run the smoke checks. Hard-refresh the browser to load the new embedded page. When finished, Ctrl-C stops the foreground process; `sudo systemctl start aysha-music` restores the installed service, which may still use the older binary until you update its actual ExecStart path. Do not copy to an assumed installation path.

Bluetooth still targets the existing JBL MAC 04:CB:88:92:43:60. Confirm this is your speaker. Connect uses the existing connection script, optionally overridden by MUSIC_BT_CONNECT_SCRIPT. No real device scan or automatic pairing is claimed. Failures are now shown in the Bluetooth page; powering/pairing/adapter and service-user permissions require local diagnostics. Automatic background reconnect was removed so Disconnect is respected.

## Missing yt-dlp JavaScript runtime / occasional HTTP 403

First run `bash scripts/diagnose_local.sh` and capture the complete output. Each probe is bounded and systemctl paging is disabled. The script reports yt-dlp, Node and Deno paths/versions, the Linux distribution and Bluetooth/audio status. It does not install or enable any runtime.

According to the official guide (https://github.com/yt-dlp/yt-dlp/wiki/EJS), YouTube extraction needs both a supported JS runtime and EJS challenge scripts. Deno >=2.3 is recommended and enabled by default. Node >=22 needs `--js-runtimes node`; merely installing Node does not enable it in yt-dlp. EJS packaging depends on the yt-dlp installation method. Confirm those details before upgrading, installing, editing config or enabling remote component downloads.

A successful stream test is not proof of audible playback. A completed download can simply mean the browser buffered a whole track. Report separately whether sound is heard, which browser/device is used, and the visible player error. A 403 remains an upstream extraction/access failure until a retest establishes its cause; a missing JS runtime is a candidate, not a guaranteed explanation.
