#!/usr/bin/env python3
"""Aysha's Music Player - lightweight stdlib HTTP server.

Endpoints:
  GET     /
  GET     /status
  GET     /search?q=...
  POST    /play          (json {url,title,artist,id})
  POST    /pause
  POST    /stop
  POST    /seek?pos=N
  POST    /volume?val=N
  GET     /history
  GET     /bt_devices
  GET     /bt_scan
  GET     /bt_status
  POST    /bt_connect?mac=...
  POST    /bt_disconnect?mac=...

Designed to run under ~100MB RAM: only Python stdlib in-process; yt-dlp/mpv/
bluetoothctl are invoked as external subprocesses.
"""
import http.server
import json
import os
import socket
import subprocess
import threading
from urllib.parse import urlparse, parse_qs

PORT = int(os.environ.get("PORT", "5050"))
SOCK = "/tmp/aysha-mpv.sock"
HIST_FILE = "/tmp/aysha-history.json"
MAX_HISTORY = 50

# Which external tools exist (checked lazily / at startup)
TOOLS = {
    "yt-dlp": None,
    "mpv": None,
    "bluetoothctl": None,
}

for name in TOOLS:
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
    "title": "",
    "artist": "",
    "id": "",
    "url": "",
    "duration": 0,
    "pos": 0,
    "volume": 100,
    "queue": [],
    "history": [],
    "tools": TOOLS,
}
state_lock = threading.Lock()
mpv_proc = [None]


def run_cmd(cmd, timeout=20):
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
        return r.returncode, r.stdout, r.stderr
    except Exception as e:
        return -1, "", str(e)


def tool(name):
    return TOOLS.get(name)


def with_state(fn):
    with state_lock:
        return fn(dict(state))


# ---------------------------------------------------------------------------
# mpv IPC helpers
# ---------------------------------------------------------------------------
def mpv_ipc(cmd_obj, timeout=2.0):
    if not tool("mpv") or not os.path.exists(SOCK):
        return None
    try:
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        s.settimeout(timeout)
        s.connect(SOCK)
        s.sendall((json.dumps(cmd_obj) + "\n").encode())
        data = b""
        while True:
            chunk = s.recv(4096)
            if not chunk:
                break
            data += chunk
            if b"\n" in data:
                break
        s.close()
        line = data.decode().strip().split("\n")[-1]
        return json.loads(line)
    except Exception:
        return None


def mpv_prop(prop, req_id=1):
    resp = mpv_ipc({"command": ["get_property", prop], "request_id": req_id})
    if resp and resp.get("error") == "success":
        return resp.get("data")
    return None


def refresh_state():
    st = with_state(lambda s: s)
    if tool("mpv") and os.path.exists(SOCK):
        pos = mpv_prop("time-pos", 1)
        dur = mpv_prop("duration", 2)
        pause = mpv_prop("pause", 3)
        if pos is not None:
            st["pos"] = float(pos)
        if dur is not None:
            st["duration"] = float(dur)
        if pause is not None:
            st["paused"] = bool(pause)
            st["playing"] = st["playing"] and not st["paused"]
    return st


