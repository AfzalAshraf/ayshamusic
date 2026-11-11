# Local Linux QA ledger

Scope: Aysha's Music Player, Rust music-server, port 5050, systemd, mpv/Bluetooth, and browser streaming fallback. Cloud services and Python fallback are outside this improvement pass.

## Department workflow

Use one department per turn, declaring it explicitly. Product & Architecture defines and obtains approval for the plan before the next department starts. Active failures may be handled by QA first. Append a build manifest and error/risk ledger to every turn. Ask about consequential unknowns rather than guessing. Do not upgrade services or dependencies without a demonstrated need and verification.

Ratings below are provisional evidence-based readiness scores, not guarantees. A department reaches 5/5 only when its acceptance checks pass on the target Linux machine. Do not inflate scores to meet a target.

| Department | Initial rating | Required evidence for 5/5 |
| --- | --- | --- |
| Product & Architecture | 2/5 | Approved local-only flows, network access model, multi-client policy, storage and memory-budget scope, installation and recovery design. |
| UI/UX Design & Frontend | 2/5 | Verified browser and Bluetooth flows, responsive and keyboard-accessible controls, loading/error states, next/previous/queue behavior and browser compatibility. |
| Backend & Database | 2/5 | Validated inputs, explicit playback ownership, correct stream metadata and lifecycle, reliable queue/autoplay, approved persistence behavior, repeatable systemd installation. |
| QA, Security & Debugging | 1/5 | Reproducible clean build/tests, security regressions, local end-to-end playback, disconnect/reconnect tests, reboot recovery and measured memory under load. |

## Verified evidence

- User's machine built release commit 5281fc4 successfully, with unused_mut and dead_code warnings.
- User's curl response returned HTTP 200 and binary bytes containing a WebM header. This verifies stream transport, not browser decoding or end-to-end playback.
- curl error (23) after piping to head is consistent with head closing the output pipe. It is not by itself evidence of a server failure.
- Rust serves src/index.html via include_str!. The separate src/www/index.html is not the root page served by that handler.

## Current QA changes

- Removed unnecessary mut from YtDlpStream::poll_next.
- Removed unused get_stream_url helper; source inspection found no callers.
- No dependencies, endpoints, installed services, cloud files or playback behavior changed.
- git diff --check and source-level assertions are used for this cleanup. Cargo is unavailable in the agent workspace; a fresh compile and runtime validation remain required on the target machine.

## Open risk ledger

| Finding | Location / evidence | Next verification or remediation |
| --- | --- | --- |
| Incorrect media type | api_stream sends audio/mpeg while user captured WebM bytes | Agree browser targets; select a known container and advertise matching MIME type. Do not claim decoding is verified. |
| Unsupported range claim | api_stream sends Accept-Ranges: bytes but does not implement Range/206 | Remove claim or implement and test actual seek behavior. |
| Downloader lifecycle | api_stream drops Child handle; response retains stdout only | Retain ownership, terminate/reap on disconnect and test repeated skips for orphan processes. |
| Hidden upstream failures | yt-dlp stderr is discarded; HTTP 200 is sent before successful download | Add bounded diagnostics and startup/idle timeout with useful failure reporting. |
| Network/security boundary undefined | Binds 0.0.0.0:5050, permissive CORS, no authentication, raw stream URL passed to yt-dlp | Confirm localhost/LAN/remote access requirements; validate allowed URLs and enforce the approved boundary. Local Linux does not imply localhost-only. |
| TLS checks disabled | api_stream uses --no-check-certificate | Restore verification and diagnose actual certificate failures rather than bypassing validation. |
| Resource budget unverified | Prior idle measurement excludes playback evidence; multiple yt-dlp processes possible | Clarify whether 20 MB includes child processes, then measure idle, streaming, Bluetooth and repeated skips. |
| Restart persistence undefined | queue, likes and history are held in PlayerState vectors | Ask whether persistence across service restart is required. |
| Production unit unknown | Actual installed systemd unit is on user's machine | Inspect systemctl cat aysha-music before prescribing installation paths or hardening. |
| End-to-end playback unverified | curl transport only | Verify selected browser, Bluetooth output, fallback, queue transitions and recovery. |

