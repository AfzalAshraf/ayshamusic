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

### Mobile UI CI result

Source commit 4e19638 passed Linux CI run 43078897480:
https://github.com/AfzalAshraf/ayshamusic/actions/runs/43078897480

Both jobs passed: Rust tests/release build, 14 Python tests, 12 frontend tests, Linux shell syntax, and real Chromium fixture layout/history checks at all seven viewports. The isolated CI browser job completed successfully without the sandbox-only shared-library workaround. This verifies the new embedded frontend builds with Rust; it does not verify real Android audio, two-device isolation or a deployed tunnel.

## User-approved foundation and main upgrade skill

Active department: Product & Architecture. User explicitly approves the latest mobile design ("Fantastic and the best i love it") and asks for a main skill making this the foundation for future upgrades. User also explicitly requests the visible dedication with the real love emoji.

Change contract: preserve the approved Android layout/controls and local Rust path; add upgrade governance/discovery, correct the requested branding only, and verify it without redesign or infrastructure changes. Recovery is a selective revert of these files, not a reset of unrelated later work.

Created:
- AGENTS.md: project entry point linking the main and specialist skills.
- skills/main-upgrade/SKILL.md: baseline-preserving workflow, department routing, approval/ambiguity rules, priorities, verification ladder, ownership/resource/security constraints, host/maintenance boundaries, evidence-based scoring and mandatory logs.
- docs/UPGRADE_BASELINE.md: approved source/verification anchors (4e19638 / 307db41), preservation requirements, known gaps and current readiness gates. These anchors are references, not instructions to reset future work.
- tests/test_upgrade_baseline.py: exact dedication/emoji and skill-discovery checks.

Updated README links and both visible src/index.html footers to: Made with ❤️ for Aysha — with love, from Afzal. The heart uses U+2764 U+FE0F with an accessible love label. Expanded footer text wraps while preserving the Stop control. Browser tests assert the exact text in both locations and no expanded-player horizontal overflow.

Verification: all 16 Python tests and 12 frontend tests pass locally. Real Chromium fixture tests pass all seven portrait/tablet/landscape/desktop viewports with the new footer checks. CI build result is recorded separately when complete. No new runtime dependencies, APIs, installed services, domain/tunnel, timer or scheduling changes. No actual target Android/hardware tests were performed here.

Current app scores remain Architecture 2/5, UI/UX 4/5, Backend 2/5, QA/Security 3/5 and Networking/Self-Hosting 1/5. Approval establishes the design baseline, not completion of unresolved device isolation, authentication, Bluetooth connection, remote-access, persistence, memory or maintenance gates. The main skill is a repository procedure, not a platform skill registration or unlimited authorization to upgrade services.

### Main skill / dedication verification result

CI run 43080374193 for source commit 86e9d30 passed both jobs: Rust tests and release build, Python/frontend regressions, shell syntax, and all seven Chromium layout/history/footer checks.
https://github.com/AfzalAshraf/ayshamusic/actions/runs/43080374193

Both visible footers contain the exact requested heart-emoji sentence; the full-player footer wraps without horizontal overflow in the browser suite. No host-side deployment is claimed by this CI result.

## Soundtrack Stop -> Play regression and separate comfort-space proposal

Active department: QA, Security & Debugging. User reports the YOUR DAILY SOUNDTRACK card does not play after Stop and asks for a separate supportive-quote experience with personalization.

Reproduced failure: new frontend tests for browser Stop/Play, repeated Stop and Bluetooth Stop/Play failed with AssertionError, actual 1 versus expected 2 play registrations/attempts in tests/frontend.test.cjs. Root cause in src/index.html stopPlay/togglePlay: Stop cleared current_track/playMode and removed the audio source (or killed mpv), while Play required a current selection and only knew the resume path.

Fix: retain the selected track/output on this page, add an explicit stopped lifecycle flag, release browser media via removeAttribute('src') and load(), and restart a stopped selection with playSong plus /api/play. Pause remains resume-only. Failed Bluetooth Stop does not falsely mark the server stopped; a failed browser server notification still leaves local audio stopped with a visible diagnostic. Failed restart keeps the Play control and error available. The selected track is not persisted across reloads, and global backend/device-state limitations are unchanged.

