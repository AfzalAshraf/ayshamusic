# Aysha's Music Player

A lightweight music player backend and Android-first frontend.

Made with ❤️ for Aysha — with love, from Afzal

**Updated target: at most 100 MB for agreed simultaneous-device use—not yet achieved or verified.** See the [memory-budget proposal](docs/MEMORY_BUDGET_PLAN.md) for the pending capacity and accounting decisions.

### Source, builds and current completion status

The production server is Rust: `src/main.rs` (API), `src/media.rs` (audio proxy) and `src/playback.rs` (queue/history). The embedded interface is `src/index.html`; web browsers still run HTML/CSS/JavaScript. The current Rust server invokes **yt-dlp/Node and mpv**; these have not been replaced with native Rust implementations.

The Stop/Play and same-tab refresh-recovery changes are implemented. An opt-in [native Rust local-audio prototype](docs/RUST_AUDIO_PROTOTYPE.md) is now available as a separate Cargo package; it does not run mpv or change the production service. A guaranteed 100 MB concurrent-device build, isolated device sessions, a production replacement for extraction/playback helpers and verified iPhone audio are **not complete**. Pulling the current branch does not install those planned features.

For compile-only commands, installation paths and host update verification, see [the local build/update guide](docs/LOCAL_SERVICE.md).

## Future upgrades: preserve the approved foundation

Start with [the main upgrade skill](skills/main-upgrade/SKILL.md), discoverable through [AGENTS.md](AGENTS.md). The [approved baseline](docs/UPGRADE_BASELINE.md) records the design to preserve, current limits and honest readiness gates. Future upgrades should build on this version—not replace the interface you approved.

## Local Linux: run without keeping SSH open

For the confirmed `afine` UID 1000 machine, stop any foreground player with Ctrl-C, then run:

```bash
cd /home/afine/ayshamusic
bash scripts/install_local.sh
```

The installer tests/builds the Rust binary, backs up the existing installation, installs the systemd service, enables boot startup and user lingering, and supplies the tested Node runtime. Run it as `afine`, not as root. Closing the SSH client is then safe; the Linux server itself must remain powered on. Browser audio still requires its listening browser to remain open.

See [local service installation and verification](docs/LOCAL_SERVICE.md), [local tests](docs/LOCAL_TESTING.md), and the dedicated [SVG icon skill](skills/icons/SKILL.md). Hardware Bluetooth connectivity and reboot recovery must be checked on the real machine; automated builds do not prove those gates.

## What it is

- **Rust server** (`src/main.rs`) — runs on your own Linux machine/Raspberry Pi with `mpv` + `bluetoothctl` + `pactl`. Plays audio to your Bluetooth speaker via PulseAudio. ~6MB RAM.
- **Frontend** — the local Rust server embeds `src/index.html` and `src/icons.svg`; `src/www/index.html` is the separate web/PWA asset copy. Changes to the latter alone do not update the Rust-served player.
- **Free-tier deployment** — `Dockerfile`, `fly.toml`, `render.yaml` for one-click deploy to Fly.io, Render, Koyeb. Uses Python `server.py` fallback (no Rust compile needed on the platform).

## Features

| Feature | Rust (local) | Free-tier (Docker) |
|---------|:---:|:---:|
| YouTube search via yt-dlp | ✅ | ✅ |
| Play via mpv + PulseAudio | ✅ | ❌ (no audio stack) |
| Bluetooth speaker (JBL, etc.) | ✅ | ❌ |
| Browser audio via /api/stream | ✅ | ✅ |
| Queue + autoplay (Spotify Radio) | ✅ | ✅ |
| Like / Unlike songs | ✅ | ✅ |
| Tailscale mesh networking | ✅ | ✅ (userspace mode) |
| PWA + MediaSession lock screen | ✅ | ✅ |

## One-click deploy to free platforms

### Fly.io (free tier)

```bash
# Install flyctl: https://fly.io/docs/hands-on/install/
fly login
fly launch --dockerfile Dockerfile --name aysha-music --region ewr
fly deploy
# Open: https://aysha-music.fly.dev
```

### Render (free tier)

```bash
# Connect your GitHub repo to Render, it auto-detects render.yaml
# Or: https://render.com/deploy?repo=https://github.com/AfzalAshraf/ayshamusic
```

### Koyeb (free tier)

```bash
# In Koyeb dashboard: Create App → GitHub → AfzalAshraf/ayshamusic
# Build command: (Dockerfile auto-detected)
# Run command: python3 server.py
# Port: 5050
# Open: https://aysha-music-XXXX.koyeb.app
```

**Note:** Koyeb free tier restricts `/dev/net/tun` and `NET_ADMIN`, so Tailscale runs in **userspace networking mode** (no kernel tunnel needed). See Tailscale section below.

---