## Questions for Product & Architecture

1. Is use restricted to the Linux host, trusted LAN devices, or remote devices too?
2. Does the 20 MB RAM ceiling cover music-server alone, or its yt-dlp/mpv child processes as well?
3. Which browser/device must browser fallback support?
4. Must queue, likes and history survive restarts, and may multiple clients control playback?

No next-department implementation should begin until the user approves the architecture step and resolves its consequential ambiguities.

## Runnable test baseline

- Added scripts/test_local.py: read-only UI/status/queue smoke checks and optional bounded real-stream container/header validation, with nonzero failure exits.
- Added tests/test_local_smoke.py: 11 offline tests for the checker, including the observed WebM/MPEG mismatch, empty stream, unverified ranges and a local HTTP fixture.
- Added docs/LOCAL_TESTING.md: exact Linux commands, service inspection, resource snapshots and manual acceptance matrix.
- Verified: all 11 checker tests pass; CLI help and git diff --check pass. These are test-tool results, not a Rust application or hardware verification.
- Current stream metadata remains a known failing application check; it is deliberately not hidden by the test suite.
- No department is promoted to 5/5 from test-tool fixture results. Architecture access scope, total memory budget, browser targets, persistence and multi-client requirements remain unanswered.

## Browser playback and Bluetooth failure remediation

Evidence supplied by user: release build succeeds, Rust reports zero tests, all 11 Python checker tests pass, live UI/status/queue pass, live audio fails MIME mismatch (audio/mpeg versus WebM).

Additional source findings:
- The served src/index.html still parsed the binary stream as JSON and expected stream_url.
- Frontend posted play metadata as JSON while play_handler extracts Query<PlayQuery>, yielding missing URL errors.
- Browser play notifications still launched server mpv. The autoplay watcher could also launch mpv while browser audio was selected.
- Card action handlers inserted unescaped JSON into double-quoted HTML attributes.
- Disconnect called connect, Scan Speakers was not a scan, connection diagnostics were discarded, and a background reconnect loop could undo a user disconnect.
- Audio routing forced UID 1000 regardless of the service user.

Changes:
- src/media.rs owns stream startup and process lifetime, detects media headers, preserves header bytes, advertises no seeking, enables TLS verification, logs downloader failures, and imposes a startup timeout. Downloader input is restricted to HTTPS YouTube hosts. This is not a complete SSRF/authentication boundary; redirects and configured downloader behavior still require review.
- The embedded page directly assigns the proxy URL, sends play query parameters with explicit browser/bt mode, reports request/media failures, and binds card buttons with closures.
- Browser mode tracks state without spawning mpv. The server watcher no longer starts mpv for browser mode.
- Bluetooth connection verifies actual bluetoothctl state after the configured script, exposes errors and timeout, and disconnect has a dedicated endpoint. Background reconnect was removed so explicit disconnect stays disconnected.
- Existing JBL MAC remains unchanged, pending user confirmation. MUSIC_BT_CONNECT_SCRIPT can override the existing /home/afine/connect_jbl.sh path.
- Audio commands honor service-provided XDG_RUNTIME_DIR/PULSE_SERVER; otherwise derive the runtime directory from the process user's id rather than UID 1000. mpv startup failures are returned, and mpv children are reaped after exit.
- Added seven Rust media tests and six embedded-frontend tests. Added read-only scripts/diagnose_local.sh and Linux CI build/test workflow; no deployment workflow is added.

Verification: six frontend tests and eleven Python tests pass locally. Rust is unavailable in the agent workspace; CI/target-machine verification is recorded separately when available.

Still open: actual Bluetooth adapter/pairing/permissions and audio server status, installed service path, browser decoding/device compatibility, browser ended-to-next behavior, Bluetooth track-end detection (watcher still has pre-existing eager advancement), process concurrency/idle timeouts, persistence and memory budget. Hardware playback is not claimed fixed or rated 5/5 based on these code changes.

### Linux build verification result

GitHub Actions run 43069050583 for source commit 6088c84 completed successfully:
https://github.com/AfzalAshraf/ayshamusic/actions/runs/43069050583

