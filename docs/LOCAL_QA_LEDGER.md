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
