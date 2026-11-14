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