Verification: the three originally failing cases now pass; 19 frontend tests pass including pause/resume, initial empty selection and failure cases. All 16 Python tests pass. The Chromium suite passes all seven viewports and exercises actual soundtrack/expanded Stop/Play buttons at 390px with media methods explicitly stubbed. An initial browser assertion sampled src before asynchronous play registration completed; it was corrected to await the source assignment, then passed. These tests prove UI lifecycle/wiring, not actual audio/Bluetooth output. Linux CI result is recorded separately when available.

Quotes are deliberately separate from the bug fix: docs/COMFORT_SPACE_PROPOSAL.md proposes an optional, hideable space with original gentle messages and transparent on-device personalization using explicit likes/Show more actions only. No quotes, preference collection, dwell tracking, mood inference, remote analytics or relationship scoring were added to the app. Ask for approval of the opt-in approach before the next department implements it. No claim is made that messages cure anxiety, guarantee safety or make someone love another person; the design emphasizes choice and kindness.

No dependencies, endpoints, service schedules or host installations changed. Current scores remain Architecture 2/5, UI/UX 4/5, Backend 2/5, QA/Security 3/5, Networking/Self-Hosting 1/5. Preserve the approved design and dedication. Remaining work includes real Android restart verification, global-state isolation, hardware connection, remote-access/security/resource gates and approval for the separate comfort space.

CI confirmation for 8e846d2: https://github.com/AfzalAshraf/ayshamusic/actions/runs/43081818535 completed successfully. Both jobs passed: Rust regression tests/release build, Python checks, 19 embedded frontend tests, Linux syntax and seven-viewport Chromium checks. Non-blocking runner annotations flag checkout@v4's Node 20 migration and ubuntu-latest image migration; no action-version changes are bundled into this playback fix.

## Follow-up: empty Now Playing after Stop, and unrecoverable refresh

Active department: QA, Security & Debugging. The user still reports "Choose a song from Home or Explore" after Stop/Play and says refresh stops playback and prevents restart. This exact empty-player text comes from syncNowPlaying when current_track is null. On current source, same-page Stop/Play retains selection and passes the prior regression. Whether the user's running host binary contains that previous fix is unknown: no agent-side host installation occurred. Old deployment is a hypothesis, not a demonstrated cause of the same-page repeat report.

Demonstrated missing coverage: reload creates a new state object with current_track null; the prior fix deliberately did not persist selection, and Rust Stop clears server current_track. Global status could also set UI playing without a local audio source after refresh. Added refresh/polling regressions initially yielded 20/23 Node passing with three failures (missing restoration and no directly testable pollStatus function).

Scoped recovery fix: store only last selected ID/title/artist/URL and output in tab-scoped sessionStorage (aysha.selection.v1), validate bounded fields on restore, regenerate thumbnail, render metadata synchronously and offer Play with no autoplay. Existing pause/resume stays separate from restart. Handle inaccessible/malformed storage without breaking normal playback. Extract pollStatus and stop it from overwriting a local selection or asserting browser playback solely from shared server state. This is metadata restoration, not seamless audio across reload, playback-position persistence, a permanent library or backend device isolation. Server Bluetooth may continue across refresh; restored Play conservatively restarts rather than assuming mpv exists.

Chromium's first real reload test found an additional exception while constructing MediaMetadata: artwork src was undefined after restoring the minimal track. The browser timed out waiting for a play attempt and captured that exact page error. Fixed MediaSession artwork to use the validated track ID, and regenerated thumbnail on restored tracks for subsequent API/like/queue use. Removed temporary diagnostics and reran successfully.

Verification: 26 Node frontend tests pass (refresh after Stop, refresh while server still says playing, status mismatch, MediaSession artwork, Bluetooth restart, invalid/corrupt/denied storage plus prior cases); 16 Python tests and shell syntax/diff checks pass. Chromium passes all seven viewports, with actual Home-card/hero/expanded clicks and real reloads at 390px, zero autoplay attempts after load and successful restarted play attempts. Media output is stubbed, not live audible playback. Linux CI is recorded below once completed.

