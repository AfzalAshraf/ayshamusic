# Aysha's Music Player — Pure Rust

A lightweight music player backend + PWA frontend, made with ♥ for Aysha — with love, from Afzal.
**Target: under 20MB RAM.** Pure Rust standard library — no external crates.

## Features

- YouTube search via `yt-dlp` subprocess
- Playback via `mpv` subprocess with IPC socket (PulseAudio output)
- Bluetooth control via `bluetoothctl` / `pactl` (PulseAudio)
- Tailscale status endpoint (`/api/tailscale/status`) — no port forwarding needed
- Liked songs with heart button (♥) on every card, saved to Library tab
- Spotify-style auto-play queue with related tracks (auto-fetches when empty)
- Seek, volume, next/prev controls
- PWA: installable on iPhone Home Screen, full-screen standalone mode
- MediaSession API: lock screen / Control Center controls with artwork + title
- Modern dark UI, slim scrollbars, mobile-responsive

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | SPA UI (PWA) |
| GET | `/manifest.json` | PWA manifest |
| GET | `/sw.js` | Service worker |
| GET | `/api/status` | Player state + queue length |
| GET | `/api/search?q=…` | YouTube search |
| POST | `/api/play` | Play track (JSON body) |
| POST | `/api/pause` | Pause |
| POST | `/api/stop` | Stop |
| POST | `/api/seek?pos=N` | Seek to position (seconds) |
| POST | `/api/volume?val=N` | Set volume (0-100) |
| POST | `/api/volume/up` | Volume up |
| POST | `/api/volume/down` | Volume down |
| POST | `/api/next` | Next track |
| POST | `/api/prev` | Previous track |
| GET | `/api/queue` | Current queue |
| POST | `/api/queue_add` | Add track to queue |
| POST | `/api/queue_clear` | Clear queue |
| GET | `/api/bt/status` | Bluetooth status |
| POST | `/api/bt/connect` | Bluetooth connect |
| GET | `/api/tailscale/status` | Tailscale IP + online status |
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

---

## 🔒 Access from anywhere — Tailscale (no port forwarding)

Tailscale creates a private mesh network. Your server gets a stable `100.x.y.z` IP that works from any device on your tailnet, with **no port forwarding, no public IP, no router config**.

### Option A: Run on any VPS + Tailscale (recommended)

Best free option: **Oracle Cloud Free Tier** ARM instance (4 cores, 24GB RAM, always free).

```bash
# On your VPS
curl -fsSL https://tailscale.com/install.sh | sh
sudo tailscale up
# Note the 100.x.y.z IP shown

# Install dependencies + build
sudo apt-get install -y yt-dlp mpv bluez pulseaudio rustc cargo
cargo build --release
./target/release/music-server

# On your phone/laptop: install Tailscale, join same tailnet
# Open http://100.x.y.z:5050
```

### Option B: Tailscale Funnel (expose to public internet)

If you want to share a public URL with anyone (no Tailscale install needed on their end):

```bash
# On your server
tailscale up
tailscale funnel 5050
# Gives you a public https://*.ts.net URL
```

### Option C: Fly.io (free tier, small VM)

```bash
# Install flyctl, then:
fly launch --dockerfile Dockerfile
fly deploy
# App runs at https://aysha-music.fly.dev
```

### Option D: Render (free tier)

```bash
# Connect your GitHub repo to Render, it uses render.yaml
# Deploys as a web service
```

**Note:** Koyeb's free tier does **not** support Tailscale well (needs kernel `NET_ADMIN` / `/dev/net/tun` which Koyeb restricts). Fly.io or a small VPS with Tailscale works better.

---

## Deploy with Docker

```bash
docker build -t aysha-music .
docker run -p 5050:5050 --device /dev/net/tun --cap-add NET_ADMIN aysha-music
```

---

## Compile requirements

Compile on any machine with `rustc` + `cargo` + crates.io access, then copy the
`target/release/music-server` binary to the target device. Host needs
`yt-dlp`, `mpv`, `bluetoothctl`, and `pactl` for full functionality.
