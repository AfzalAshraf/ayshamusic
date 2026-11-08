# Aysha's Music Player — Pure Rust

A lightweight music player backend + PWA frontend, made with ♥ for Aysha — with love, from Afzal.
**Target: under 20MB RAM.** Pure Rust standard library — no external crates.

## Features

- YouTube search via `yt-dlp` subprocess
- Playback via `mpv` subprocess with IPC socket
- Bluetooth control via `bluetoothctl` / `pactl` (PulseAudio)
- Liked songs with heart button (♥) on every card, saved to Library tab
- PWA: installable on iPhone Home Screen, full-screen standalone mode
- MediaSession API: lock screen / Control Center controls with artwork + title
- Modern dark UI, slim scrollbars, mobile-responsive

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | SPA UI (PWA) |
| GET | `/manifest.json` | PWA manifest |
| GET | `/sw.js` | Service worker |
| GET | `/api/status` | Player state |
| GET | `/api/search?q=…` | YouTube search |
| POST | `/api/play` | Play track (JSON body) |
| POST | `/api/pause` | Pause |
| POST | `/api/stop` | Stop |
| POST | `/api/volume/up` | Volume up |
| POST | `/api/volume/down` | Volume down |
| POST | `/api/next` | Next track |
| POST | `/api/prev` | Previous track |
| GET | `/api/bt/status` | Bluetooth status |
| POST | `/api/bt/connect` | Bluetooth connect |
| GET | `/api/liked` | Liked songs |
| POST | `/api/like` | Like a track |
| POST | `/api/unlike` | Unlike a track |

## Build & Run

```bash
cargo build --release
./target/release/music-server
# open http://localhost:5050
```

Requires `yt-dlp`, `mpv`, `bluetoothctl`, and `pactl` on the host for full functionality.
Endpoints degrade gracefully when external tools are unavailable.

## Deploy

Compile on any machine with `rustc` + `cargo` + crates.io access, then copy the
`target/release/music-server` binary to the target device.