Deployment: docs/LOCAL_SERVICE.md now explains the embedded UI boundary, existing safe installer and a curl check for the served selectionKey declaration. User must load the updated build and select a song once before expecting restored metadata; old code never saved the selection. Host deployment, browser address/cache state and actual Android/Bluetooth audio remain unverified. No runtime dependency, Rust endpoint, service scheduling, quote feature or analytics changed. Preserve unrelated tracked pyc deletion. Scores remain Architecture 2/5, UI 4/5, Backend 2/5, QA/Security 3/5, Networking 1/5 pending target-device and security/resource gates.

CI confirmation for refresh source 7258412: https://github.com/AfzalAshraf/ayshamusic/actions/runs/43082963236 completed successfully. Both jobs passed: Rust regression tests and release build, 16 Python tests, 26 frontend tests, Linux shell syntax and seven Chromium viewports including real document reloads. Existing non-blocking checkout Node-runtime/image migration annotations remain. This does not verify which executable is running on the user's host.

## User-host installation evidence; new iPhone playback report

Active department: QA, Security & Debugging. User supplied installer output showing fast-forward from 8efbd91 to 71bfaa1, all 15 host Rust tests passing, release build succeeding, previous service unit backed up, and the status endpoint returning Ready. The subsequent local root-page curl printed `const selectionKey = 'aysha.selection.v1';`. This is user-provided evidence that the host endpoint serves the refresh-recovery build; an old host binary is no longer the leading explanation for that endpoint. The initial curl connection refusal was recovered by the installer's configured connection-refused retry and followed by valid JSON. No reinstall is required from this output alone. Logout/reboot survival and phone audio are not proven by a responding host endpoint.

User additionally reports iPhone browser cannot play songs, without iOS/browser version, page-load result, output destination or displayed error. Inspected src/media.rs: yt-dlp uses unconstrained `-f bestaudio`; the proxy detects MIME from returned bytes, streams sequentially and explicitly advertises Accept-Ranges: none (no Range/206 implementation or known content length). Inspected src/index.html: initial play waits for Bluetooth discovery and play registration before calling audio.play; a connected home speaker automatically selects server Bluetooth output. These are demonstrated source behaviors, not a confirmed cause of the iPhone report. Version-dependent container/codec support, Safari media-loading/range behavior, gesture restrictions, intentional routing to the home speaker, and reachability (if page does not load) need separating. Do not claim all iPhones reject WebM or that switching MIME labels fixes the actual stream.

Next evidence: whether UI loads, iOS/browser version and exact playback message, Now Playing output label, and recent service journal immediately after one failed attempt. A second explicit Play tap can distinguish a gesture restriction from a persistent stream issue, but is not proof of a full fix. No transcoder, disk cache, dependency or streaming architecture change introduced before diagnosis/resource review. No fresh audio or iPhone hardware test performed. Scores unchanged; iPhone compatibility remains unverified. Unrelated tracked pyc deletion preserved.

### iPhone follow-up: successful downloader, playback returns to Play

User now confirms the page opens on iPhone and Play changes to Pause then back to Play after about 2–3 seconds. Supplied host journal shows two completed yt-dlp downloads of Jm7Satp7Zqk, each selecting format 251, and another extraction starting. Format 251 is normally WebM/Opus; this narrows investigation toward browser playback/stream delivery rather than a demonstrated downloader extraction failure. A completed downloader log still does not establish complete HTTP delivery or successful decoding. No iOS version, native error code/message or direct stream-response capture is supplied. Source sets Playing optimistically before audio.play resolves and resets to Play on play rejection/audio error (also on natural end); button transition alone does not distinguish those causes. Request exact visible error or screenshot and iOS version before calling format, Range support or gesture permission the confirmed cause. No runtime fix or reinstall requested solely from these logs.

The journal also reports the previous service invocation's memory peak as 120.3M. This is service-accounting evidence, potentially including child processes, not a Rust-process RSS measurement or proof about the currently running invocation. It does not satisfy a 20 MB whole-service target; resource scope and per-process measurement remain unresolved. Do not impose a hard limit that kills music as a purported optimization.

## Updated user requirement: maximum 100 MB with simultaneous devices

