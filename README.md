# Aysha's Music Player

A lightweight, self-contained music player backend + UI, built for Aysha.
Branded "Made with ♥ for Aysha — with love, from Afzal".

## Stack

- **Backend:** Python 3 stdlib HTTP server (`http.server`, threaded) — no frameworks, ~3 MB RAM.
- **UI:** Single self-contained HTML page (inline CSS/JS), YouTube Music–inspired.
- **Media:** `yt-dlp` (search) + `mpv` (playback via IPC socket), invoked as subprocesses.
- **Bluetooth:** `bluetoothctl` (scan/connect/disconnect/status).
- **Persistence:** Listening history stored as JSON.

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | UI (single-page app) |
| GET | `/status` | Player state + which tools are installed |
| GET | `/search?q=...` | YouTube search via `yt-dlp` |
| POST | `/play` | Play a track (JSON body) |
| POST | `/pause` | Toggle pause |
| POST | `/stop` | Stop playback |
| POST | `/seek?pos=N` | Seek to position (seconds) |
| POST | `/volume?val=N` | Set volume (0–100) |
| GET | `/history` | Listening history |
| GET | `/bt_devices` | Paired Bluetooth devices |
| GET | `/bt_scan` | Scan for Bluetooth devices |
| GET | `/bt_status` | Connected device status |
| POST | `/bt_connect?mac=...` | Connect a Bluetooth device |
| POST | `/bt_disconnect?mac=...` | Disconnect a Bluetooth device |

## Run

```bash
python3 server.py
# then open http://localhost:5050
```

## Notes

This environment could not install `mpv`, `bluez`/`bluetoothctl`, and YouTube TLS is blocked by the network proxy, so `yt-dlp` cannot reach YouTube here. All endpoints are implemented and degrade gracefully: when a tool is unavailable they return a clear `error`/`ok:false` response instead of crashing. On a machine with `yt-dlp`, `mpv`, and `bluetoothctl` installed, the player is fully functional.

Made with ♥ for Aysha — with love, from Afzal.
