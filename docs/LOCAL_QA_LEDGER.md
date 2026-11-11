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

## SSH-independent service, controls and icon regression pass

New target evidence: explicit Node runtime test downloaded 10 KiB of previously failing Jm7Satp7Zqk successfully. JBL direct connection failed with org.bluez.Error.Failed br-connection-page-timeout; remains paired/trusted but disconnected, with no Bluetooth sink. This does not establish a need to reset pairing or reinstall PipeWire.

User asks for SSH-independent operation, working next/previous and better SVG icons with a dedicated icon skill.

Root causes found in source:
- Browser navigation called server Next/Previous and then called playSong with another play notification, duplicating history.
- Previous popped history then play_handler pushed the displaced current song back into history, causing back-and-forth behavior instead of walking backward.
- Empty navigation stopped playback. Browser autoplay repeatedly posted Next from a polling loop without assigning the next audio source.
- Related-track watcher immediately started another song instead of only prefetching; browser mode was excluded from prefetch.
- Volume slider called an absent /api/volume route; library called an absent /api/liked route; unlike payload did not match the Rust Track schema.
- Previous/next and mute icons had confusing or malformed path geometry and duplicated state-specific markup.

Changes:
- Added pure queue/history transitions in src/playback.rs with four tests; navigation returns track/mode and preserves playback on empty history/lookup failure.
- Browser navigation is single-flight, reuses returned selection without duplicate server notification, and ended event advances once. Related prefetch is asynchronous/time-bounded and no longer interrupts current playback.
- Added /api/volume and /api/liked, fixed unlike metadata, visible heart state and category event handling.
- Added 16 original local SVG symbols in src/icons.svg, embedded route, shared rendering, native navigation buttons, labels and focus styles. skills/icons/SKILL.md defines the dedicated semantic/accessibility procedure. This is a repository skill, not a platform feature registration.
- Added deploy/aysha-music.service and scripts/install_local.sh for the verified afine machine: test/build first, backup, install to the confirmed ExecStart, supply audio/PATH/Node environment, enable linger and boot service. The installer refuses a foreground copy or unreviewed drop-ins. It is not executed in the sandbox or claimed tested on the user's host.
- MUSIC_YTDLP_JS_RUNTIME is an optional administrator environment setting, forwarded to stream yt-dlp and mpv's downloader hook. The unit selects the user's successfully tested node:/usr/bin/node without installing runtimes or changing global yt-dlp config.
- Added four Rust handler tests, six additional frontend tests and three icon-integrity tests. Updated Linux CI script syntax check.

Initial local test-tool failure: two frontend tests reported TypeError: document.getElementById(...).setAttribute is not a function because the minimal DOM mock lacked the new native accessibility API. Extended the mock with attributes/listeners; all 12 frontend and 14 Python tests subsequently pass. Shell syntax and diff checks pass. New Rust tests/build are verified by CI separately before reporting success.

Remaining gates: actual updated service installation, SSH logout/reboot survival, audible browser playback, JBL radio connection, Bluetooth natural-end autoplay, concurrent clients, browser visual/screen-reader validation, persistence decision, network boundary, and memory budget/measurement. Closing SSH client is supported by systemd; shutting down the Linux server itself cannot preserve playback.

### Verification of controls/service/icon source commit 1420ffa

Linux CI run 43073462317 passed: Rust regression tests, release build, all Python tests (14 locally), embedded frontend tests (12 locally) and Linux script syntax. Source commit: 1420ffaf07be4113a22cc3e72808c3a2d9e90c6b.
https://github.com/AfzalAshraf/ayshamusic/actions/runs/43073462317

The host-specific installer has not been executed against the user's machine by the agent, and CI does not exercise systemd installation, reboot/logout, speaker hardware or visual browser rendering. README now directs local users to the installer and correctly identifies src/index.html as the Rust-served frontend. No pending source compile failure remains from this pass.

## Remote access and simultaneous devices: architecture department

Active skill: Product & Architecture. New department: Networking & Self-Hosting, provisional 1/5 until deployment and off-LAN security/playback tests pass. User now reports "it plays" and identifies Android Chromium as the primary browser. Audible playback is reported; output destination, actual Android build, SSH logout/reboot and multiple-device acceptance are not yet established.

User requests: custom-domain outside-home listening using Cloudflare Tunnel, simultaneous devices without interference, a skill/rating for the new department and recommendations.