Active department: QA, Security & Debugging. User explicitly replaces the prior 20 MB target with maximum 100 MB during concurrent multi-device use and requests Rust. Production server remains Rust; yt-dlp/Node/mpv are still external processes and their memory cannot be excluded merely by using a Rust API/proxy. Maximum listener count, budget boundary (especially tunnel/shared audio services), and independent versus synchronized playback are still open. No arbitrary capacity or successful cap is claimed.

Inspected code confirms per-stream downloader spawning, independently spawned search fallback/related prefetch, whole-output/JSON collection without byte budgets and global playback state/process-kill behavior. The journal's 120.3M service peak does not establish attribution or concurrency. A stream-only semaphore or MemoryMax alone is insufficient to establish working simultaneous playback below the target.

Created docs/MEMORY_BUDGET_PLAN.md with a conservative 100,000,000-byte target, recommended complete-service-cgroup accounting, bounded Rust admission and in-flight sharing, a candidate bounded preparation/fan-out path, device-isolation and iPhone Range/format gates, cancellation/metadata/cache limits, and measurable acceptance with no OOM interruptions or hidden workers. Updated current budget references in main upgrade skill, approved-baseline limitations, service and remote-device docs; earlier ledger entries remain historical. No production code, dependency, service limit, swap setting or host install changed. Obtain capacity/scope/session-mode approval before architecture implementation. Tests are not rerun for documentation-only changes; formatting checked. Scores unchanged (Architecture 2/5, UI 4/5, Backend 2/5, QA/Security 3/5, Networking 1/5).

## Source/build clarification and native Rust helper feasibility

Active department: QA, Security & Debugging. User asks where the complete updated code is, how to compile it, and whether mpv/yt-dlp can be Rust implementations. Clarified that existing source/Stop-Play/refresh work is implemented while 100 MB concurrency, isolated sessions, iPhone compatibility and native helpers are not completed. Updated README status/100 MB target and LOCAL_SERVICE compile-only versus installed-executable instructions. Extended MEMORY_BUDGET_PLAN with native audio-worker/extractor feasibility and acceptance, not a wrapper mislabeled as Rust. Rodio/Symphonia repository metadata was checked through gh api; no dependencies selected/installed. An initial native-extractor candidate lookup returned HTTP 404, so no availability/compatibility claim is made from it.

Workspace Git metadata was at the known initial ancestor while completed files remained present. Fetched the existing active remote branch and used an ancestry-guarded mixed reset to reconcile the current branch's index/HEAD without overwriting working files. Outstanding changes are the intended documentation and unrelated tracked pyc deletion; do not stage that deletion or switch branches. Published source has not been rewritten by this metadata repair.

Verification rerun: 26 frontend tests, 16 Python tests, Linux shell syntax and diff whitespace checks pass. No local cargo executable is available; no new Rust source changes made. Previously recorded Rust CI and the user's host build evidence apply to the existing implementation, not to planned native replacements or a 100 MB guarantee. Runtime/dependencies/service remain unchanged; publishing documentation does not require reinstalling the already verified host binary. Capacity/accounting/session-mode approval remains the architecture gate; ratings unchanged.

## Approved first native Rust audio-only prototype

Active department: Backend & Database. User reports another successful 15-test/release host build and explicitly approves implementing the first prototype. Scope approved here is a standalone local-file audio worker, not full extraction, production replacement, shared/independent session semantics, 100 MB enforcement or a service/tunnel change.

Implemented isolated package prototypes/rust-audio (aysha-audio-prototype), using exactly Rodio 0.21.1 and explicit Symphonia-backed WAV/PCM, AAC/MP4, MP3, FLAC and Vorbis features. It never invokes mpv/yt-dlp/Node/ffmpeg/a shell or networking. Uses a 32 KiB reader/on-demand decoder, one source, <=64 MiB trusted regular-file limit, finite 0..1 volume, 1..3600-second experiment limit (default 60 seconds, includes pauses), bounded 128-byte stdin lines/four-command queue and explicit pause/resume/volume/seek/status/stop. EOF stops interactive runs. Seek is checked against known duration and reports decoder errors. Headless decode never opens an audio device. Linux RSS/high-water RSS are labeled process-only and unavailable measurements are not reported as zero. No 100 MB guarantee or complete hostile-file isolation is claimed from these bounds.

