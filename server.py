#!/usr/bin/env python3
"""Aysha's Music Player — Python stdlib HTTP server.

Implements the full API designed for the Rust version:
  GET  /                 — SPA UI (PWA, MediaSession, liked songs, queue)
  GET  /manifest.json    — PWA manifest
  GET  /sw.js            — Service worker
  GET  /api/status       — Player state
  GET  /api/search?q=    — YouTube search via yt-dlp
  GET  /api/stream?url=  — Direct audio stream URL via yt-dlp
  POST /api/play         — Play track (JSON body)
  POST /api/pause        — Pause
  POST /api/stop         — Stop
  POST /api/seek?pos=N   — Seek (client-side)
  POST /api/volume?val=N — Set volume
  POST /api/volume/up    — Volume up
  POST /api/volume/down  — Volume down
  POST /api/next         — Next track
  POST /api/prev         — Previous track
  GET  /api/queue        — Current queue
  POST /api/queue_add    — Add to queue
  POST /api/queue_clear  — Clear queue
  GET  /api/bt/status    — Bluetooth status
  POST /api/bt/connect   — Bluetooth connect
  GET  /api/tailscale/status — Tailscale status
  GET  /api/liked        — Liked songs
  POST /api/like         — Like
  POST /api/unlike       — Unlike

Designed to run under ~50MB RAM. Only Python stdlib in-process;
yt-dlp invoked as subprocess for search/stream resolution.
"""
import http.server
import json
import os
import socket
import subprocess
import threading
from urllib.parse import urlparse, parse_qs

PORT = int(os.environ.get("PORT", "5050"))
HIST_FILE = "/tmp/aysha-history.json"
LIKED_FILE = "/tmp/aysha-liked.json"
MAX_HISTORY = 50
MAX_QUEUE = 200

# Check which external tools exist
TOOLS = {}
for name in ("yt-dlp", "mpv", "bluetoothctl", "tailscale", "pactl"):
    try:
        r = subprocess.run(["which", name], capture_output=True, text=True, timeout=3)
        TOOLS[name] = r.stdout.strip() or None
    except Exception:
        TOOLS[name] = None


# ---------------------------------------------------------------------------
# shared player state
# ---------------------------------------------------------------------------
state = {
    "playing": False,
    "paused": False,
    "current_track": None,
    "title": "",
    "artist": "",
    "id": "",
    "url": "",
    "thumbnail": "",
    "duration": 0,
    "volume": 80,
    "queue": [],
    "history": [],
    "liked": [],
    "tools": TOOLS,
}
state_lock = threading.Lock()


def run_cmd(cmd, timeout=20):
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
        return r.returncode, r.stdout, r.stderr
    except Exception as e:
        return -1, "", str(e)


# ---------------------------------------------------------------------------
# persistence
# ---------------------------------------------------------------------------
def load_json(path, default):
    try:
        with open(path, "r") as f:
            return json.load(f)
    except Exception:
        return default


def save_json(path, data):
    try:
        with open(path, "w") as f:
            json.dump(data, f)
    except Exception:
        pass


def load_tracks(path):
    data = load_json(path, [])
    out = []
    for t in data:
        out.append({
            "id": t.get("id", ""),
            "title": t.get("title", ""),
            "artist": t.get("artist", ""),
            "thumbnail": t.get("thumbnail", ""),
            "url": t.get("url", ""),
            "duration": t.get("duration", 0),
        })
    return out


def save_tracks(path, tracks):
    save_json(path, tracks)


def add_history(track):
    h = load_tracks(HIST_FILE)
    if not any(t.get("id") == track.get("id") for t in h):
        h.insert(0, track)
    save_tracks(HIST_FILE, h[:MAX_HISTORY])
    with state_lock:
        state["history"] = h[:MAX_HISTORY]


def autoplay_enqueue(track):
    """Fetch related songs and append them to the queue (Spotify Radio style)."""
    try:
        related = fetch_related(track.get("id", ""), track.get("title", ""), track.get("artist", ""))
        if not related:
            return
        with state_lock:
            existing_ids = {qt.get("id", "") for qt in state["queue"]}
            ct_id = state["current_track"].get("id", "") if state["current_track"] else ""
            to_add = [rt for rt in related if rt.get("id", "") not in existing_ids and rt.get("id", "") != ct_id]
            state["queue"].extend(to_add)
    except Exception:
        pass


# ---------------------------------------------------------------------------
# youtube search via yt-dlp subprocess
# ---------------------------------------------------------------------------
def extract_field(obj, key, default=""):
    try:
        v = obj.get(key)
        return v if v is not None else default
    except Exception:
        return default


def search_youtube(q):
    if not TOOLS.get("yt-dlp"):
        return [], "yt-dlp not installed"
    rc, out, err = run_cmd(
        ["yt-dlp", "--flat-playlist", "--dump-json", f"ytsearch16:{q}"],
        timeout=30,
    )
    tracks = []
    if rc == 0:
        for line in out.splitlines():
            line = line.strip()
            if not line:
                continue
            try:
                v = json.loads(line)
            except Exception:
                continue
            vid = v.get("id") or ""
            if not vid:
                continue
            tracks.append({
                "id": vid,
                "title": v.get("title") or "Unknown Title",
                "artist": v.get("uploader") or "YouTube",
                "thumbnail": f"https://img.youtube.com/vi/{vid}/hqdefault.jpg",
                "url": f"https://www.youtube.com/watch?v={vid}",
                "duration": v.get("duration") or 0,
            })
    return tracks, ""


def get_stream_url(url):
    if not TOOLS.get("yt-dlp"):
        return ""
    rc, out, err = run_cmd(
        ["yt-dlp", "-f", "bestaudio", "--get-url", url],
        timeout=30,
    )
    if rc == 0:
        return out.strip().split("\n")[0] if out.strip() else ""
    return ""


