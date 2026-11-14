# Run independently of SSH on the confirmed afine Linux server

## Which computer can be turned off?

- Closing SSH or shutting down the laptop/PC used as the SSH client does not stop a systemd-managed server.
- The Linux host running music-server must remain powered on and awake. A powered-off or suspended local server cannot serve audio.
- Browser-mode audio still needs the listening browser/device to remain open. Bluetooth mode runs mpv on the Linux server; closing the SSH client does not close mpv when it belongs to the service.

## Optional native Rust playback experiment

The user-approved first prototype is a **separate local-file audio command**, not a service replacement. Follow [RUST_AUDIO_PROTOTYPE.md](RUST_AUDIO_PROTOTYPE.md) for its isolated build, prepared-file test, controls and memory reporting. The root Cargo commands and production installer below do not build or install that experiment. Do not replace ExecStart with the prototype.

## Source and compile only

Current work is on `arena/a850fb0d-ayshamusic`. Use the existing checkout of that branch; do not launch a second server alongside systemd.

```bash
cd /home/afine/ayshamusic &&
git pull --ff-only origin arena/a850fb0d-ayshamusic &&
cargo test &&
cargo build --release
```

Output: `/home/afine/ayshamusic/target/release/music-server`. Compiling does not update the installed service executable, which lives at `/home/afine/music-server/target/release/music-server`. To test/install/restart the service safely, use the installer below instead of manually copying over a running binary. It already runs both Cargo commands, so there is no need to compile twice when updating.

The UI and SVGs are embedded at compile time. Current source still requires yt-dlp/Node for extraction and mpv for server-speaker playback; compilation is not a native Rust replacement for these tools. The 100 MB target and verified iPhone compatibility are not completed features of this build. See [the memory and native-helper proposal](MEMORY_BUDGET_PLAN.md).

## One-time installation / update

This installer is deliberately for the verified local host: afine UID 1000, /home/afine, Node at /usr/bin/node, yt-dlp at /home/afine/.local/bin/yt-dlp. It refuses a different user/UID or existing service drop-ins that have not been reviewed. It does not install Python as the server or deploy anything to a cloud provider.

1. Stop any foreground `./target/release/music-server` using Ctrl-C in its terminal.
2. Run as afine (do not run the script itself with sudo):

```bash
cd /home/afine/ayshamusic
git pull --ff-only origin arena/a850fb0d-ayshamusic &&
bash scripts/install_local.sh
```

### Confirm the refresh fix is actually served

The UI is embedded by Rust's `include_str!`: `git pull` or refreshing the phone alone does **not** replace the running executable. Run the installer above after stopping any foreground copy. Rebuilding a repository binary without installing it at the systemd ExecStart path is also insufficient.

On the Linux host after installation:

```bash
curl --fail --silent --show-error http://127.0.0.1:5050/ | grep -F "const selectionKey = 'aysha.selection.v1';"
```

The expected output is that exact declaration. If it is absent, inspect `systemctl status aysha-music` and `systemctl show aysha-music -p ExecStart -p MainPID`: the endpoint is not serving this fix. Do not run a second server on port 5050. If local output is correct but the phone still gets old behavior, verify it uses this same server/address and reload the page; a different origin has separate tab storage. Do not erase site storage as the first fix, since that removes the saved selection.

After loading the updated page, select a song once (the old version did not save one), Stop, Play, Stop, refresh, and Play again. Confirm the same title/artist remains and music audibly restarts. Refresh ends browser audio by design; it should no longer make the song unplayable. There is no uninterrupted-refresh or cross-tab persistence promise.

The script tests and builds before stopping the service, backs up the existing unit and binary, atomically installs the newly built Rust binary to the confirmed ExecStart path, installs deploy/aysha-music.service, enables lingering for afine, and enables/starts the system service. It verifies service liveness and the status endpoint. A failing installation step stops the script; it does not claim a successful install or automatically roll back.

The unit explicitly provides HOME, PATH, XDG_RUNTIME_DIR and PULSE_SERVER for the known local audio session. MUSIC_YTDLP_JS_RUNTIME=node:/usr/bin/node enables the same runtime that successfully downloaded the previously failing track. The setting is passed to browser-stream yt-dlp and mpv's yt-dlp hook. No new JS runtime or remote EJS download is installed/enabled by this script.

Lingering keeps afine's user manager alive after logout and starts it at boot. PipeWire and WirePlumber still need correctly enabled user units; enabling linger alone does not guarantee that audio services or the JBL connection work.

## Verify SSH independence and boot behavior

```bash
systemctl is-enabled aysha-music
systemctl is-active aysha-music
loginctl show-user afine -p Linger
curl --fail http://127.0.0.1:5050/api/status
sudo journalctl -u aysha-music -n 80 --no-pager
```

Close SSH, reconnect, and repeat the checks. Later, when a reboot is convenient, verify them again after reboot. Do not claim boot recovery has been tested based only on `enable --now`.

The installed service is managed with:

```bash
sudo systemctl restart aysha-music
sudo systemctl stop aysha-music
sudo systemctl start aysha-music
```

Do not also launch a foreground copy on port 5050. Updates must install the fresh binary, not just rebuild the repository copy. Timestamped service and binary backups allow manual rollback after inspecting the actual files.

## Controls and icon regression scope