Production Cargo manifest/src/main.rs/UI/service/installer remain unchanged. The optional package's build products are ignored. Added tests/rust_audio_smoke.sh and a separate CI job with ALSA development headers; ffmpeg is used only to generate test fixtures and is not a playback dependency. Added docs/RUST_AUDIO_PROTOTYPE.md with exact pull/build/preparation/headless/playback commands, prerequisites, supported/unsupported media, default-device routing, measurements and non-disruptive rollback. Existing yt-dlp preparation is explicitly separate and still external; WebM/Opus 251 is not supported by the prototype. Default ALSA routing still needs the user's audio stack; setting Pulse environment alone does not ensure it. Do not modify the service ExecStart or installer to use this CLI.

Verification chronology: initial source 7da70f2 compiled and passed all six prototype unit tests, but CI smoke failed on generated AAC/M4A with `Prototype error: Unrecognized format`, after successful WAV decode. The sandbox cannot reach GitHub Actions' redirected blob log/archive hosts (EOF); direct gh log download also failed. Added bounded smoke-log error annotations retrievable through api.github.com, confirmed the M4A failure, inspected Rodio's documented File/Builder path and supplied byte length, seekability and optional format hint while retaining the 32 KiB reader. This fixes file-container access; it is not evidence about the separate iPhone HTTP stream bug. CI source babe4a3 then passed all three jobs: https://github.com/AfzalAshraf/ayshamusic/actions/runs/43088120487 . Prototype: six unit tests/release, WAV/AAC-M4A/MP3 headless decoding, bad/missing/oversized file rejection and ALSA-null playback. Existing production Rust tests/release, 16 Python, 26 frontend, shell and seven-viewport Chromium checks also passed. Local Node26/Python16/shell/diff checks rerun successfully. No local cargo or physical speaker is present in this sandbox.

The resolved prototype dependency lock is retained as a CI artifact; download into the sandbox failed at the restricted blob host (EOF), so no lockfile was fabricated or claimed committed. The exact direct Rodio version is pinned in the isolated manifest. Native playback memory numbers on a real host, audible JBL sound, control/seek/reconnect behavior, decoder error recovery and comparison to mpv remain target acceptance gates. No claim of whole-service 100 MB compliance, arbitrary device count, native YouTube extraction or fixed iPhone playback. Overall scores remain Architecture 2/5, UI 4/5, Backend 2/5, QA/Security 3/5 and Networking 1/5; the isolated prototype has CI integration evidence but does not raise overall production readiness. Unrelated tracked pyc deletion preserved.

## User-requested Quotes bottom tab

Active department: UI/UX Design & Frontend. User explicitly requests an additional bottom tab for the earlier quotes experience. Implemented the separate Quotes page in src/index.html, fifth navigation column at mobile sizes and same desktop sidebar item, preserving existing four routes, approved colors/layout, safe-area mini-player and exact dedication. Reused the local heart icon; no dependencies, Rust endpoints or playback/host/service changes. The first version provides 24 original loving/gentle/rest/kindness/courage messages, category filters, Next, saved quotes/unsave and truthful empty state. A rose gradient serif quote card, concise autonomy-respecting copy and collapsed preferences keep the experience separate from playback.

Privacy: no browsing writes, timing/view/dwell/eye/mood tracking, remote generation/analytics, diagnosis or cure/relationship/safety promises. Explicit saves are stored locally for favorites. Optional personalization is off by default and explains that only saved IDs and More like this choices tailor the mix on this browser/device. A dedicated bounded/validated localStorage record stores version/saved IDs/opt-in/capped category counters. Disabling opt-in stops weighting; reset removes the record, never music selection. Storage errors allow in-memory operation and reset failures honestly warn persisted choices may reappear. Messages do not auto-rotate; the user can always choose another tab. Hiding the entire navigation item is not implemented because the user requested the visible tab.