def fetch_related(id_, title, artist):
    if not TOOLS.get("yt-dlp"):
        return []
    q = f"{title} {artist} similar"
    rc, out, err = run_cmd(
        ["yt-dlp", "--flat-playlist", "--dump-json", f"ytsearch10:{q}"],
        timeout=30,
    )
    tracks = []
    if rc == 0:
        for line in out.splitlines():
            line = line.strip()
            if not line:
                continue
            try:
                v = json.loads(line)
            except Exception:
                continue
            tid = v.get("id") or ""
            if not tid or tid == id_:
                continue
            tracks.append({
                "id": tid,
                "title": v.get("title") or "Unknown Title",
                "artist": v.get("uploader") or "YouTube",
                "thumbnail": f"https://img.youtube.com/vi/{tid}/hqdefault.jpg",
                "url": f"https://www.youtube.com/watch?v={tid}",
                "duration": v.get("duration") or 0,
            })
    return tracks


# ---------------------------------------------------------------------------
# bluetooth
# ---------------------------------------------------------------------------
def bt_devices_list():
    if not TOOLS.get("bluetoothctl"):
        return []
    rc, out, err = run_cmd(["bluetoothctl", "devices"], timeout=10)
    devs = []
    if rc == 0:
        for line in out.splitlines():
            parts = line.split()
            if len(parts) >= 3 and parts[0] == "Device":
                devs.append({"mac": parts[1], "name": " ".join(parts[2:])})
    return devs


def bt_status():
    if not TOOLS.get("bluetoothctl"):
        return {"connected": False, "device": "", "error": "bluetoothctl not installed"}
    rc, out, err = run_cmd(["bluetoothctl", "info"], timeout=10)
    connected = False
    name = ""
    if rc == 0:
        if "Connected: yes" in out:
            connected = True
            for line in out.splitlines():
                if line.strip().startswith("Name:"):
                    name = line.split(":", 1)[1].strip()
                    break
    return {"connected": connected, "device": name}


def bt_connect(mac):
    if not TOOLS.get("bluetoothctl"):
        return {"ok": False, "error": "bluetoothctl not installed"}
    run_cmd(["bluetoothctl", "trust", mac], timeout=10)
    run_cmd(["bluetoothctl", "connect", mac], timeout=15)
    import time
    time.sleep(2)
    return {"ok": bt_status()["connected"]}


# ---------------------------------------------------------------------------
# tailscale
# ---------------------------------------------------------------------------
def tailscale_status():
    if not TOOLS.get("tailscale"):
        return {"online": False, "ip": "", "url": "", "error": "tailscale not installed"}
    rc, out, err = run_cmd(["tailscale", "ip", "-4"], timeout=10)
    ip = out.strip() if rc == 0 else ""
    rc2, out2, err2 = run_cmd(["tailscale", "status"], timeout=10)
    online = (rc2 == 0 and "100." in out2)
    url = f"http://{ip}:{PORT}" if ip else ""
    return {"online": online, "ip": ip, "url": url}