All four job steps passed: cargo test, cargo build --release, Python checker tests, and embedded frontend regression tests. This closes the unverified-compilation caveat for that source commit. Full log download was blocked by this workspace's outbound-host restriction, but run/job success was confirmed by gh run watch --exit-status. The workflow reported a nonblocking actions/checkout v4 Node runtime deprecation notice.

This does not verify the user's Bluetooth hardware, service environment, YouTube availability or browser decoding. Live retest must use the freshly built binary, not just the previously installed systemd binary. Department scores remain provisional pending those acceptance gates.

## Target-machine stream success; extractor/runtime warning

User verified seven Rust tests and the release build pass on local Linux. Live UI, status, queue, and audio transport/metadata checks all pass. Sample is audio/webm, 4096 bytes. Browser decoding is still not confirmed by these checks.

New reported errors:
- yt-dlp: "No supported JavaScript runtime could be found. Only deno is enabled by default".
- Track Jm7Satp7Zqk: "unable to download video data: HTTP Error 403: Forbidden"; media::response_from_child reports "early eof".
- Other reported tracks reach download completion. Completion means download finished, not that the browser played the audio.
- Diagnostic output stops at systemctl show; no adapter, pairing or sink evidence was supplied. Cannot infer Bluetooth root cause from uid=1000 or group membership.

Research: https://github.com/yt-dlp/yt-dlp/wiki/EJS (both setup sections read). Official guide requires a supported JS runtime plus EJS scripts. Deno >=2.3 is recommended and enabled by default; Node >=22 must be explicitly enabled with --js-runtimes node. Script installation depends on yt-dlp distribution. An absent enabled runtime is a plausible extraction problem, not proof that it caused this particular 403.

Changes: diagnostics now print downloader/runtime paths and versions, OS/architecture, disable systemctl paging, and force-kill stuck probes two seconds after their 15-second timeout. No runtime is installed, no downloader configuration changed, and no automatic remote script downloads are enabled.

Next action: get full diagnostic output, confirm whether sound is actually audible and which browser is used, then choose installation/configuration steps matching the existing yt-dlp installation and approved memory budget. Do not prescribe sudo pip, replace system packages or install a JS runtime blindly. Department ratings remain provisional.

## Full target diagnostics: stopped player, installed Node, disconnected JBL

Evidence supplied by user:
- Ubuntu 26.04 x86_64, afine uid=1000.
- yt-dlp 2026.08.19 at /home/afine/.local/bin/yt-dlp; Node v22.22.1 at /usr/bin/node; Deno/QuickJS not on the shell PATH.
- aysha-music MainPID=0; no music-server, yt-dlp or mpv processes were found at diagnostic time. ps exits nonzero when no processes match; that output does not establish a timeout.
- Confirmed installed ExecStart: /home/afine/music-server/target/release/music-server; WorkingDirectory=/home/afine/music-server; User=afine.
- Bluetooth service active, controller powered. JBL 04:CB:88:92:43:60 paired/bonded/trusted, not blocked, Connected: no.
- PulseAudio compatibility server is PipeWire 1.6.2 at /run/user/1000/pulse/native. Only the internal ALSA card/sink appears. User-shell access works; systemd environment is not yet verified.

Conclusions/actions:
- The player must be started for UI/API playback; successful compilation does not imply a running server.
- Test the already-installed Node explicitly with yt-dlp --js-runtimes node:/usr/bin/node against the previously failing track before any runtime installation or package upgrade.
- Direct bluetoothctl connect is the next diagnostic, followed by info and sink enumeration. The reason the speaker is disconnected is still unknown. Pairable: no is not itself a reason to re-pair an already bonded speaker.
- No automatic pairing reset, trust changes, service reinstall or runtime installation is justified by these logs.
- Persistent yt-dlp settings and systemd PATH must be checked separately after a successful direct test: the user's ~/.local/bin downloader is not guaranteed to be visible in the service environment.

Only documentation changed in this turn. Latest known application build/tests remain successful; browser audibility, a successful JBL connection and service recovery are still pending. All departmental scores remain provisional below 5/5.