- Next consumes one queued track; if empty, it requests related tracks. Lookup failure leaves current playback alone and displays an error.
- Previous walks backward through session history and puts the displaced current track at the front of the queue, allowing Next to return to it. No previous entry does not stop the player.
- Browser navigation consumes the track/mode returned by Rust without posting a second play notification.
- Browser `ended` triggers one guarded Next action rather than repeated polling requests.
- Related-track prefetch fills the queue without interrupting the current song. Bluetooth natural-end advancement remains a separate pending hardware/integration gate.
- The absolute-volume endpoint used by the slider now exists, as does the liked-library endpoint; unlike sends a full track as Rust expects.
- Local SVG artwork lives in src/icons.svg and is embedded in the binary. No CDN or icon-font dependency is needed. See skills/icons/SKILL.md for semantic inventory and acceptance rules.

## Bluetooth remains disconnected

The last direct connection failed with `org.bluez.Error.Failed br-connection-page-timeout`. This is a failed Bluetooth connection attempt before a usable Bluetooth audio sink exists; it is not evidence of a broken SVG, Rust build or stopped PipeWire server. The exact reason (speaker availability, range, another connection or adapter/link behavior) is not established. Do not erase existing pairing records without further evidence.

The service is not a solution to that radio-level failure. Keep the JBL on nearby, ensure it is available to connect, and retest bluetoothctl / the app's Connect JBL action. No unattended pairing reset is performed.

## Resource and security limits

The server remains bound to 0.0.0.0:5050 with its existing permissive API boundary. This is not an authenticated internet-facing service; network exposure must be approved and restricted separately. The user has replaced the earlier 20 MB target with a 100 MB concurrent-device maximum. No new cgroup ceiling has been applied: limiting the existing process tree without reducing/admitting work could kill playback. Confirm device count/accounting scope and follow MEMORY_BUDGET_PLAN.md before claiming the target is met. Measure the aggregate service cgroup as well as Rust and child processes separately during actual playback. Restart loses in-memory queue/history/likes; persistence has not been promised or implemented in this update.


## One-command paste-ready health report

Run as **afine**, not root, on the Linux host:

```bash
cd /home/afine/ayshamusic
python3 scripts/check_health.py
```

The report is automatically printed to the terminal and saved to a unique
`diagnostics/health-*.txt` file (owner-only mode 0600; directory ignored by Git).
Review it, then copy from BEGIN AYSHA DEBUG REPORT through END AYSHA DEBUG REPORT
into the conversation. Nothing is automatically uploaded or copied to a clipboard.
The default probes are read-only apart from writing the report; no audio playback,
service restart, pairing, credential inspection or external HTTP request is performed.
All HTTP probes use fixed `127.0.0.1:5050`, ignore proxy variables, reject redirects
and use timeouts. Commands disable paging and have timeouts. Raw subprocess logs,
track metadata and addresses are not included in the report.

For source regression checks as well:

```bash
python3 scripts/check_health.py --code-checks
```

This adds Python, Node and Rust tests (five-minute timeout per suite). Tests may
write caches/build artifacts and fetch Cargo dependencies; avoid running these
while measuring playback memory. It does not install/restart the production app.
Use `--expect-bluetooth` only if testing host-speaker playback; otherwise a missing
Bluetooth sink is a warning rather than a failure. Node and yt-dlp version probes
use this documented host's confirmed executable paths.

Exit/`RETURN_CODE`: **0** checked probes passed, **1** failure found, **2** warnings
or incomplete checks, **3** reporter error (e.g. cannot save). CLI usage errors
also exit 2 but show usage instead of a diagnostic report. To see the shell exit
status, run `echo $?` immediately after the checker. Reports deliberately include
an incomplete loaded-revision warning: file hashes do not prove the loaded process
identity, so warning exit 2 can be expected even for a healthy service.

The report uses stable check IDs. `MEMORYPEAK` above 104,857,600 bytes fails either
100 MB interpretation; 100,000,001 through 104,857,600 bytes warns that the exact
threshold needs agreement. These are observed service-cgroup readings, not an
aggregate concurrent-load certificate. A historical peak does not identify the
responsible operation. Lower current usage is not proof of compliance.

For a failed code suite, rerun that suite directly for detailed diagnostics and
review its output before sharing. No general journal or raw traceback is copied
by the health reporter. Audible playback, stream extraction/decoding, autoplay,
real-device behavior, reboot, isolation and remote authentication still require
separate acceptance tests. The checker cannot detect every possible defect.

### Attribute a high memory reading before optimizing

With one ordinary song playing, run:

```bash
python3 scripts/check_health.py --memory-samples 30
```

This records 30 snapshots spaced two seconds apart (about one minute, plus initial
probes). Change song once during the sampling window and report the approximate
time and number of listening devices. Do not run builds/tests simultaneously.
Cannot be combined with `--code-checks`. It does not start music or reset counters.

Each snapshot includes service-cgroup-v2 current/peak/swap and memory.stat
anonymous/file/kernel values in bytes, plus allowlisted process counts/RSS
(including nested service cgroups). Missing/unreadable counters or process races
are warnings. Process enumeration is capped at 64 cgroups/256 processes and flags
incomplete observations. Unknown process names are grouped as `other`; no PID,
arguments, song URLs, Bluetooth addresses or full cgroup paths are printed.

Cgroup totals and process RSS are **different accounting views**. Do not add them
together or assume RSS proves a total peak; shared pages can be double counted.
Helpers outside the service cgroup are excluded, and sampled values may miss
short spikes. `memory.peak` remains historical, not the peak of just this window.
File/page cache remains part of the reported budget, not an exemption. These
observations can narrow the cause but do not automatically identify it or prove
multi-device memory compliance. No memory limits or service settings are changed.
