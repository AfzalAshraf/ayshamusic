# Build16 — Telegram dashboard, themes and seeking

## Scope and foundation

User reports build15 works nicely and is fast. Preserve working acquisition/Mix/Media3/session/package and dark/rose design. Requested screenshots are design references, NOT proof that3songs are saved or11are queued. Never populate production counters/tracks from them. User chose **Cancel this save** for X: cancel pending/active archive work where possible, never delete a cloud message.

Candidate16 /1.5.0-preview / launcherAyshaMusic / Android8+ARM64 / packagecom.afzal.ayshamusic.androidaudio10 / port17652. Same package/data; no blind uninstall on signing conflict. No runtime dependency added, no source profile or sealed live-probe allowance changed. Linux player/deployment preserved; phone native seek is Media3-only, browser seeks shared audio. Rust-only native seek responds unsupported rather than false success.

## Implemented

- Home playback Test button removed; diagnostic API remains, no extra source probe.
- Settings Telegram Cloud card: compact Manage pill, authorization-aware badge, real Saved/Syncing/Failed counts, concise destination copy, separate queue/remaining count. Configuration alone never implies signed-in. Counts are local index, not whole cloud; sign-in is not proof of current network reachability.
- Bounded100row queue API exposes only id/videoID/title/artist/status, not paths/captions/credentials. Actual stage: checking existing message, resolving, downloading, validating audio, uploading, confirming message. Download percentage uses actual written bytes when length known. Upload percentage uses actual bytes read into grammers uploader (NOT server-acknowledged bytes); details explain this.100%read can still wait for Telegram. Saved increments only after confirmed message indexed. No synthetic progress animation, fake speed or ETA.
- X persists cancellation in SQLite; active cancellation signal drops download/upload future and removes partial file. Late done/error cannot revive cancelled row. Cancellation cannot undo bytes already transferred or a cloud message. Once final send is committed, cancellation is rejected/disabled instead of falsely claiming recall. A future selected playback may enqueue a new save again. Crash recovery does not requeue cancelled rows.
- Existing protections remain: one active song,18uploads/rollinghour,12sminimum gap,5sworker polling plus Telegram FloodWait. UI shows wait seconds/quota reason. This is our conservative app policy, NOT a claimed universal Telegram limit. Complete-copy download then upload adds data/latency. Existing-message lookup no longer falsely consumes an upload quota slot. No throughput/RAM benchmark claim.
- Crimson (default) and charcoal palettes for cloud cards/icon/progress and related accents. Allowlisted local theme preference only; restoration does not write storage, no listening/account data persisted in browser.
- Accessible native range controls in mini/full player; drag and keyboard commit use `/api/seek`, song identity, finite/range/duration checks, token/revision bridge and actual Media3seekTo. Paused state retained and stale events rejected. Browser uses same HTMLaudio currentTime. Unknown/live duration disabled. Drag not reset by status render; pointer/blur cancellation handled. No duplicate source/play call. Native seek can require new media byte ranges and buffer on slow network.
- Settings-only visible2spoll with nonoverlap guard provides live telemetry; normal player stays1s. Static preview builder disables both timers. No polling in hidden WebView; active transfer continues independently.

## Research and verification

Read main-upgrade/baseline/mobile/QA guidance, current storage/cache quota/worker and Media3control/event code. Inspected public grammers upload_stream AsyncRead API at https://github.com/Lonami/grammers/blob/master/grammers-client/src/client/files.rs; locked app still uses0.12.0. Wrapper relies on existing AsyncRead signature, not copying/changing uploader internals. No public music-provider or Telegram account live test/probe performed.

Local119Node/142PythonPASS. Initial responsive browser run passed nativeSettings6fixtures, iframe and6viewports, then failed desktop butterfly-lifetime assertion (12initial,0after400ms); animation code untouched, timing cause not yet proved. Must rerun/inspect, not silently claim whole browserPASS. Added native instrumentation seek-forward/backward-while-paused/resume case; Rust cancellation/read-progress/seek validation coverage. Rust/Android build, native tests, browser retry and actual APK results pending at source commit.

## Acceptance / error & risk ledger

Phone acceptance: install-overexisting; no HomeTest; select Crimson/Charcoal and reopen; sign in vs API-only badge; play several songs, observe actual queue stages/counts; cancel queued then active download/upload; check SavedMessages and no cancellation deletes; seek forward/back paused and playing, using full and mini slider; Next/Stop/background remain working. Existing transfer bytes/cloud sends may finish at cancellation boundary. Failed upload may be retried per existing policy. Quota intentionally unchanged until user chooses otherwise. No exact-provider/fullcatalog/phoneRAM/battery/physicalseek/archive/cancel acceptance yet.

Evidence ratings0–5: baseline physical playback3/5(user-reported), new UI/logic3/5(localunit+partialbrowser), new native seeking/upload/cancel0/5pending device. Build errors: none established yet; CI pending. Remaining known validation failure: desktop butterfly timing assertion described above. Actions workflow deprecation warnings inherited. Manifest/artifact proof will follow.