Verification: 33 frontend tests pass, including browsing no writes/no API calls, saved reload/empty state, explicit weighting and deterministic distribution/variety, disabling, corrupt/denied/sanitized storage, reset and music-state preservation. All 16 Python tests and diff checks pass. Seven Chromium viewports pass categories/save/settings/reset, real #quotes reload, Back/Forward, no overflow, five visible >=44px navigation controls and unchanged audio source/play attempt count. Initial browser launch failed due missing libnspr4.so in this sandbox; restored test-only packaged al2023 libraries and reran. A browser regression after #quotes reload -> Back -> Home exposed uninitialized Home content: renderTab's history-only path previously marked tabs loaded without fetching. Changed it to skip fetching only already-loaded tabs, then the full suite passed. Inspected mobile-quotes screenshot; no generated images committed. Actual running UI-only preview port 5050 also passes direct #quotes/local Next/no uncaught page error checks; its Rust/audio backend is still unavailable and labeled as such.

Reconciled stale initial-ancestor Git metadata with the active remote branch using an ancestry-guarded mixed reset, preserving all working files and unrelated tracked pyc deletion; no branch switch. Linux CI is recorded below after completion. Device accessibility/real audio and all earlier iPhone/backend isolation/resource/security/tunnel gates remain open. Scores remain UI 4/5, Architecture 2/5, Backend 2/5, QA/Security 3/5, Networking 1/5. No change in runtime memory claims.

Quotes CI confirmation: source 988c660 passed all three jobs in https://github.com/AfzalAshraf/ayshamusic/actions/runs/43156823415 . This includes production Rust tests/release, 33 frontend and 16 Python checks/shell syntax, all seven Chromium viewports with Quotes behavior, and the unchanged native audio prototype's unit/build/media smoke job. No new runtime/backend failure was observed by those tests; physical-device accessibility/audio and prior architecture/resource/security gates are still not certified. Quotes are available immediately in the existing sandbox UI-only preview after refreshing; the user's Rust-embedded Ubuntu page still requires the normal pull/build/install flow.

## Requested home wording and small AYSHA profile mark

Active department: UI/UX Design & Frontend. Updated the served home heading exactly to “Your music.” followed by “Your kind of moment. For you from my heart”. Changed the header mark from AY to AYSHA, retaining aria-label="Aysha's Music" and the existing circular badge with smaller 8px lettering/.25px spacing. Removed the <=360px hide rule so the requested full name stays visible on narrow phones. No playback, Quotes, service, icon or dependency changes; exact dedication preserved.

Verification: 17 Python tests, 33 frontend tests, diff whitespace and all seven Chromium viewports pass. Added static exact-copy checks and browser assertions for heading text, visible full profile name at 320px and text fitting inside the badge. Inspected updated 390px home screenshot: heading wraps naturally and the AYSHA mark fits. Generated screenshots remain ignored. UI stays 4/5 pending physical-device accessibility/audio checks; other scores/gates unchanged. The sandbox UI preview reads the changed source after reload; Ubuntu's embedded page still needs the standard build/install. CI recorded below when available.

Home/profile CI confirmation: source d894503 passed all three jobs in https://github.com/AfzalAshraf/ayshamusic/actions/runs/43157810587 (production Rust tests/release, 17 Python/33 frontend/shell checks, seven Chromium viewports and unchanged native audio prototype checks). The running sandbox port 5050 also serves the requested wording and AYSHA badge. No new dependencies or host deployment performed.

## Full-player love button and after-song ratings

Active department: UI/UX Design & Frontend. User requests a heart in now-controls and a rating after each completed song, skipping previously rated songs. Kept the approved design/dedication/header/Quotes and existing Rust API/service; no dependency or backend/session change. Added full-favorite (44px, dynamic accessible label/aria-pressed/filled pink) using existing like/unlike/liked endpoints and full Track JSON. Added per-track mutation guards, shared initial reads with revision protection against stale reads, shared card/full-player heart rendering and honest inline failure feedback. No selected track disables the button. Added a balancing spacer so Play stays centered with the new right-side heart. Existing server favorites are still global/in-memory and are not represented as permanent device-private storage.

