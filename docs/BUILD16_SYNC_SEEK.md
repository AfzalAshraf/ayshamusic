# Build16 — Telegram dashboard, themes and seeking

## Download — built and checks passed

**[Download Build16 APK ZIP](https://github.com/AfzalAshraf/ayshamusic/actions/runs/44317291462/artifacts/14432927990)**

Unzip then install over the existing app. Version1.5.0-preview/code16; source **d84ea2369a500780e816576f72b3608ecaa0093a**. No blind uninstall on a signing conflict.

Android44317291462 **SUCCESS** release134846834928/emulator134846834444: Rust host checks, ARM64release/R8, APKsize budget and Android contractsPASS. **All6actualMedia3fixturesPASS**, now includingseekForwardAndBackwardWhilePausedThenResume, plus600s@4x and10sgap. This is real emulator Media3 against local fixture media, not a live provider/Telegram/physical speaker test.

Linux44317291647 **SUCCESS all6jobs**, including new sync-seek browser fixtures. Local119Node/142PythonPASS; local full responsive browser rerunPASS all7sizes/nativeSettings/iframe; dedicateddashboard/cancel/themes/seek-request and generatedstaticpreview suitesPASS. Initial desktop butterfly timing failure was transient on unchanged animation code and did not recur locally or in final CI; root timing cause not established, not silently counted as an initial pass. Inspected390pxCrimsondashboard screenshot (fixture counts clearly restricted to tests).

APK artifactZIP4,995,978bytes; **ZIP**SHA256 `398b8e84dde0c8f7654086af4f04ac472132c9af3ba3bbe2ed001f047af08dd9` (not innerAPKhash). Metadata verified for built source; local blob-storage downloads remain outside allowed workspace egress, so no local binary/signature inspection claimed. Hosted GitHub download may need sign-in;14dayAPKartifactretention. Emulatorreportartifact14433282386/24,589bytes, ZIPdigest `6e7604608fa385053e6315bc97de077d166929ad7cb96566e89c449765e26c4b`; safe outcomes in docs/research/build16-media3-emulator-result.json.

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

Final automated results are recorded above. User physical playback/fast Next acceptance applies to build15, not automatically to the new archive/cancel/seek features. Superseded first-source Android44316879710 was cancelled for this source update, not counted as failure or as passing instrumentation. Local Rust unavailable; Rust/Android checks ran in CI. No new account login, public provider, physical upload/cancel or phoneRAM measurement was performed.

## Acceptance / error & risk ledger

Phone acceptance: install-overexisting; no HomeTest; select Crimson/Charcoal and reopen; sign in vs API-only badge; play several songs, observe actual queue stages/counts; cancel queued then active download/upload; check SavedMessages and no cancellation deletes; seek forward/back paused and playing, using full and mini slider; Next/Stop/background remain working. Existing transfer bytes/cloud sends may finish at cancellation boundary. Failed upload may be retried per existing policy. Quota intentionally unchanged until user chooses otherwise. No exact-provider/fullcatalog/phoneRAM/battery/physicalseek/archive/cancel acceptance yet.

Evidence ratings0–5: baseline physical playback3/5(user-reported), new UI/logic4/5(unit+browser), native seek3/5(actualemulator), realTelegramupload/cancel0/5pendingdevice. Build errors: none in final checked source. Initial desktop butterfly assertion passed on rerun and CI; timing-flake risk remains. Actions workflow deprecation warnings inherited. Manifest/artifact proof will follow.