Source audit: all requests share one AppStateInner.player, global mpv stop pattern and a frontend that auto-selects the home speaker if connected. This explains a concrete cross-device interference risk but not every unspecified symptom. /api/stream starts downloader processes without a configured per-device/global admission cap. Origin is unauthenticated with permissive CORS and 0.0.0.0:5050 binding. Do not publish it as-is for multi-user use.

Created skills/networking-self-hosting/SKILL.md and docs/REMOTE_MULTI_DEVICE_PLAN.md. They specify private Access-before-route setup, current provider media-term verification, per-device sessions and owned streams, separate administrator-owned Bluetooth output, resource caps, Android acceptance tests, rollout/rollback and department ratings. No application code, packages, firewall, DNS, installed services or tunnel credentials were changed.

Current provisional ratings for the expanded local-plus-remote scope: Product & Architecture 2/5, UI/UX Design & Frontend 3/5, Backend & Database 2/5, QA/Security 3/5, Networking & Self-Hosting 1/5. Exact acceptance gates are in the plan. These supersede only the current readiness assessment, not historical audit evidence.

Research: official Cloudflare Tunnel overview, self-hosted public Access application guidance, remote tunnel API/install guidance, and current CDN service-specific terms were checked. Access must protect UI/API/audio before the route is public. CDN audio restrictions are a suitability concern, not grounds to assert that all Tunnel uses are prohibited; confirm the intended product/plan before sustained audio relay. No unlimited-free-audio promise is made.

Questions blocking implementation: independent playback vs synchronized room, actual domain and permitted audience, concurrency and RAM-budget scope, remote Bluetooth privileges and persistence. Await design approval before changing data ownership or deploying the route. Verification this turn is source inspection and git diff --check only; no remote account configuration or new live multi-device tests occurred.

## Android portrait and back navigation frontend pass

Active department: UI/UX Design & Frontend. User requests a premium music-app-like mobile experience, working Back/portrait layout, plus Cloudflare installation and nightly updates. Networking work is deferred to its own department because the domain, access policy, maintenance time/timezone and update scope remain unknown. No remote shell is available to install services on the user's host.

Source findings: the prior mobile sidebar was positioned at the bottom but its nav-group still used a vertical column, hiding/clipping controls. Desktop-style player sizing and rigid viewport dimensions did not provide a coherent portrait layout. Tab switching was DOM-only with no history entries, so browser Back could leave instead of returning inside the app. Search initialization could race a default genre request. The page advertised a manifest/worker not served by the Rust router.

Frontend changes:
- Rebuilt the served src/index.html layout and styles with original Aysha design: dynamic viewport/safe areas, compact hero and song rows, four labelled bottom tabs, 44px primary targets, native search, zoom/reduced-motion support, clear loading/empty/error states and a mini-player.
- Added a full-screen native Now Playing dialog reusing the existing audio element and playback API, with synchronized title/art/output/progress/volume and accessible controls.
- Added tab/overlay history with Back/Forward, header back button, modal cancel handling and per-tab scroll restoration. Search/genre responses are generation-guarded.
- Added arrow-left and chevron-down to the embedded SVG set (18 total). Removed unserved manifest/worker registration rather than claiming offline/PWA behavior. Updated frontend unit harness initialization and icon count checks.
- Added skills/mobile-frontend/SKILL.md, docs/MOBILE_UI.md and tests/mobile.browser.cjs. Browser dependencies are test-only, outside the repo. Added a CI browser-layout job. Screenshots are sample-data test previews, excluded from Git.

Verification: 12 frontend regressions and 14 Python tests pass. Chromium fixture tests passed at seven viewports: 320x740, 360x800, 390x844, 412x915, 768x1024, 844x390 and 1440x900. Checks cover overflow, nav target size, search/genre, browser/app Back, Forward and modal close/restore, with no uncaught page errors. Home/full-player screenshots were inspected; a gray default range track was replaced with an explicitly styled volume slider.

Test infrastructure issues encountered (not target-player failures): sandbox headless Chromium initially lacked libnspr4/libnss3; supplied package-bundled shared libraries outside Git. Repeated isolated-context launches produced browser termination/SIGSEGV with the serverless binary; changed the test harness to reuse one browser context/page across viewport checks. Subsequent complete browser run passed. No runtime/hardware audio claims are derived from fixtures.

Current ratings: Product/Architecture 2/5; UI/UX 4/5 pending actual Android/TalkBack/background/audio checks; Backend 2/5 pending per-device isolation; QA/Security 3/5; Networking/Self-Hosting 1/5 pending Access/tunnel/maintenance decisions. Shared-state multi-device interference, Bluetooth connection timeout, provider media suitability and total RAM remain unresolved. No automatic updates were enabled and no public route was published.