Browser audio natural end snapshots the finished song and presents a dismissible 1–5 rating without waiting before Next. Stopped-source end events are ignored. No prompt on manual Stop/skip/error. Already-rated IDs are skipped after reload. Scores are explicit browser-local only; no listening-time/mood inference, remote analytics or automatic recommender changes. Up to 1,000 validated scores/titles/IDs are retained without evicting existing ratings; new scores at capacity are refused visibly. A bounded pending prompt is replaced by the latest unrated completion if ignored. Not now does not rate. Separate dedicated storage, sanitized Library list (latest 20), current-song score and delete-all support; storage errors report page-only behavior/reset failure. Favorites, quote settings and music selection use their existing separate boundaries. Two mutually exclusive rating-card instances support both native modal and non-modal playback without blocked/inert controls, a focus trap or forced pause.

Known scope limit explicitly communicated: home Bluetooth/mpv has no reliable completion event in the current API; automatic prompts therefore cover browser natural ends only. Do not manufacture Bluetooth end events from is_playing false/timeouts. Cross-device ratings/aggregation are not implemented. Actual output, iPhone streaming, independent sessions and 100 MB target remain open.

Verification: 46 Node tests and 17 Python checks pass; seven Chromium viewports pass heart add/remove/Library/reload/centered Play/44px targets and source preservation, rating of completed rather than next track, uninterrupted fixture navigation, visible modal/non-modal buttons and skip-after-reload, Not now, no overflow and prior navigation/Quotes/header/playback regressions. The fixture now models the existing favorites endpoints instead of returning a fixed liked list. Test corrections: shared .quote-settings/.quote-reset selectors were scoped to Quotes after adding Library settings, and a hidden Library count/old card was sampled before asynchronous reload—now tests enter Library and await the actual update. No production API failure was hidden by these corrections. Inspected generated 390px favorite screenshot (not staged); no production host installation. CI result to follow. Overall scores unchanged: UI 4/5, Architecture 2/5, Backend 2/5, QA/Security 3/5, Networking 1/5. Unrelated tracked pyc deletion preserved.

Published feature source e638a55. GitHub Actions run 43159265841 completed successfully: production Rust tests/release build, Python/frontend/shell checks, seven browser viewports and isolated Rust audio prototype tests/build/media smoke all green. CI emitted platform action-runtime/runner migration warnings but no failed jobs. Evidence: https://github.com/AfzalAshraf/ayshamusic/actions/runs/43159265841 . This confirms build/regression checks, not installation or real Bluetooth completion support.

## Romantic love-letter landing (UI/UX Design & Frontend)

User requests a love bubble/letter on startup, “You are loved…”, black-and-white background, two lyric-style lines at a time for 15 seconds, bottom loading label, then full UI, with romantic icons. Implemented a dependency-free native dialog on every full page load: white heart bubble/concentric outline backdrop, greeting to Aysha, title, three original two-line verses for 5s each, Afzal signature and bottom “Opening your little world of music”. Progress means letter time, not a fake percentage of network load. Keep existing backend loading/failure truth and no media autoplay. Native dialog supports keyboard/inertness, Skip/Escape, safe areas/landscape scroll, reduced-motion, explicit timer cleanup and wall-clock deadline after background throttling. No new storage/history/session behavior. Disabled JS or unsupported dialog exposes the normal UI rather than trapping the user. Repeated in-app navigation does not replay it; full reload does. Refined 5 short verses to 3 short verses for more comfortable reading at the requested 15s. Added subtle heart motifs to original Home/Library SVGs, preserving conventional action/control silhouettes, labels, existing design, heading, Quotes, favorites/ratings and dedication.

Keep/change/prove/recover: frontend-only opening and two existing local symbols; Rust/API/output/resource ownership untouched. Revert this source commit to remove the opening without altering favorites/ratings/preferences. Local 50 Node cases, 17 Python checks and all seven Chromium fixture sizes pass; diff whitespace clean. Browser checks include actual 15s natural reveal at 390px, verse changes, Skip on other layouts, Escape/reduced-motion at 360px, no horizontal overflow, 44px reachable Skip, no audio start, and all previous navigation/favorites/ratings/refresh flows (now dismissing startup after document reload). Unit checks include exact 5s/15s boundaries, timer cleanup, wall-clock jump, no requests/storage writes and dialog fallback. Inspected previews/mobile-love-letter.png (ignored): white heart, centered readable serif heading, subtle rings, signature and loading at bottom. CI pending publication. No host installation, real phone/TalkBack or new hardware/audio acceptance claimed. UI 4/5; Architecture 2/5, Backend 2/5, QA/Security 3/5, Networking 1/5 unchanged. Existing Bluetooth ratings/favorites persistence/100 MB/multi-device gaps remain out of scope. Unrelated pyc deletion preserved.