# ---------------------------------------------------------------------------
# HTML UI (served inline)
# ---------------------------------------------------------------------------
INDEX_HTML = r"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1,maximum-scale=1,user-scalable=no,viewport-fit=cover">
<meta name="apple-mobile-web-app-capable" content="yes">
<meta name="apple-mobile-web-app-status-bar-style" content="black-translucent">
<meta name="apple-mobile-web-app-title" content="Aysha's Music">
<meta name="theme-color" content="#030303">
<link rel="manifest" href="/manifest.json">
<link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='%23ff0000'><path d='M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z'/></svg>">
<title>Aysha's Music Player</title>
<style>
:root{--bg:#030303;--sidebar:#000000;--panel:#121212;--accent:#ff0000;--text:#fff;--sub:#aaaaaa;--dim:#717171;--border:rgba(255,255,255,0.08);--liked:#ff0000}
*{margin:0;padding:0;box-sizing:border-box;font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif;-webkit-tap-highlight-color:transparent}
body{background:var(--bg);color:var(--text);display:flex;height:100vh;overflow:hidden}
.sidebar{width:230px;background:var(--sidebar);border-right:1px solid var(--border);display:flex;flex-direction:column;padding:20px 12px;flex-shrink:0;z-index:10}
.brand{display:flex;align-items:center;gap:10px;margin-bottom:30px;padding-left:10px;cursor:pointer}
.brand svg{width:28px;height:28px;fill:var(--accent)}
.brand h1{font-size:16px;font-weight:700;letter-spacing:-0.5px}
.nav-group{display:flex;flex-direction:column;gap:4px}
.nav-item{display:flex;align-items:center;gap:16px;padding:12px 16px;border-radius:12px;color:var(--sub);font-size:14px;font-weight:600;cursor:pointer;transition:all .2s}
.nav-item svg{width:22px;height:22px;fill:currentColor}
.nav-item:hover{color:#fff;background:rgba(255,255,255,0.05)}
.nav-item.active{color:#fff;background:rgba(255,255,255,0.12)}
.main-wrapper{flex:1;display:flex;flex-direction:column;height:100vh;overflow:hidden;position:relative}
.header{height:64px;background:rgba(3,3,3,0.9);backdrop-filter:blur(12px);display:flex;align-items:center;justify-content:space-between;padding:0 24px;border-bottom:1px solid var(--border);flex-shrink:0}
.search-wrap{flex:1;max-width:480px;position:relative}
.search-bar{width:100%;background:#1a1a1a;border:1px solid var(--border);border-radius:20px;padding:8px 16px 8px 40px;color:#fff;font-size:14px;outline:none}
.search-bar:focus{border-color:var(--accent)}
.search-icon{position:absolute;left:14px;top:50%;transform:translateY(-50%);width:16px;height:16px;fill:var(--sub)}
.content-area{flex:1;overflow-y:auto;padding:24px 28px 120px}
.tab-page{display:none}
.tab-page.active{display:block}
.now-playing-hero{background:linear-gradient(135deg,rgba(255,0,0,0.15)0%,rgba(20,20,20,0.8)100%);border:1px solid var(--border);border-radius:16px;padding:20px;display:flex;gap:24px;align-items:center;margin-bottom:32px}
.hero-art-box{width:220px;aspect-ratio:16/9;border-radius:10px;overflow:hidden;background:#202020;flex-shrink:0;box-shadow:0 8px 20px rgba(0,0,0,0.6)}
.hero-art{width:100%;height:100%;object-fit:cover}
.hero-details{flex:1;overflow:hidden}
.hero-tag{font-size:11px;font-weight:700;text-transform:uppercase;letter-spacing:1px;color:var(--accent);margin-bottom:6px}
.hero-title{font-size:22px;font-weight:700;color:#fff;margin-bottom:4px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.hero-artist{font-size:14px;color:var(--sub);margin-bottom:16px}
.hero-controls{display:flex;gap:12px;align-items:center}
.btn-hero{display:inline-flex;align-items:center;gap:8px;background:#fff;color:#000;border:none;padding:10px 20px;border-radius:20px;font-size:13px;font-weight:700;cursor:pointer}
.btn-hero:hover{background:#f0f0f0}
.btn-hero.btn-stop{background:rgba(255,255,255,0.15);color:#fff}
.btn-hero svg{width:18px;height:18px;fill:currentColor}
.section-title{font-size:18px;font-weight:700;margin:24px 0 16px}
.grid-cards{display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:16px}
.song-card{display:flex;align-items:center;gap:12px;padding:10px;border-radius:10px;background:rgba(255,255,255,0.03);border:1px solid transparent;transition:all .2s;cursor:pointer;position:relative}
.song-card:hover{background:rgba(255,255,255,0.08);border-color:var(--border)}
.card-thumb{width:80px;aspect-ratio:16/9;object-fit:cover;border-radius:6px;background:#202020;flex-shrink:0}
.card-info{overflow:hidden;flex:1}
.card-title{font-size:13px;font-weight:600;color:#fff;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.card-artist{font-size:11px;color:var(--sub);margin-top:3px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.card-action{background:rgba(255,255,255,0.1);border:none;color:#fff;width:34px;height:34px;border-radius:50%;display:flex;align-items:center;justify-content:center;cursor:pointer;flex-shrink:0}
.card-action svg{width:18px;height:18px;fill:currentColor}
.card-action.active{background:var(--accent)}
.card-heart{position:absolute;top:10px;right:10px;width:32px;height:32px;border-radius:50%;background:rgba(255,255,255,0.1);border:none;color:var(--sub);display:flex;align-items:center;justify-content:center;cursor:pointer;z-index:2;transition:all .2s}
.card-heart:hover{background:rgba(255,255,255,0.2);color:#fff}
.card-heart.liked{color:var(--liked);background:rgba(255,0,0,0.2)}
.chips-row{display:flex;gap:10px;overflow-x:auto;padding-bottom:12px;margin-bottom:20px}
.chip{background:rgba(255,255,255,0.08);color:#fff;padding:8px 18px;border-radius:18px;font-size:13px;font-weight:600;cursor:pointer;white-space:nowrap;border:1px solid transparent}
.chip:hover,.chip.active{background:#fff;color:#000}
.player-bar{position:absolute;bottom:0;left:0;right:0;height:72px;background:rgba(18,18,18,0.96);backdrop-filter:blur(16px);border-top:1px solid var(--border);display:flex;flex-direction:column;z-index:100}
.progress-container{width:100%;height:3px;background:rgba(255,255,255,0.15);cursor:pointer;position:relative}
.progress-bar{height:100%;width:0%;background:var(--accent)}
.player-content{display:flex;align-items:center;justify-content:space-between;height:69px;padding:0 24px}
.player-left{display:flex;align-items:center;gap:12px;width:30%}
.np-art{width:64px;aspect-ratio:16/9;object-fit:cover;border-radius:4px;background:#282828;flex-shrink:0}
.np-title{font-size:13px;font-weight:600;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.np-artist{font-size:11px;color:var(--sub);margin-top:2px}
.player-center{display:flex;align-items:center;gap:16px}
.ctrl-btn{background:none;border:none;color:#fff;cursor:pointer;padding:6px;border-radius:50%;display:flex;align-items:center;justify-content:center}
.ctrl-btn svg{width:22px;height:22px;fill:currentColor}
.play-main{background:#fff;color:#000;width:40px;height:40px}
.play-main svg{fill:#000;width:22px;height:22px}
.player-right{display:flex;align-items:center;gap:12px;width:25%;justify-content:flex-end}
.vol-box{display:flex;align-items:center;gap:8px}
.vol-box input{width:80px;accent-color:var(--accent);cursor:pointer}
.time-str{font-size:12px;color:var(--sub);font-family:monospace}
.footer{text-align:center;padding:24px;color:var(--sub);font-size:12px;border-top:1px solid var(--border);margin-top:40px}
.bt-card{background:rgba(255,255,255,0.04);border:1px solid var(--border);border-radius:12px;padding:16px;margin-bottom:12px;display:flex;justify-content:space-between;align-items:center}
.bt-card b{color:#fff}
.bt-card small{color:var(--sub);display:block}
.empty-msg{color:var(--sub);text-align:center;padding:40px;font-size:14px}
.liked-count{color:var(--accent);font-weight:700;margin-left:4px}
@media(max-width:768px){
 body{flex-direction:column}
 .sidebar{width:100%;height:60px;flex-direction:row;justify-content:space-around;padding:0;position:fixed;bottom:0;border-right:none;border-top:1px solid var(--border);z-index:200}
 .brand{display:none}
 .nav-item span{display:none}
 .nav-item{padding:10px}
 .main-wrapper{height:calc(100vh - 60px)}
 .player-bar{bottom:60px}
 .content-area{padding-bottom:140px}
}
*::-webkit-scrollbar{width:6px;height:6px}
*::-webkit-scrollbar-track{background:transparent}
*::-webkit-scrollbar-thumb{background:rgba(255,255,255,0.12);border-radius:10px}
*::-webkit-scrollbar-thumb:hover{background:rgba(255,255,255,0.25)}
.content-area{scrollbar-width:thin;scrollbar-color:rgba(255,255,255,0.12) transparent}
</style>
</head>
<body>
<div class="sidebar">
  <div class="brand" onclick="switchTab('home')">
    <svg viewBox="0 0 24 24"><path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z"/></svg>
    <h1>Aysha's Music</h1>
  </div>
  <div class="nav-group">
    <div class="nav-item active" id="nav-home" onclick="switchTab('home')"><svg viewBox="0 0 24 24"><path d="M10 20v-6h4v6h5v-8h3L12 3 2 12h3v8z"/></svg><span>Home</span></div>
    <div class="nav-item" id="nav-explore" onclick="switchTab('explore')"><svg viewBox="0 0 24 24"><path d="M12 10.9c-.61 0-1.1.49-1.1 1.1s.49 1.1 1.1 1.1c.61 0 1.1-.49 1.1-1.1s-.49-1.1-1.1-1.1zM12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm2.19 12.19L6 18l3.81-8.19L18 6l-3.81 8.19z"/></svg><span>Explore</span></div>
    <div class="nav-item" id="nav-library" onclick="switchTab('library')"><svg viewBox="0 0 24 24"><path d="M4 6H2v14c0 1.1.9 2 2 2h14v-2H4V6zm16-4H8c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2zm0 14H8V4h12v12z"/></svg><span>Library</span></div>
    <div class="nav-item" id="nav-bluetooth" onclick="switchTab('bluetooth')"><svg viewBox="0 0 24 24"><path d="M17.71 7.71L12 2h-1v7.59L6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 11 14.41V22h1l5.71-5.71-4.3-4.29 4.3-4.29zM13 5.83l1.88 1.88L13 9.59V5.83zm1.88 10.46L13 18.17v-3.76l1.88 1.88z"/></svg><span>Bluetooth</span></div>
  </div>
</div>
<div class="main-wrapper">
  <div class="header">
    <div class="search-wrap">
      <svg class="search-icon" viewBox="0 0 24 24"><path d="M15.5 14h-.79l-.28-.27C15.41 12.59 16 11.11 16 9.5 16 5.91 13.09 3 9.5 3S3 5.91 3 9.5 5.91 16 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z"/></svg>
      <input type="text" class="search-bar" placeholder="Search songs, artists..." onkeypress="if(event.key==='Enter')searchMusic(this.value)">
    </div>
  </div>
  <div class="content-area">
    <div class="tab-page active" id="tab-home">
      <div class="now-playing-hero" id="hero-box">
        <div class="hero-art-box"><img class="hero-art" id="hero-art" src="https://img.youtube.com/vi/dikC8UmxxnE/hqdefault.jpg"></div>
        <div class="hero-details">
          <div class="hero-tag">Now Playing</div>
          <div class="hero-title" id="hero-title">Select a song to play</div>
          <div class="hero-artist" id="hero-artist">Aysha's Music Player</div>
          <div class="hero-controls">
            <button class="btn-hero" onclick="togglePlay()"><svg id="hero-play-svg" viewBox="0 0 24 24"><path d="M8 5v14l11-7z"/></svg><span id="hero-play-label">Play</span></button>
            <button class="btn-hero btn-stop" onclick="stopPlay()"><svg viewBox="0 0 24 24"><path d="M6 6h12v12H6z"/></svg>Stop</button>
          </div>
        </div>
      </div>
      <div class="section-title">Quick Picks for Aysha</div>
      <div class="grid-cards" id="quick-picks"></div>
      <div class="section-title">Listen Again</div>
      <div class="grid-cards" id="recent-history"></div>
    </div>
    <div class="tab-page" id="tab-explore">
      <div class="chips-row">
        <div class="chip active" onclick="fetchCategory('Jazz')">Jazz</div>
        <div class="chip" onclick="fetchCategory('Malayalam')">Malayalam</div>
        <div class="chip" onclick="fetchCategory('Tamil')">Tamil</div>
        <div class="chip" onclick="fetchCategory('Hindi')">Hindi</div>
        <div class="chip" onclick="fetchCategory('Enrique')">Enrique</div>
        <div class="chip" onclick="fetchCategory('English love songs')">English Love Songs</div>
        <div class="chip" onclick="fetchCategory('Lofi')">Lofi</div>
        <div class="chip" onclick="fetchCategory('Arabic')">Arabic</div>
      </div>
      <div class="section-title" id="explore-title">Featured Genre</div>
      <div class="grid-cards" id="explore-cards"></div>
    </div>
    <div class="tab-page" id="tab-library">
      <div class="section-title">Liked Songs <span class="liked-count" id="liked-count">0</span></div>
      <div class="grid-cards" id="full-history"></div>
      <div class="section-title">Queue <span id="queue-count">0</span> <button class="chip" onclick="queueClear()" style="float:right">Clear</button> <span style="color:var(--sub);font-size:11px">auto-plays next</span></div>
      <div class="grid-cards" id="queue-list"></div>
    </div>
    <div class="tab-page" id="tab-bluetooth">
      <div class="section-title">Bluetooth Settings</div>
      <div class="bt-card" id="bt-status-card">
        <div><b id="bt-status-text">Status: Checking...</b><small id="bt-device-text">--</small></div>
        <button class="chip" onclick="scanBluetooth()">Scan Speakers</button>
      </div>
      <div class="section-title">Available Devices</div>
      <div id="bt-list"></div>
    </div>
    <div class="footer">Made with &hearts; for Aysha &mdash; with love, from Afzal</div>
  </div>
  <div class="player-bar" id="player-bar">
    <div class="progress-container" onclick="seekTrack(event)"><div class="progress-bar" id="progress-bar"></div></div>
    <div class="player-content">
      <div class="player-left">
        <img class="np-art" id="np-art" src="https://img.youtube.com/vi/dikC8UmxxnE/hqdefault.jpg">
        <div><div class="np-title" id="np-title">No track playing</div><div class="np-artist" id="np-artist">Aysha's Music</div></div>
      </div>
      <div class="player-center">
        <button class="ctrl-btn play-main" onclick="togglePlay()"><svg id="play-svg" viewBox="0 0 24 24"><path d="M8 5v14l11-7z"/></svg></button>
        <button class="ctrl-btn" onclick="stopPlay()" title="Stop"><svg viewBox="0 0 24 24"><path d="M6 6h12v12H6z"/></svg></button>
        <span class="time-str" id="time-display">0:00 / 0:00</span>
      </div>
      <div class="player-right">
        <div class="vol-box"><input type="range" min="0" max="100" step="5" value="80" oninput="changeVol(this.value)"></div>
      </div>
    </div>
  </div>
  <audio id="audio-player" crossorigin="anonymous"></audio>
</div>
<script>
let state = { current_track: null, is_playing: false, volume: 80, pos: 0, duration: 0, liked: [], queue: [] };
let durationSec = 0;

async function api(method, path, body) {
  const opts = { method, headers: { 'Content-Type': 'application/json' } };
  if (body) opts.body = JSON.stringify(body);
  const r = await fetch(path, opts);
  try { return await r.json(); } catch (e) { return {}; }
}

function setPlayingState(playing) {
  state.is_playing = playing;
  const a = document.getElementById('play-svg');
  const b = document.getElementById('hero-play-svg');
  const c = document.getElementById('hero-play-label');
  if (playing) {
    a.innerHTML = '<path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/>';
    b.innerHTML = '<path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/>';
    c.textContent = 'Pause';
  } else {
    a.innerHTML = '<path d="M8 5v14l11-7z"/>';
    b.innerHTML = '<path d="M8 5v14l11-7z"/>';
    c.textContent = 'Play';
  }
}

function updateMediaSession() {
  if (!('mediaSession' in navigator)) return;
  const t = state.current_track;
  const audio = document.getElementById('audio-player');
  navigator.mediaSession.metadata = new MediaMetadata({
    title: t ? t.title : "Aysha's Music",
    artist: t ? t.artist : '',
    album: "Aysha's Music",
    artwork: t ? [{ src: t.thumbnail, sizes: '512x512', type: 'image/jpeg' }] : [],
  });
  navigator.mediaSession.setActionHandler('play', () => { if (!state.is_playing) togglePlay(); });
  navigator.mediaSession.setActionHandler('pause', () => { if (state.is_playing) togglePlay(); });
  navigator.mediaSession.setActionHandler('previoustrack', prevTrack);
  navigator.mediaSession.setActionHandler('nexttrack', nextTrack);
  navigator.mediaSession.setActionHandler('stop', stopPlay);
  if (t && audio) {
    navigator.mediaSession.setPositionState({
      duration: audio.duration || 0,
      position: audio.currentTime || 0,
      playbackRate: 1,
    });
  }
}

async function getStreamUrl(t) {
  const r = await api('GET', '/api/stream?url=' + encodeURIComponent(t.url));
  return r.stream_url || '';
}

async function playSong(t) {
  const thumb = 'https://img.youtube.com/vi/' + t.id + '/hqdefault.jpg';
  document.getElementById('hero-art').src = thumb;
  document.getElementById('hero-title').textContent = t.title;
  document.getElementById('hero-artist').textContent = t.artist;
  document.getElementById('np-art').src = thumb;
  document.getElementById('np-title').textContent = t.title;
  document.getElementById('np-artist').textContent = t.artist;
  document.getElementById('time-display').textContent = 'Loading stream...';
  setPlayingState(true);
  state.current_track = t;

  const streamUrl = await getStreamUrl(t);
  const audio = document.getElementById('audio-player');
  if (streamUrl) {
    audio.src = streamUrl;
    audio.volume = state.volume / 100;
    audio.play().catch(function(e) {
      console.log('Autoplay blocked', e);
      document.getElementById('time-display').textContent = 'Tap Play to start';
    });
  } else {
    document.getElementById('time-display').textContent = 'Stream unavailable';
  }

  await api('POST', '/api/play', t);
  updateMediaSession();
}

async function togglePlay() {
  const audio = document.getElementById('audio-player');
  if (state.is_playing) {
    audio.pause();
  } else {
    audio.play().catch(function(e) { console.log('Play failed', e); });
  }
  setPlayingState(!state.is_playing);
  await api('POST', '/api/pause');
}

async function stopPlay() {
  const audio = document.getElementById('audio-player');
  audio.pause();
  audio.src = '';
  audio.currentTime = 0;
  setPlayingState(false);
  document.getElementById('progress-bar').style.width = '0%';
  document.getElementById('time-display').textContent = '0:00 / 0:00';
  state.current_track = null;
  await api('POST', '/api/stop');
  updateMediaSession();
}

async function nextTrack() { await api('POST', '/api/next'); }
async function prevTrack() { await api('POST', '/api/prev'); }

async function seekTrack(e) {
  const audio = document.getElementById('audio-player');
  if (!audio.duration || audio.duration === 0) return;
  const r = e.currentTarget.getBoundingClientRect();
  const pct = (e.clientX - r.left) / r.width;
  audio.currentTime = pct * audio.duration;
}

async function changeVol(v) {
  const audio = document.getElementById('audio-player');
  audio.volume = v / 100;
  await api('POST', '/api/volume?val=' + v);
}

function escapeHtml(s) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

function renderGrid(tracks, containerId) {
  const c = document.getElementById(containerId);
  c.innerHTML = '';
  if (!tracks || !tracks.length) {
    c.innerHTML = '<div class="empty-msg">No results found.</div>';
    return;
  }
  tracks.forEach(t => {
    const isLiked = state.liked.some(l => l.id === t.id);
    const qIn = state.queue ? state.queue.some(q => q.id === t.id) : false;
    const card = document.createElement('div');
    card.className = 'song-card';
    card.innerHTML =
      '<img class="card-thumb" src="https://img.youtube.com/vi/' + t.id + '/hqdefault.jpg">' +
      '<div class="card-info">' +
      '<div class="card-title">' + escapeHtml(t.title) + '</div>' +
      '<div class="card-artist">' + escapeHtml(t.artist) + '</div>' +
      '</div>' +
      '<button class="card-action" onclick="event.stopPropagation(); playSong(' + JSON.stringify(t) + ')" title="Play">' +
      '<svg viewBox="0 0 24 24"><path d="M8 5v14l11-7z"/></svg></button>' +
      '<button class="card-action' + (qIn ? ' active' : '') + '" onclick="event.stopPropagation(); queueAdd(' + JSON.stringify(t) + ')" title="Add to Queue">' +
      '<svg viewBox="0 0 24 24"><path d="M3 5v14h2V5H3zm14 14v-14h2v14h-2zm-6-4v-14h2v14h-2z"/></svg></button>' +
      '<button class="card-heart' + (isLiked ? ' liked' : '') + '" onclick="event.stopPropagation(); toggleLike(' + JSON.stringify(t) + ')">' +
      (isLiked ? '<svg viewBox="0 0 24 24" fill="currentColor"><path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z"/></svg>' :
       '<svg viewBox="0 0 24 24" fill="currentColor"><path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z"/></svg>') +
      '</button>';
    card.onclick = () => playSong(t);
    c.appendChild(card);
  });
}

async function toggleLike(t) {
  const already = state.liked.some(l => l.id === t.id);
  if (already) {
    await api('POST', '/api/unlike', { id: t.id });
    state.liked = state.liked.filter(l => l.id !== t.id);
  } else {
    await api('POST', '/api/like', t);
    state.liked.push(t);
  }
  document.getElementById('liked-count').textContent = state.liked.length;
  if (document.getElementById('tab-home').classList.contains('active')) {
    loadHome();
  } else if (document.getElementById('tab-library').classList.contains('active')) {
    loadLiked();
  }
}

async function searchMusic(q) {
  if (!q) return;
  switchTab('explore');
  document.getElementById('explore-title').textContent = 'Results for "' + q + '"';
  const d = await api('GET', '/api/search?q=' + encodeURIComponent(q));
  const tracks = Array.isArray(d) ? d : (d.results || []);
  if (d.error) {
    document.getElementById('explore-cards').innerHTML = '<div class="empty-msg">' + escapeHtml(d.error) + '</div>';
  }
  renderGrid(tracks, 'explore-cards');
}

async function fetchCategory(cat) {
  document.querySelectorAll('.chip').forEach(c => c.classList.remove('active'));
  if (event && event.target) event.target.classList.add('active');
  document.getElementById('explore-title').textContent = cat;
  const d = await api('GET', '/api/search?q=' + encodeURIComponent(cat + ' songs'));
  const tracks = Array.isArray(d) ? d : (d.results || []);
  if (d.error) {
    document.getElementById('explore-cards').innerHTML = '<div class="empty-msg">' + escapeHtml(d.error) + '</div>';
  }
  renderGrid(tracks, 'explore-cards');
}

async function loadHome() {
  const picks = await api('GET', '/api/search?q=Aysha%20love%20songs');
  const pt = Array.isArray(picks) ? picks : (picks.results || []);
  renderGrid(pt, 'quick-picks');
  const hist = await api('GET', '/api/liked');
  state.liked = Array.isArray(hist) ? hist : (hist.results || []);
  document.getElementById('liked-count').textContent = state.liked.length;
  const ht = Array.isArray(hist) ? hist : (hist.results || []);
  renderGrid(ht, 'recent-history');
  loadQueue();
}

async function loadLiked() {
  const hist = await api('GET', '/api/liked');
  state.liked = Array.isArray(hist) ? hist : (hist.results || []);
  document.getElementById('liked-count').textContent = state.liked.length;
  const ht = Array.isArray(hist) ? hist : (hist.results || []);
  renderGrid(ht, 'full-history');
  loadQueue();
}

async function loadQueue() {
  const q = await api('GET', '/api/queue');
  const tracks = Array.isArray(q) ? q : (q.results || []);
  state.queue = tracks;
  document.getElementById('queue-count').textContent = tracks.length;
  renderGrid(tracks, 'queue-list');
}

async function queueAdd(t) {
  await api('POST', '/api/queue_add', t);
  state.queue = state.queue || [];
  if (!state.queue.some(q => q.id === t.id)) {
    state.queue.push(t);
  }
  loadQueue();
}

async function queueClear() {
  await api('POST', '/api/queue_clear');
  state.queue = [];
  loadQueue();
}

async function loadBluetooth() {
  const st = await api('GET', '/api/bt/status');
  document.getElementById('bt-status-text').textContent = st.connected ? 'Status: Connected' : 'Status: Disconnected';
  document.getElementById('bt-device-text').textContent = st.device || 'No device active';
  document.getElementById('bt-list').innerHTML = st.connected ?
    '<div class="bt-card"><div><strong>' + escapeHtml(st.device) + '</strong><small>' + st.mac + '</small></div><button class="chip" onclick="disconnectBT()">Disconnect</button></div>' :
    '<div class="empty-msg">No device connected.</div>';
}

async function disconnectBT() {
  await api('POST', '/api/bt/connect?mac=04:CB:88:92:43:60');
  setTimeout(loadBluetooth, 2000);
}

async function scanBluetooth() {
  document.getElementById('bt-list').innerHTML = '<div class="empty-msg">Scanning...</div>';
  await api('POST', '/api/bt/connect?mac=04:CB:88:92:43:60');
  loadBluetooth();
}

function switchTab(tab) {
  document.querySelectorAll('.tab-page').forEach(p => p.classList.remove('active'));
  document.querySelectorAll('.nav-item').forEach(n => n.classList.remove('active'));
  document.getElementById('tab-' + tab).classList.add('active');
  document.getElementById('nav-' + tab).classList.add('active');
  if (tab === 'bluetooth') loadBluetooth();
  if (tab === 'library') loadLiked();
  if (tab === 'explore') fetchCategory('Jazz');
  if (tab === 'home') loadHome();
}

setInterval(async () => {
  try {
    const st = await api('GET', '/api/status');
    if (st.current_track) {
      document.getElementById('hero-title').textContent = st.current_track.title;
      document.getElementById('hero-artist').textContent = st.current_track.artist;
      document.getElementById('hero-art').src = 'https://img.youtube.com/vi/' + st.current_track.id + '/hqdefault.jpg';
      document.getElementById('np-title').textContent = st.current_track.title;
      document.getElementById('np-artist').textContent = st.current_track.artist;
      document.getElementById('np-art').src = 'https://img.youtube.com/vi/' + st.current_track.id + '/hqdefault.jpg';
      state.current_track = st.current_track;
      state.volume = st.volume;
      setPlayingState(st.is_playing);
      updateMediaSession();
    }
    if (st.queue_len !== undefined) {
      document.getElementById('queue-count').textContent = st.queue_len;
    }
    const audio = document.getElementById('audio-player');
    if (audio && audio.duration > 0) {
      durationSec = audio.duration;
      const pct = (audio.currentTime / audio.duration) * 100;
      document.getElementById('progress-bar').style.width = pct + '%';
      const m1 = Math.floor(audio.currentTime / 60), s1 = Math.floor(audio.currentTime % 60);
      const m2 = Math.floor(audio.duration / 60), s2 = Math.floor(audio.duration % 60);
      document.getElementById('time-display').textContent = m1 + ':' + (s1 < 10 ? '0' : '') + s1 + ' / ' + m2 + ':' + (s2 < 10 ? '0' : '') + s2;
      if (audio.ended) {
        await api('POST', '/api/next');
      }
    } else if (state.is_playing) {
      document.getElementById('time-display').textContent = 'Loading...';
    }
  } catch (e) {}
}, 400);

if ('serviceWorker' in navigator) {
  navigator.serviceWorker.register('/sw.js').catch(() => {});
}
if ('mediaSession' in navigator) {
  updateMediaSession();
}

switchTab('home');
</script>
</body>
</html>"""

MANIFEST_JSON = json.dumps({
    "name": "Aysha's Music",
    "short_name": "Aysha Music",
    "description": "Music player made with love for Aysha",
    "start_url": "/",
    "display": "standalone",
    "background_color": "#030303",
    "theme_color": "#ff0000",
    "orientation": "portrait",
    "icons": [{
        "src": "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='%23ff0000'%3E%3Cpath d='M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z'/%3E%3C/svg%3E",
        "sizes": "any",
        "type": "image/svg+xml",
        "purpose": "any maskable"
    }]
}, indent=2)

SW_JS = """const CACHE = 'aysha-music-v1';
const ASSETS = ['/', '/manifest.json'];
self.addEventListener('install', (e) => {
  e.waitUntil(caches.open(CACHE).then((c) => c.addAll(ASSETS)));
});
self.addEventListener('fetch', (e) => {
  e.respondWith(caches.match(e.request).then((r) => r || fetch(e.request)));
});
"""


class Handler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *args):
        pass

    def _send(self, code, body, ctype="application/json"):
        b = body.encode() if isinstance(body, str) else body
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(b)))
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(b)

    def do_GET(self):
        u = urlparse(self.path)
        path = u.path
        qs = parse_qs(u.query)

        if path == "/":
            self._send(200, INDEX_HTML, "text/html; charset=utf-8")
        elif path == "/manifest.json":
            self._send(200, MANIFEST_JSON, "application/manifest+json")
        elif path == "/sw.js":
            self._send(200, SW_JS, "application/javascript")
        elif path == "/api/status":
            with state_lock:
                s = dict(state)
            self._send(200, json.dumps({
                "current_track": s["current_track"],
                "is_playing": s["playing"],
                "volume": s["volume"],
                "queue_len": len(s["queue"]),
                "liked_count": len(s["liked"]),
            }))
        elif path == "/api/search":
            tracks, err = search_youtube(qs.get("q", [""])[0])
            if err:
                self._send(200, json.dumps({"error": err, "results": tracks}))
            elif not tracks:
                self._send(200, json.dumps({"error": "No results found. Check network access to YouTube.", "results": []}))
            else:
                self._send(200, json.dumps(tracks))
        elif path == "/api/stream":
            url = qs.get("url", [""])[0]
            stream_url = get_stream_url(url)
            if stream_url:
                self._send(200, json.dumps({"stream_url": stream_url}))
            else:
                self._send(200, json.dumps({"stream_url": "", "error": "Could not resolve stream URL"}))
        elif path == "/api/queue":
            with state_lock:
                q = list(state["queue"])
            self._send(200, json.dumps(q))
        elif path == "/api/liked":
            with state_lock:
                l = list(state["liked"])
            self._send(200, json.dumps(l))
        elif path == "/api/bt/status":
            self._send(200, json.dumps(bt_status()))
        elif path == "/api/tailscale/status":
            self._send(200, json.dumps(tailscale_status()))
        else:
            self._send(404, json.dumps({"error": "not found"}))

    def do_POST(self):
        u = urlparse(self.path)
        path = u.path
        qs = parse_qs(u.query)
        length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(length) if length else b""
        payload = {}
        if body:
            try:
                payload = json.loads(body)
            except Exception:
                payload = {}

        if path == "/api/play":
            t = {
                "url": payload.get("url", ""),
                "title": payload.get("title", ""),
                "artist": payload.get("artist", ""),
                "thumbnail": payload.get("thumbnail", ""),
                "id": payload.get("id", ""),
                "duration": payload.get("duration", 0),
            }
            with state_lock:
                if state["current_track"]:
                    state["history"].insert(0, state["current_track"])
                    if len(state["history"]) > MAX_HISTORY:
                        state["history"].pop()
                state["current_track"] = t
                state["playing"] = True
                state["paused"] = False
            add_history(t)
            self._send(200, json.dumps({"status": "playing"}))
            threading.Thread(target=autoplay_enqueue, args=(t,), daemon=True).start()
        elif path == "/api/pause":
            with state_lock:
                state["paused"] = not state["paused"]
                state["playing"] = state["playing"] and not state["paused"]
            self._send(200, json.dumps({"status": "paused" if state["paused"] else "playing"}))
        elif path == "/api/stop":
            with state_lock:
                state["playing"] = False
                state["paused"] = False
                state["current_track"] = None
            self._send(200, json.dumps({"status": "stopped"}))
        elif path == "/api/seek":
            self._send(200, json.dumps({"status": "seeked"}))
        elif path == "/api/volume":
            val = min(100, int(qs.get("val", [80])[0]))
            with state_lock:
                state["volume"] = val
            self._send(200, json.dumps({"status": "ok", "volume": val}))
        elif path == "/api/volume/up":
            step = int(qs.get("step", [5])[0])
            with state_lock:
                state["volume"] = min(100, state["volume"] + step)
                val = state["volume"]
            self._send(200, json.dumps({"status": "ok", "volume": val}))
        elif path == "/api/volume/down":
            step = int(qs.get("step", [5])[0])
            with state_lock:
                state["volume"] = max(0, state["volume"] - step)
                val = state["volume"]
            self._send(200, json.dumps({"status": "ok", "volume": val}))
        elif path == "/api/next":
            with state_lock:
                q = list(state["queue"])
            if q:
                track = q[0]
                with state_lock:
                    state["queue"] = state["queue"][1:]
                    state["current_track"] = track
                    state["playing"] = True
                add_history(track)
                self._send(200, json.dumps({"status": "next"}))
            else:
                with state_lock:
                    state["playing"] = False
                self._send(200, json.dumps({"status": "stopped"}))
        elif path == "/api/prev":
            with state_lock:
                h = list(state["history"])
            if h:
                track = h[0]
                with state_lock:
                    state["history"] = state["history"][1:]
                    state["current_track"] = track
                    state["playing"] = True
                add_history(track)
                self._send(200, json.dumps({"status": "prev"}))
            else:
                self._send(200, json.dumps({"status": "no_prev"}))
        elif path == "/api/queue_add":
            tid = payload.get("id", "")
            if not tid:
                self._send(400, json.dumps({"error": "missing id"}))
                return
            track = {
                "id": tid,
                "title": payload.get("title", ""),
                "artist": payload.get("artist", ""),
                "thumbnail": payload.get("thumbnail", ""),
                "url": payload.get("url", ""),
                "duration": payload.get("duration", 0),
            }
            with state_lock:
                if len(state["queue"]) < MAX_QUEUE and not any(q.get("id") == tid for q in state["queue"]):
                    state["queue"].append(track)
            self._send(200, json.dumps({"status": "queued"}))
        elif path == "/api/queue_clear":
            with state_lock:
                state["queue"] = []
            self._send(200, json.dumps({"status": "cleared"}))
        elif path == "/api/bt/connect":
            mac = qs.get("mac", [""])[0]
            self._send(200, json.dumps(bt_connect(mac)))
        elif path == "/api/like":
            tid = payload.get("id", "")
            if not tid:
                self._send(400, json.dumps({"error": "missing id"}))
                return
            track = {
                "id": tid,
                "title": payload.get("title", ""),
                "artist": payload.get("artist", ""),
                "thumbnail": payload.get("thumbnail", ""),
                "url": payload.get("url", ""),
                "duration": payload.get("duration", 0),
            }
            with state_lock:
                if not any(t.get("id") == tid for t in state["liked"]):
                    state["liked"].append(track)
            save_tracks(LIKED_FILE, state["liked"])
            self._send(200, json.dumps({"status": "liked"}))
        elif path == "/api/unlike":
            tid = payload.get("id", "")
            with state_lock:
                state["liked"] = [t for t in state["liked"] if t.get("id") != tid]
            save_tracks(LIKED_FILE, state["liked"])
            self._send(200, json.dumps({"status": "unliked"}))
        else:
            self._send(404, json.dumps({"error": "not found"}))


def main():
    # Load liked songs
    with state_lock:
        state["liked"] = load_tracks(LIKED_FILE)
    print("Tools:", json.dumps(TOOLS))
    print("Aysha's Music Player on http://0.0.0.0:%d" % PORT)
    http.server.ThreadingHTTPServer(("0.0.0.0", PORT), Handler).serve_forever()


if __name__ == "__main__":
    main()