def start_mpv(url, volume):
    stop_mpv()
    if not tool("mpv"):
        return False, "mpv not installed"
    try:
        if os.path.exists(SOCK):
            os.remove(SOCK)
    except Exception:
        pass
    try:
        mpv_proc[0] = subprocess.Popen(
            [
                "mpv", "--no-video", "--really-quiet", "--no-terminal",
                "--idle=yes", "--keep-open=no",
                "--ytdl-format=bestaudio/best",
                "--volume=" + str(volume),
                "--input-ipc-server=" + SOCK,
                url,
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    except Exception as e:
        return False, str(e)
    with state_lock:
        state["playing"] = True
        state["paused"] = False
        state["pos"] = 0
        state["duration"] = 0
    return True, None


def stop_mpv():
    p = mpv_proc[0]
    if p and p.poll() is None:
        try:
            if tool("mpv") and os.path.exists(SOCK):
                mpv_ipc({"command": ["stop"], "request_id": 0}, timeout=1.0)
        except Exception:
            pass
        try:
            p.terminate()
            p.wait(timeout=3)
        except Exception:
            try:
                p.kill()
            except Exception:
                pass
    mpv_proc[0] = None


# ---------------------------------------------------------------------------
# history persistence (tiny JSON)
# ---------------------------------------------------------------------------
def load_history():
    try:
        with open(HIST_FILE, "r") as f:
            return json.load(f)
    except Exception:
        return []


def save_history(h):
    try:
        with open(HIST_FILE, "w") as f:
            json.dump(h[-MAX_HISTORY:], f)
    except Exception:
        pass


def add_history(track):
    h = load_history()
    if not any(t.get("id") == track.get("id") for t in h):
        h.insert(0, track)
    save_history(h)
    with state_lock:
        state["history"] = h[-MAX_HISTORY:]


# ---------------------------------------------------------------------------
# youtube search via yt-dlp CLI (subprocess, keeps this process light)
# ---------------------------------------------------------------------------
def search_youtube(query, n=12):
    if not tool("yt-dlp"):
        return []
    rc, out, err = run_cmd(
        ["yt-dlp", "--flat-playlist", "-j", "ytsearch%d:%s" % (n, query)],
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
                "duration": v.get("duration_string") or "",
                "url": "https://www.youtube.com/watch?v=" + vid,
            })
    return tracks


# ---------------------------------------------------------------------------
# bluetooth helpers
# ---------------------------------------------------------------------------
def bt_devices_list():
    if not tool("bluetoothctl"):
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
    if not tool("bluetoothctl"):
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


def bt_action(mac, action):
    if not tool("bluetoothctl"):
        return {"ok": False, "error": "bluetoothctl not installed"}
    run_cmd(["bluetoothctl", "trust", mac], timeout=10)
    rc, out, err = run_cmd(["bluetoothctl", action, mac], timeout=15)
    ok = rc == 0 and ("successful" in out or "Connected" in out or action == "disconnect")
    if action == "connect" and ok:
        run_cmd(["pactl", "set-card-profile", "bluez_card", "a2dp_sink"], timeout=5)
    return {"ok": ok}


# ---------------------------------------------------------------------------
# HTML UI (self-contained, inline CSS/JS)
# ---------------------------------------------------------------------------
INDEX_HTML = r"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1,maximum-scale=1,user-scalable=no">
<title>Aysha's Music Player</title>
<style>
:root{--bg:#030303;--sidebar:#000000;--panel:#121212;--accent:#ff0000;--text:#fff;--sub:#aaaaaa;--dim:#717171;--border:rgba(255,255,255,0.08)}
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
.song-card{display:flex;align-items:center;gap:12px;padding:10px;border-radius:10px;background:rgba(255,255,255,0.03);border:1px solid transparent;transition:all .2s;cursor:pointer}
.song-card:hover{background:rgba(255,255,255,0.08);border-color:var(--border)}
.card-thumb{width:80px;aspect-ratio:16/9;object-fit:cover;border-radius:6px;background:#202020;flex-shrink:0}
.card-info{overflow:hidden;flex:1}
.card-title{font-size:13px;font-weight:600;color:#fff;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.card-artist{font-size:11px;color:var(--sub);margin-top:3px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.card-action{background:rgba(255,255,255,0.1);border:none;color:#fff;width:34px;height:34px;border-radius:50%;display:flex;align-items:center;justify-content:center;cursor:pointer;flex-shrink:0}
.card-action svg{width:18px;height:18px;fill:currentColor}
.chips-row{display:flex;gap:10px;overflow-x:auto;padding-bottom:12px;margin-bottom:20px}
.chip{background:rgba(255,255,255,0.08);color:#fff;padding:8px 18px;border-radius:18px;font-size:13px;font-weight:600;cursor:pointer;white-space:nowrap;border:1px solid transparent}
.chip:hover,.chip.active{background:#fff;color:#000}
.player-bar{position:absolute;bottom:0;left:0;right:0;height:72px;background:rgba(18,18,18,0.96);backdrop-filter:blur(16px);border-top:1px solid var(--border);display:flex;flex-direction:column;z-index:100}

/* Modern slim scrollbars */
*::-webkit-scrollbar{width:6px;height:6px}
*::-webkit-scrollbar-track{background:transparent}
*::-webkit-scrollbar-thumb{background:rgba(255,255,255,0.12);border-radius:10px}
*::-webkit-scrollbar-thumb:hover{background:rgba(255,255,255,0.25)}
.content-area{scrollbar-width:thin;scrollbar-color:rgba(255,255,255,0.12) transparent}
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
      <div class="section-title">Listening History</div>
      <div class="grid-cards" id="full-history"></div>
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
        <div class="vol-box"><input type="range" min="0" max="100" step="5" value="100" oninput="changeVol(this.value)"></div>
      </div>
    </div>
  </div>
</div>
<script>
let isPlaying=false,durationSec=0;
function switchTab(tab){
  document.querySelectorAll('.tab-page').forEach(p=>p.classList.remove('active'));
  document.querySelectorAll('.nav-item').forEach(n=>n.classList.remove('active'));
  document.getElementById('tab-'+tab).classList.add('active');
  document.getElementById('nav-'+tab).classList.add('active');
  if(tab==='bluetooth')loadBluetooth();
  if(tab==='library')loadLibrary();
  if(tab==='explore')fetchCategory('Jazz');
}
function setPlayingState(playing){
  isPlaying=playing;
  const a=document.getElementById('play-svg'),b=document.getElementById('hero-play-svg'),c=document.getElementById('hero-play-label');
  if(playing){a.innerHTML='<path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/>';b.innerHTML='<path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/>';c.textContent='Pause'}
  else{a.innerHTML='<path d="M8 5v14l11-7z"/>';b.innerHTML='<path d="M8 5v14l11-7z"/>';c.textContent='Play'}
}
async function api(method,path,body){
  const opts={method,headers:{}};
  if(body){opts.headers['Content-Type']='application/json';opts.body=JSON.stringify(body)}
  const r=await fetch(path,opts);
  try{return await r.json()}catch(e){return {}}
}
async function playSong(t){
  const thumb='https://img.youtube.com/vi/'+t.id+'/hqdefault.jpg';
  document.getElementById('hero-art').src=thumb;
  document.getElementById('hero-title').textContent=t.title;
  document.getElementById('hero-artist').textContent=t.artist;
  document.getElementById('np-art').src=thumb;
  document.getElementById('np-title').textContent=t.title;
  document.getElementById('np-artist').textContent=t.artist;
  document.getElementById('time-display').textContent='Loading...';
  setPlayingState(true);
  await api('POST','/play',t);
}
async function togglePlay(){
  setPlayingState(!isPlaying);
  await api('POST','/pause');
}
async function stopPlay(){
  setPlayingState(false);
  document.getElementById('progress-bar').style.width='0%';
  document.getElementById('time-display').textContent='0:00 / 0:00';
  await api('POST','/stop');
}
async function seekTrack(e){
  if(!durationSec)return;
  const r=e.currentTarget.getBoundingClientRect();
  const pct=(e.clientX-r.left)/r.width;
  await api('POST','/seek?pos='+(pct*durationSec));
}
async function changeVol(v){await api('POST','/volume?val='+v);}
function renderGrid(tracks,id){
  const c=document.getElementById(id);c.innerHTML='';
  if(!tracks||!tracks.length){
    c.innerHTML='<div style="color:var(--sub);padding:24px;text-align:center">No results found.</div>';
    return;
  }
  tracks.forEach(t=>{
    const card=document.createElement('div');card.className='song-card';
    card.innerHTML='<img class="card-thumb" src="https://img.youtube.com/vi/'+t.id+'/hqdefault.jpg">'+
      '<div class="card-info"><div class="card-title">'+t.title+'</div><div class="card-artist">'+t.artist+'</div></div>'+
      '<button class="card-action" title="Play"><svg viewBox="0 0 24 24"><path d="M8 5v14l11-7z"/></svg></button>';
    card.onclick=()=>playSong(t);
    c.appendChild(card);
  });
}
async function searchMusic(q){
  if(!q)return;
  switchTab('explore');
  document.getElementById('explore-title').textContent='Results for "'+q+'"';
  const d=await api('GET','/search?q='+encodeURIComponent(q));
  renderGrid(d,'explore-cards');
}
async function fetchCategory(cat){
  document.querySelectorAll('.chip').forEach(c=>c.classList.remove('active'));
  if(event&&event.target)event.target.classList.add('active');
  document.getElementById('explore-title').textContent=cat;
  const d=await api('GET','/search?q='+encodeURIComponent(cat+' songs'));
  renderGrid(d,'explore-cards');
}
async function loadHome(){
  const picks=await api('GET','/search?q=Aysha%20love%20songs');
  renderGrid(picks,'quick-picks');
  const hist=await api('GET','/history');
  renderGrid(hist,'recent-history');
}
async function loadLibrary(){
  const hist=await api('GET','/history');
  renderGrid(hist,'full-history');
}
async function loadBluetooth(){
  const st=await api('GET','/bt_status');
  document.getElementById('bt-status-text').textContent=st.connected?'Status: Connected':'Status: Disconnected'+(st.error?' &mdash; '+st.error:'');
  document.getElementById('bt-device-text').textContent=st.device||'No device active';
  const devs=await api('GET','bt_devices'.replace('bt','/bt'));
  const list=document.getElementById('bt-list');list.innerHTML='';
  (devs||[]).forEach(d=>{
    list.innerHTML+='<div class="bt-card"><div><b>'+d.name+'</b><small>'+d.mac+'</small></div><button class="chip" onclick="connectBT(\''+d.mac+'\')">Connect</button></div>';
  });
}
async function scanBluetooth(){
  document.getElementById('bt-list').innerHTML='<div style="color:var(--sub)">Scanning...</div>';
  await api('GET','/bt_scan');
  loadBluetooth();
}
async function connectBT(mac){
  await api('POST','/bt_connect?mac='+encodeURIComponent(mac));
  setTimeout(loadBluetooth,2000);
}
setInterval(async()=>{
  try{
    const st=await api('GET','/status');
    if(st.track){
      document.getElementById('hero-title').textContent=st.track.title;
      document.getElementById('hero-artist').textContent=st.track.artist;
      document.getElementById('hero-art').src='https://img.youtube.com/vi/'+st.track.id+'/hqdefault.jpg';
      document.getElementById('np-title').textContent=st.track.title;
      document.getElementById('np-artist').textContent=st.track.artist;
      document.getElementById('np-art').src='https://img.youtube.com/vi/'+st.track.id+'/hqdefault.jpg';
      setPlayingState(st.playing);
    }
    if(st.duration>0){
      durationSec=st.duration;
      const pct=(st.pos/st.duration)*100;
      document.getElementById('progress-bar').style.width=pct+'%';
      const m1=Math.floor(st.pos/60),s1=Math.floor(st.pos%60);
      const m2=Math.floor(st.duration/60),s2=Math.floor(st.duration%60);
      document.getElementById('time-display').textContent=m1+':'+(s1<10?'0':'')+s1+' / '+m2+':'+(s2<10?'0':'')+s2;
    }else if(st.playing&&st.pos===0){
      document.getElementById('time-display').textContent='Buffering...';
    }
  }catch(e){}
},400);
loadHome();
</script>
</body>
</html>"""


class Handler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *args):
        pass  # quiet

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
            self._send(200, INDEX_HTML, "text/html")
        elif path == "/status":
            self._send(200, json.dumps(refresh_state()))
        elif path == "/search":
            self._send(200, json.dumps(search_youtube(qs.get("q", [""])[0])))
        elif path == "/history":
            self._send(200, json.dumps(load_history()))
        elif path == "/bt_devices":
            self._send(200, json.dumps(bt_devices_list()))
        elif path == "/bt_scan":
            bt_devices_list()  # side effect: bluetoothctl scan on isn't supported without interactive; just list
            self._send(200, json.dumps(bt_devices_list()))
        elif path == "/bt_status":
            self._send(200, json.dumps(bt_status()))
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

        if path == "/play":
            t = {
                "url": payload.get("url", ""),
                "title": payload.get("title", ""),
                "artist": payload.get("artist", ""),
                "id": payload.get("id", ""),
            }
            ok, err = start_mpv(t["url"], state["volume"])
            if ok:
                add_history(t)
                with state_lock:
                    state["current_track"] = t
            self._send(200, json.dumps({"status": "ok" if ok else "error", "track": t, "error": err}))
        elif path == "/pause":
            run_cmd(["mpv", "--input-ipc-server=" + SOCK, "--no-video", "--really-quiet"], timeout=1) if False else None
            mpv_ipc({"command": ["cycle", "pause"], "request_id": 0})
            with state_lock:
                state["paused"] = not state["paused"]
                state["playing"] = state["playing"] and not state["paused"]
            self._send(200, json.dumps({"status": "ok", "paused": state["paused"]}))
        elif path == "/stop":
            stop_mpv()
            with state_lock:
                state["playing"] = False
                state["paused"] = False
            self._send(200, json.dumps({"status": "ok"}))
        elif path == "/seek":
            pos = float(qs.get("pos", [0])[0])
            mpv_ipc({"command": ["seek", pos, "absolute"], "request_id": 0})
            self._send(200, json.dumps({"status": "ok"}))
        elif path == "/volume":
            val = min(100, int(qs.get("val", [100])[0]))
            with state_lock:
                state["volume"] = val
            mpv_ipc({"command": ["set_property", "volume", val], "request_id": 0})
            self._send(200, json.dumps({"status": "ok", "volume": val}))
        elif path == "/bt_connect":
            self._send(200, json.dumps(bt_action(qs.get("mac", [""])[0], "connect")))
        elif path == "/bt_disconnect":
            self._send(200, json.dumps(bt_action(qs.get("mac", [""])[0], "disconnect")))
        else:
            self._send(404, json.dumps({"error": "not found"}))


def main():
    print("Tools:", json.dumps(TOOLS))
    print("Aysha's Music Player on http://0.0.0.0:%d" % PORT)
    http.server.ThreadingHTTPServer(("0.0.0.0", PORT), Handler).serve_forever()


if __name__ == "__main__":
    main()