Published opening source f10104b. CI run 43159996832 passed all three jobs: production Rust tests/release build and Python/frontend/shell checks; seven-viewport browser checks including the actual timed opening; isolated Rust audio prototype build/tests/media smoke. https://github.com/AfzalAshraf/ayshamusic/actions/runs/43159996832 . The existing port-5050 UI-only preview response was fetched and confirmed to contain the new opening. This is not host deployment or live audio acceptance. Platform action-runtime/runner migration warnings remain non-failing.

## Love-letter reading-time correction

Active department: UI/UX Design & Frontend. User reports the two-line letter is difficult to read and asks for 5 seconds per pair. Inspected timing already advances at 5 seconds, but text spent .65 seconds fading from transparent. Removed the opacity animation: each pair is now fully opaque immediately, with only an 8px gentle upward entrance. Centralized LOVE_VERSE_MS=5000 and derived the total/progress from the actual verse count (3 pairs = 15 seconds). No added delay or backend/media change. The lost full-opacity reading time is demonstrated by the old CSS, not a claim about the user's device or deployed binary. Local 50 frontend cases (including exact timing boundaries), 17 Python checks and all seven browser sizes pass; added a computed-opacity=1 assertion during the actual opening. Existing timed reveal, Skip/Escape/reduced-motion and player regressions remain green. Diff check clean. No host installation or real-device reading acceptance claimed. UI remains 4/5. Unrelated pyc deletion preserved.

## Interactive heart trail + tap burst (UI/UX Design & Frontend)

User supplied a combined mouse-trail/click-burst example. Adapted it into the approved app instead of adopting the demo body/background/layout or adding emoji/core-icon packages. Reused local heart SVGs: white for the love-letter opening and rose/pink otherwise; eight hearts per primary tap/click, one-second scatter/float/fade and a throttled mouse-only trail (90ms). Keep the five-second fully opaque letter verses/15-second opening, Skip, media controls, history, search, ratings, dedication and approved UI. No storage, requests, analytics or media actions in the effect. No backend/dependency changes.

Safety/recovery: capped 32 live particles/timers across three clipped, aria-hidden/noninteractive layers, including native dialog top layers; passive listeners without preventDefault. Ignore keyboard activations/form editing/dragging/non-mouse trails. Remove via animationend and 1.1s fallback; clear on hidden document and reduced-motion changes, and suppress while either applies. Reverting the source change removes decoration without touching user data. No claim of meeting service RAM budget based on bounded DOM nodes.

Evidence: local 56 frontend cases, 17 Python checks, seven Chromium sizes and diff check pass. New unit cases verify burst count, local SVG, no fetch/storage/audio calls, host routing, exclusions/throttling, cap/eviction and animation/fallback cleanup, preference/visibility cleanup, invalid inputs and idempotence. Browser verifies actual tap-generated bursts in the opening, white color/pointer transparency, favorites clicks still work through full-player decoration, reduced-motion suppression, desktop trail/cap/live preference cleanup, plus all prior UI regressions and timed reading. Initially browser expected exactly eight total particles; a separate mouse trail was also present in emulation, so the corrected assertion counts exactly eight burst particles (not all decorative nodes). No app exception was hidden. Visually inspected ignored mobile-love-hearts.png. CI pending publication; no host install or real Android performance/screen-reader acceptance. UI 4/5; Architecture 2/5, Backend 2/5, QA/Security 3/5, Networking 1/5 unchanged. Existing session/audio/storage/budget gaps and unrelated tracked pyc deletion preserved.

Published heart-effects source 9ab3b88. CI 43161245095 passed all three jobs: production Rust tests/release build and Python/frontend/shell checks, seven browser sizes, isolated Rust audio prototype tests/build/media smoke. https://github.com/AfzalAshraf/ayshamusic/actions/runs/43161245095 . Fetched existing UI-only preview on port 5050 and confirmed heart-effect code/modal layer markup in its response. This is source/fixture/build verification, not production-host installation or physical-device performance evidence.
