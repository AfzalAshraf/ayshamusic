# Build15 — Mix and responsiveness upgrade

## Download — built and checks passed

**[Download Build15 APK ZIP](https://github.com/AfzalAshraf/ayshamusic/actions/runs/44149184745/artifacts/14357224361)**

Unzip and install the APK over the existing app. Launcher **Aysha Music**, Android App info **1.4.0-preview**, code15. Built source **2d8bf5ab001d44f153aaecb86dbec3cccce73381**; source/engine diagnostics are deliberately no longer shown in the player/Settings.

- Android run44149184745 **SUCCESS**, release134272532279 and emulator134272531721. Rust host checks, native ARM64/R8, size budget and Android contracts passed. All5 actual Media3 fixture tests passed, including600s@4x playback and10s source gap. This does not exercise the live Mix endpoint or a real Telegram upload.
- Linux44149184711 **SUCCESS all6jobs**. Local113Node/142PythonPASS; responsive browser/nativeSettings/iframe and generated static-preview browser suitesPASS. Inspected the390px Settings screenshot: layout contained, no diagnostic label clutter, API-only sign-in state clearly shown.
- GitHub artifact ZIP4,981,010bytes, SHA256 `493c301a6f80f03a4087254cc4b64a967b60a3ded29b2a07cb99135af253b2d3`. **ZIP digest, not inner APK digest**. Metadata confirmed at built source; APK size gate passed. Local artifact download is blocked by the workspace's outbound-host restriction at GitHub's blob-storage redirect, so no local binary/signature inspection is claimed. The hosted artifact link above is the deliverable; normal GitHub sign-in may be needed.14day APK artifact retention.
- Safe emulator outcome record: `docs/research/build15-media3-emulator-result.json`. CI emits action/runtime deprecation warnings (checkout/setup-java/upload-artifact); no failed job in this run. Workflow modernization is deferred rather than bundled with the playback upgrade.

## Baseline and scope

User confirms build14/source2928821 plays and advances on their physical phone (Asalayavale/Ponnaninja Pennu screenshots). This is positive phone evidence for those selections, not full-song/background/Telegram proof or exact Suno Aisha acceptance. Preserve that acquisition profile, exact-ID/directAAC policy, head+tail checks, Media3 output, Saved Messages backend and rose/dark design.

Build15: versionCode15 /1.4.0-preview, launcher **Aysha Music**, same package `com.afzal.ayshamusic.androidaudio10`, port17652, Android8+/ARM64. Do not uninstall to resolve a signing mismatch: this can erase the Telegram session.

## Implemented changes

- Next uses the actual anonymous YouTube radio playlist (`next`, videoId seed, playlistId RD+seed), preserving provider order. No bespoke preference-search ranking for Next. Manually selected songs seed a fresh mix; queued songs/Previous retain user priority. It is an anonymous related-song mix, not the user's signed-in YouTube Music personalization.
- At most40 metadata candidates cached for30minutes, fetched after successful manual Play. No audio prefetch, additional runtime or dependency. Same-ID/current/recent50 tracks and recognized title variants are excluded. The screenshot's singular “Lyric Video” and single-word song title variants now have regression coverage. Metadata cannot identify every cover/translation; do not promise universal music identity.
- Empty/unavailable/exhausted mix gives an explicit choose-another-song message; never disguises generic search as a mix or deliberately repeats the seed. No alternate client retries after a refusal.
- Search waits/coalesces instead of immediate local429;12query cache/5minuteTTL, rechecked after single-flight wait. Next no longer consumes the Explore search slot. A bounded30s wait may still return503 on a long backlog, and upstream provider errors remain possible.
- Telegram-first lookup budget capped at4s instead of letting an unknown song consume its15s cloud-lookup deadline before YouTube fallback. Slow Telegram retrieval may now fall back sooner; archive/download/save semantics otherwise unchanged.
- Android ready/events use an executor independent of potentially slow control requests. Next/Previous have a separate single-flight executor/read timeout60s; Pause/Stop and events retain1500ms timeout and independent executors. Stop tickets invalidate manual navigation during metadata waits too. Previously a notification Next could block the readiness event behind itself. This is a code-level circular-wait risk, not a measured physical timing result.
- Native natural-end supervisor stays in Rust, once-per-ticket,1s interval; metadata Next works with WebView suspended. Stop/new-selection tickets checked before and after metadata wait. No frontend duplicate native autoplay. Physical background behavior remains to test.
- Settings distinguishes saved API configuration from authorized sign-in; reports app-indexed saved-message count (not whole Telegram cloud), pending, failed, cooldown and archive errors. States Saved Messages destination/no channel or bot; explains extra data/temp storage. Authorization is not a promise of current server reachability or successful uploads. Status refresh10s only while visible Settings; no credentials returned or stored in browser storage.
- Removed player/Settings build/engine labels; technical identity API remains for native handshake. Status polling1s instead of400ms and nonoverlapping, skipped in hidden documents. Home/Explore start with romantic melody discovery instead of Aysha-name/Jazz searches. Linux route/deployment is preserved.

## Research and bounds

Inspected installed MIT youtubei.js18.3.0 `Constants.js` WEB version, `TwoColumnWatchNextResults.js` playlist shape and `PlaylistPanelVideo.js` title/byline/video fields, plus own catalog/MediaApi/Telegram code. Reference https://github.com/LuanRT/YouTube.js (already packaged MIT notice). No JS runtime linked. This is a small Rust metadata adapter, not full youtubei.js integration. Provider changes can invalidate this unofficial anonymous interface.

No live provider probe was run. Previously sealed/challenged source attempts remain sealed; there is no challenge bypass, credentials injection or identity rotation. Fixture parser success does not establish live phone Mix availability.

## Checks and acceptance

Local:113Node tests PASS,142Python tests PASS; Chromium fixture suite PASS at320/360/390/412portrait, tablet, landscape, desktop, plus native Settings both fixtures and static preview checks. Rust phone tests, ARM64 release/R8, APK size and Media3 instrumentation passed in CI as recorded above; local Rust toolchain unavailable.

Phone acceptance after install-over-existing: play a romantic seed, tap Next several times (different songs, less metadata waiting), overlap Home/Explore/category changes, verify Telegram API-only vs signed-in status and a completed upload in actual Saved Messages, finish a song with screen off, verify notification Next/Pause. No physical latency, memory, battery or entire-catalog certification yet. APK size measured in CI, not an Android RAM claim. Linux memory not remeasured.

Evidence ratings (0–5, not universal certification): baseline phone playback3/5 (reported); UI/regression fixture coverage4/5; new live mix/background/Telegram upload acceptance0/5 pending phone test. Remaining risks: unofficial metadata API availability, incomplete title identity, slow network media preparation, preview-signing continuity and unmeasured device RAM/battery.
