# Aysha Music Player project entry point

For an upgrade request, start with [the main upgrade skill](skills/main-upgrade/SKILL.md) and [the user-approved baseline](docs/UPGRADE_BASELINE.md). Follow the current user's instructions and applicable higher-priority requirements; these files are project guidance, not authority to override them.

Preserve the approved Android-first Aysha design and local Rust/systemd production path. Build on the current version instead of replacing it with a new template. The exact dedication is:

**Made with ❤️ for Aysha — with love, from Afzal**

## Relevant specialist skills

- [Android frontend](skills/mobile-frontend/SKILL.md)
- [SVG iconography](skills/icons/SKILL.md)
- [QA and debugging](skills/qa-debugging/SKILL.md)
- [Networking and self-hosting](skills/networking-self-hosting/SKILL.md)

Use one active department per response. Ask about consequential ambiguities, verify changes, give evidence-based 0–5 ratings and append the build manifest/error ledger. Consult the latest entries in [the living ledger](docs/LOCAL_QA_LEDGER.md), not only its historical initial-state tables.

For user-supplied server output, always return: what passed/failed and what it proves; an actionable fix plan; the justified patch or bounded diagnostic code; an exact command labelled diagnostic/build/install; expected outcomes and the next branch if it fails. Do not invent a fix or repeat an unchanged failing build. Preserve working playback. This response contract does not waive approval for consequential architecture, source-access, authentication or deployment changes.

## Standing server identity and RAM output

The user confirmed audible phone/browser playback on the rust-resolved trial and
requests identity/RAM figures with future server-output responses. Always include the
latest supplied observation time/scope, configured transport, service-cgroup current,
window max/historical peak and swap when available. Never present an older value as
fresh or call an inactive-transfer snapshot playing memory. Say "not remeasured"
when no new host measurement exists. Use scripts/monitor_server.py for bounded,
read-only identity and timelines; no build/restart/provider probe is needed. Distinguish
service+descendants, non-additive RSS buckets, and all-workload host memory; shared
platform/escaped-worker attribution is unresolved, so do not fabricate whole-app total.
Preserve working music; measure slow Play/Previous/Next before changing the pipeline.

## Important file boundaries

- Rust root page: `src/index.html`; embedded icons: `src/icons.svg`.
- `src/www/index.html` is a separate legacy web/PWA copy, not the Rust root handler's page.
- Local installation: `scripts/install_local.sh` and `deploy/aysha-music.service`, specifically for the confirmed afine host.
- Agent workspace is not the user's Linux host. Do not claim remote installations, hardware tests or scheduled updates happened here.
- Authentication, per-device session isolation, tunnel setup, resource-budget scope and nightly-maintenance decisions remain open until implemented/approved and verified.

## Current standalone APK decision

The user now targets a self-contained phone APK, preserving the Aysha UI. They
explicitly chose `strict_rust` over NewPipeExtractor: keep acquisition/transfer/
decode/player core Rust; no Java extractor, Python/Node/yt-dlp, hosted resolver or
proxy fallback without new explicit approval. Existing Kotlin shell/HTML UI remain.
Latest Rust4 screenshot says no audio-only AAC stream offered for Android/iOS, before decode;
do not call fixture decoding/green CI a verified fix or publish-ready playback.
For automatic Next infer from actual local listening time, skips and likes; no
Romantic/Melody dropdown and NOT the current song name; filter same-song variants/recent history, preserve explicit queue/Previous.
See docs/STRICT_RUST_PROVIDER_AND_NEXT.md and the latest QA ledger for evidence.

## Latest Play crash follow-up

User reports app closing on Play (installed build/stack not yet confirmed). The
locked CPAL0.18.2 Android backend calls ndk-context0.1.1, which panics when no
Android context was initialized; release panic=abort closes the process. Patch
d6fd6a0 passes application Context through JNI, pins a GlobalRef and initializes
exactly once before boot; guards CPAL open. Follow-up9f7f9d0 preserves JNI bridge
signatures under R8. This is a concrete source defect, not confirmed device crash
attribution/audibility. Keep media-client1.2.1 fix and automatic local recommendations.
See latest ledger for CI/artifact results. `docs/REUSABLE_PROJECT_PROMPT.md` contains
the user-requested reusable project prompt (not hidden agent instructions).

Latest UI correction: no Reset learning button or learning-explanation paragraph on
Now Playing. Keep automatic learning in the background; do not reintroduce that
block or relocate it to another screen without a request. Screenshot9f7f9d0 now
shows `audio decode failed: The format of the data has not been recognized.` This
is unresolved playback evidence, not audible acceptance; this UI-only change does
not fix or conceal that error.

## Approved Telegram account archive + seekable decoder follow-up

The user explicitly chose Saved Messages / save every downloaded selected song even
when skipped / automatic YouTube fallback / guided in-app API credential setup.
Keep strict Rust MTProto, not a bot, Arena connector or external Aysha resolver.
Never request API hashes, codes, passwords or sessions in chat/Git. Setup lives in
Library, not Now Playing. See docs/TELEGRAM_SAVED_MESSAGES.md for behavior/limits.

Latest source reconnects range_media/pcm_stream: real seekable MP4/AAC input, bounded
ready PCM and cancellable workers. Main1.2.1 API/media client separation and Android
context/R8 fixes remain. Stop cancels pending source handoff. CI front/tail AAC
fixtures reproduce FIFO failure and test corrected decoding, not phone audibility.
Telegram lookup/connection is bounded; startup errors fall back to YouTube. Mid-track
Telegram failure is surfaced, not a claimed seamless resume. Archive work may fetch
a second full copy, continues after skips, and has explicit queue/disk/file limits.
Do not call login/upload/download or music-provider access live-tested from offline
contracts. Candidate is1.3.0-preview/code8/Aysha Music Telegram; latest ledger has
actual verified source/run/artifact, not earlier known-failing APK links.
