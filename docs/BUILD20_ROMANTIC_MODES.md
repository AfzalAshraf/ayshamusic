# Build20 — romantic launcher, native shuffle and repeat

## Approved scope

User supplied `IMG_6795.jpeg` (rose-gold heart/A Aysha tile), requested a more romantic app icon and working shuffle/repeat as a larger app upgrade. This explicitly supersedes the earlier image9-only launcher choice. Preserve Play symbol, compact player/dedication/themes, package/session paths, Rust backend/Telegram and Media3 output. Lyrics remain deferred by prior choice. No new extraction allowance, source bypass, live provider/account probe or runtime language added.

Build20/code20/1.8.0-preview, ARM64/Android8+, same Android Audio package and port17652. Candidate only, not production certification.

## Romantic launcher

AI-generated rose-gold heart/A monogram with small sparkle and full-bleed plum field, inspired by supplied reference rather than a photographed tile pasted onto a launcher. No tiny wordmark. Packaged adaptive512px foreground, dark #210817 background, legacy square/round48/72/96/144/192px. Generated1024px master `android-app/artwork/romantic-launcher.png`, reproducible development-only Pillow generator. Old black/silver master is archived, no longer packaged. No Python runtime in APK, no recoloring/replacement of player Play. Adaptive emblem remains in central mask-safe region. Physical OEM launcher/themed-icon rendering still needs confirmation.

## Actual playback behavior

- `play_modes::Modes` with bool shuffle and validated repeat enum Off/All/One. JSON GET/POST `/api/playback_modes`; rejects unknown fields/invalid values. Preference file <=2KiB, app-private `playback-modes.json`, write/sync/atomic rename; failed writes don't publish changed state. Defaults off/off on missing/corrupt file. Mode reads do not initialize audio.
- Player snapshot includes authoritative modes. UI updates from server, saves whole validated preference pair, disables controls during request, keeps state on failure with visible error. No fake-success localStorage toggle. Buttons have pressed states, labels, icons and44px targets; repeat-one has distinct icon. Unsupported Linux/static preview backend doesn't show decorative nonfunctional controls; Linux server/playback unchanged.
- **Shuffle on:** choose random queued item (non-security std seeded hash choice); after successful start remove only that item's ID, not the head. Failed starts leave queue intact. When queue is empty and repeat isn't All/One, choose among already-filtered fresh provider Mix candidates; never bypass history/current/version filtering or fetch extra metadata solely for shuffle. Selection off preserves provider/queue order. Previous remains history-based.
- **Repeat one:** only natural-end navigation selects current song. Manual Next still skips normally. Queue is not consumed, current isn't marked skipped and mix isn't consumed by natural replay. Uses existing ticket/operation guards and normal cached/cloud/source playback path; no Java-side second player/loop or second autoplay controller.
- **Repeat all:** loop a bounded <=200 **session collection of explicitly selected or queued songs**, in selection order or shuffled. Provider-recommended Mix songs are not added implicitly. Explicit queue gets priority until consumed; then collection wraps. Clear queue resets collection to current song; empty/single-song collection can repeat current. UI explains this instead of claiming the infinite Mix can be repeated as a finite playlist.
- Modes persist across app restarts, but queue/history/repeat collection are still process-session state (as baseline queue was). No claim of persistent playlists. Stop retains selection/modes but cancels ticketed automatic navigation; Next/Previous retain existing duplicate-start guards. Archive ID dedup/quota/cancel semantics unchanged; repeat isn't permission to delete/reupload cloud messages.

## Verification / limits

Local **122 Node /143 Python PASS**. Updated PNG/sprite contracts and archived-master checks. Provider-offline Chromium dashboard/seek checks PASS with mode off/all/one cycling, distinct one icon, save-failure rollback, reload restoring server settings, all3themes ×5viewports including tall phones/landscape, centered/fitted controls and bounded cover-to-Play gaps. Initial full suite hit a test-only async assertion before the POST response updated DOM; fixed by waiting for confirmed mode state, not weakening assertions. Complete local mobile/dashboard/seek/Aurora/static preview rerun PASS. Linux **44930143809 SUCCESS, all six jobs PASS** at test-only source **bf8fa6d365fedcc30a7f0d33265f00ca960c9242**. Earlier Linux attempts used the stale async test; they are not substituted for the corrected passing suite.

New Rust tests cover persisted/default/corrupt/invalid modes, ordered/shuffled bounds, native API roundtrip/422 validation, natural RepeatOne vs manual Next/Previous, queue-preserving candidate selection, RepeatAll wrap and Mix fresh-candidate shuffle. No local cargo toolchain. Android source **1f723ceb92847e2f2a2b06590748d2e377c3dc32**, run **44929889579**: release job **136924054936 SUCCESS**, including locked phone Rust tests, Linux server check, ARM64 native library, release/R8, size budget and Android contract tests. Native emulator job **136925300290 SUCCESS, all eight tests PASS**: actual Media3 playback/pause/resume/error/long-gap/seek/full-track decode fixtures, shared-link allowlist and bounded square artwork. Overall Android run SUCCESS. Results: `docs/research/build20-media3-emulator-result.json`. These are separate from the new native-backend mode-decision tests and do not prove physical background repeat/shuffle end-to-end. Later test-only commits do not change packaged behavior.

Physical natural-end repeat/shuffle with real music/background/notification controls, process death, launcher masks, signing certificate/update compatibility, memory/battery/network/R8 device acceptance remain gates. Native decoder fixture tests are separate from backend decision-unit and UI-contract tests; do not portray them as a real Telegram/provider account or full native-repeat end-to-end proof. No all-bugs-fixed/production-signed claim.


## Candidate artifact

[Build20 ARM64 APK ZIP](https://github.com/AfzalAshraf/ayshamusic/actions/runs/44929889579/artifacts/14717162652). GitHub artifact14717162652, ZIP **5,174,007 bytes**, archiveSHA256 **c26d715149132b9934b905842b7f559a524021ddd6fe019199bb6efe1d95e620**. This is ZIP metadata, not an extracted APK digest or signing certificate inspection. Explicit CI preview flag remains; do not represent as production-signed. Retention14days, not a permanent published release.

Extract `.apk`, attempt **Update** over current app, then open the expanded player to find Shuffle and the three-state Repeat button. If Android refuses Update, stop; do not uninstall/clear data and lose Telegram/session settings. Modes persist; repeat collection/queue do not survive process recreation. On some launchers the old icon can remain cached until the launcher refreshes; clearing app data is not a remedy.


Native report artifact14716879977, ZIP27,504bytes, archiveSHA256 **b3c69e082d0aacdfd621352918984ac841486f8eadafab6481234028726a5850**. GitHub metadata, not a signature or device certification.