## 🔒 Tailscale — access from anywhere (no port forwarding)

Tailscale gives your server a stable `100.x.y.z` IP on your private mesh network. Open `http://100.x.y.z:5050` from any phone/laptop on your tailnet — no port forwarding, no public IP, no router config.

### Option A: VPS with kernel TUN (recommended — Oracle Cloud Free Tier ARM)

Best free option: Oracle Cloud Free Tier (4 cores, 24GB RAM, always free).

```bash
# On your VPS
curl -fsSL https://tailscale.com/install.sh | sh
sudo tailscale up          # note the 100.x.y.z IP
sudo apt-get install -y yt-dlp mpv bluez pulseaudio rustc cargo
cargo build --release
./target/release/music-server

# On your phone: install Tailscale, join same tailnet
# Open http://100.x.y.z:5050
```

### Option B: Userspace Tailscale (Koyeb/Fly/Render free tier — no /dev/net/tun)

Free platforms block `/dev/net/tun`. Use userspace networking mode instead — it creates a SOCKS5 proxy, no kernel tunnel needed:

```bash
# Install tailscale, then start in userspace mode
tailscale up --tun=userspace-networking --accept-routes
# Check status: tailscale status
# Your 100.x.y.z IP is shown; the server is reachable at http://100.x.y.z:5050
```

The `/api/tailscale/status` endpoint reports:
```json
{
  "online": true,
  "ip": "100.x.y.z",
  "url": "http://100.x.y.z:5050"
}
```

### Option C: Tailscale Funnel (public URL, no Tailscale install on client)

```bash
tailscale up
tailscale funnel 5050
# Gives you a public https://*.ts.net URL — anyone can open it
```

---

## Local build (Rust, for your own machine)

```bash
# Requires: rustc, cargo, yt-dlp, mpv, bluetoothctl, pactl
cargo build --release
./target/release/music-server
# Open http://localhost:5050
```

RAM usage: ~4.7 MB idle, well under the 20 MB target.

The systemd service file (`aysha-music.service`) is configured to run on boot as your user.

---

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | SPA UI (PWA) |
| GET | `/manifest.json` | PWA manifest |
| GET | `/sw.js` | Service worker |
| GET | `/api/status` | Player state + queue length + liked count |
| GET | `/api/search?q=…` | YouTube search |
| GET | `/api/stream?url=…` | Direct stream URL (browser audio) |
| POST | `/api/play` | Play track (JSON: url, title, artist, thumbnail, id) |
| POST | `/api/pause` | Pause |
| POST | `/api/resume` | Resume |
| POST | `/api/stop` | Stop |
| POST | `/api/seek?pos=N` | Seek to position (seconds) |
| POST | `/api/volume?val=N` | Set volume (0-100) |
| POST | `/api/volume/up` | Volume up |
| POST | `/api/volume/down` | Volume down |
| POST | `/api/next` | Next track |
| POST | `/api/prev` | Previous track |
| GET | `/api/queue` | Current queue |
| POST | `/api/queue_add` | Add track to queue (JSON body) |
| POST | `/api/queue_clear` | Clear queue |
| GET | `/api/bt/status` | Bluetooth status (JBL Flip Essential) |
| POST | `/api/bt/connect` | Connect Bluetooth |
| GET | `/api/tailscale/status` | Tailscale IP + online status |
| GET | `/api/liked` | Liked songs |
| POST | `/api/like` | Like a track |
| POST | `/api/unlike` | Unlike a track |

---

## Free-tier limitations

| Platform | Tailscale | Bluetooth/mpv | Notes |
|----------|:---:|:---:|-------|
| Oracle Cloud Free Tier | ✅ kernel mode | ✅ | Best option |
| Fly.io free tier | ⚠️ userspace | ❌ | No audio stack |
| Render free tier | ⚠️ userspace | ❌ | No audio stack |
| Koyeb free tier | ⚠️ userspace | ❌ | No /dev/net/tun |

On free platforms without mpv/bluez, Bluetooth/mpv endpoints return graceful errors, but search, queue, liked songs, and browser audio via `/api/stream` all work.

---

## Repo structure

```
Cargo.toml          — Rust package manifest (pure std + axum/serde/tokio/reqwest)
src/main.rs         — Rust HTTP server (all API endpoints, mpv IPC, autoplay watcher)
src/index.html      — HTML embedded in Rust binary
src/www/index.html  — PWA frontend (source, copied to src/index.html at build)
src/www/manifest.json — PWA manifest
src/www/sw.js        — Service worker
Dockerfile          — Docker image (yt-dlp, mpv, bluez, pulseaudio, tailscale)
fly.toml            — Fly.io deployment config
render.yaml         — Render deployment config
README.md           — This file
```

---

Made with ♥ for Aysha — with love, from Afzal.