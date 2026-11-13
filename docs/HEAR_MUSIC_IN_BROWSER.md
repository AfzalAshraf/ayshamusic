# Hear music on the phone/browser — targeted recovery

## What the supplied output actually proves

The host fast-forwarded to66b9667 and ran the OFFLINE native-evidence checker.
It made no provider request, decoded nothing, and did not install or restart the
server. Expected blocked/exit1 is not a test of today's production browser stream.
The101,171,200byte memory value is from two previously committed observations,
not a new reading. Native acquisition and whole-stack30MB acceptance remain blocked.

The user selected **phone/browser**, not the server JBL. Source inspection found
song selection automatically queried server Bluetooth and chose mpv/JBL if connected.
That can send sound away from this phone. Separately, play() waited for Bluetooth
and server API requests before invocation, risking loss of mobile user activation.
These are source-confirmed routing/activation defects, not proof that they explain
all silence or that the live provider currently delivers usable audio.

## Runtime patch

- Normal song selection defaults to browser output regardless of server Bluetooth.
- Refresh restores the song without autoplay and resets old speaker selections to
  browser output. An explicit in-code bt request still retains the speaker backend;
  no new UI output picker is implied by this patch.
- Assign the stream URL and invoke audio.play() in the tap stack before awaiting
  server registration. Playing is shown only after both succeed; errors leave Play
  available and cancel the local source. No audio-as-JSON fetch or extra media probe.
- A late start completion cannot overwrite Stop/a newer song. Media failures retain
  a retryable selection and identify this-device output.
- Added local-only triage for service/UI deployment/status and fixed recent journal
  error-signature counts. No raw logs, song metadata, signed URLs or arbitrary error
  text included; no download, watch probe, audio controls, restart, install or upload.
  The diagnostic is Python stdlib development tooling, not a production fallback.

## INSTALL — existing compatibility service, NOT the all-Rust candidate

This explicitly rebuilds/restarts the normal Rust server with the embedded routing
fix. It retains the EXISTING yt-dlp/optionalNode downloader path; it does not satisfy
all-Rust or30MB acceptance. No prototype is installed. Do not run a native prototype
or loosen source-access restrictions to make this work.

Run on the confirmed afine UID1000 Linux host (not root):

```bash
cd /home/afine/ayshamusic &&
git pull --ff-only origin arena/a850fb0d-ayshamusic &&
bash scripts/install_local.sh
```

The installer checks dependencies/drop-ins/duplicate servers, runs root Rust tests
and release build before stopping the service, saves timestamped prior unit/binary
backups and starts5050 under systemd. It can ask for normal local sudo authorization;
never share those credentials in chat. It is not an automatic rollback guarantee.
Build failure -> stop; existing service was not replaced. Preflight rejects a missing
tool/runtime/unknown drop-in -> preserve service and report that line; do not delete
locks or install arbitrary versions. Post-stop/install/start failure -> retain backups,
report the exact failing step and use a reviewed rollback; do not rerun blindly.
Do not measure the30MB budget while compiling.

Expected: tests/build succeed, service active, JSON status prints. That is deployment
health only, not proof of audio. The offline native checker SHOULD STILL be blocked.

## LISTEN — one ordinary user-controlled attempt

1. Reload the live server app on the phone/browser (port5050, not a static/Vercel UI
   preview). This HTML is embedded in the Rust binary, so a Git pull alone is not enough.
2. Raise phone **media** volume and select a song permitted for playback.
3. Tap Play. Expected: this-device output and actual sound on the phone. The server's
   JBL connection no longer selects output. No need to disconnect/reset Bluetooth.
4. If the browser says “Tap Play”, tap it once. If sound fails, note the exact visible
   player message; stop retries and use the bounded diagnostic below.

A play() resolution/clock advance alone is not a hearing test. User-confirmed sound
is required. No audible result from this host has been supplied yet.

## DIAGNOSTIC — only if the installed fix still fails

After that one ordinary attempt, as afine:

```bash
cd /home/afine/ayshamusic && python3 scripts/check_browser_playback.py
```

Expected exit2 for partial/inconclusive checks, exit1 for a service/deployment/status
failure. This NEVER exits0 or certifies sound, native extraction or30MB.
Share its JSON plus the exact phone player message, not raw journal pages.

| Result | Next branch |
|---|---|
| SERVICE_RUNNING fail | Fix service/start failure before touching upstream access |
| BROWSER_ROUTING_UI_INSTALLED fail | Old/different embedded UI is served; inspect build/install outcome and5050 origin |
| UPSTREAM_ACCESS_RESTRICTED | Stop; do not use cookies/login, generate tokens or retry clients |
| UPSTREAM_HTTP_403 / DOWNLOADER_FORMAT_UNAVAILABLE | Provider/downloader path needs targeted investigation; no unchanged native-watch trial |
| JS_RUNTIME_MISSING / DOWNLOADER_LAUNCH_FAILED | Inspect existing service dependency paths/versions; no hidden fallback |
| STREAM_NO_AUDIO_HEADER | Stream startup failed, not a Bluetooth defect; review bounded categorized evidence |
| Hints unverified/no signatures | Missing/limited log access or unmatched failure; not a clean-stream pass. Need phone error/result; no sudo/raw log dump requested |

The journal window is at most100recent lines/1MiB over5minutes, may include other
clients and does not prove causal attribution. No unrelated host changes are made.

## Verification record

Runtime patch7433267:111Python tests,79JavaScript tests and diffcheckPASS locally.
CI43422324149 all5jobsPASS, including rootRusttests/release and mobilebrowserfixtures.
CI fixtures do not use live provider playback or prove sound on the user's phone.
