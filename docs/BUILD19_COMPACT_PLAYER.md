# Build 19 — compact, balanced player

## Decision

User found excessive cover-to-Play spacing and requested a polished player for his wife. Lyrics were optional; clarification selected **Perfect the player first** (layout only). No lyrics provider, online request, placeholder lyric panel or invented lyrics added.

VersionCode19 / 1.7.1-preview. Same ARM64 Android Audio package, port17652, Rust Telegram/backend, Media3 playback and existing three themes. No changes to extraction, audio clock/speed, download/upload behavior, account/session persistence, native playback implementation, launcher or exact dedication.

## Layout fix

The Build18 player body used `flex:1`, consuming all remaining dialog height and leaving a large empty region between song metadata/seek and playback controls. Changed it to `flex:0 1 auto`: content takes its natural height but can shrink/scroll when necessary. Spare height is balanced above and below the music group via header/footer auto margins, not inserted before Play.

Tightened cover/title/seek/time/control spacing; Aurora title now tops out at28px instead of32px, keeping longer titles elegant while retaining the two-line clamp. Cover and Play now form one visual group. Play remains centered; Stop stays outside the shrinkable body and visible without scrolling. Volume, account/queue/favorite actions, rating prompt, error details and dedication retained. Small-height handling and safe-area insets preserved. Current theme/default remain unchanged.

## Checks

Local **122 Node / 142 Python PASS**. Extended provider-offline Chromium checks cover **all three themes × five viewports**: 320x568,390x640,390x844,430x932,844x390. For heights640+, assert cover-bottom-to-Play-top gap <180px (including metadata and seek), time-to-Play gap <=28px, dedication within viewport. At all sizes assert Play/Stop visible at dialog scroll0. Added assertions fail against stretched layouts; no production state mocked as passing playback evidence.

Actual390px Aurora fixture screenshot inspected: cover340square, title/artist/seek and Play in a contiguous group, balanced outer whitespace, footer dedication intact. Screenshot uses fixture artwork, not a real provider request.

Source **ce293a1904b900e4fca1adb94edac69068f4c7cf**. Android run **44535482354 SUCCESS**: release job **135599623953**, native emulator job **135602975685**, all **eight instrumentation tests PASS** (six Media3 pause/resume/gap/error/seek/full-track tests plus shared-link validation and square artwork). Release/R8/ARM64/locked phone Rust and size checks passed. Native annotations: `docs/research/build19-media3-emulator-result.json`. Instrumentation is provider-offline and excludes Rust JNI acquisition; release compilation is separate evidence.

Linux run **44535482400 SUCCESS, all six jobs PASS**, including full mobile, dashboard/seek/theme, Aurora and generated static preview browser suites. Full suites also passed locally. No new public-source probe or physical-phone acceptance performed.

## Installation / remaining gates

Preview install-over candidate, not production-certified. No signing-certificate continuity inspection or physical background/notification/process-death/network/RAM certification. Same package and higher versionCode are intended for Update, but if Android rejects Update, **do not uninstall or clear data** (Telegram session/settings loss risk). Lyrics deliberately deferred by user choice. Historical test fixtures/issues remain documented in Build18; no claim of universal crash-free behavior.


## Verified preview artifact

[Build19 ARM64 APK ZIP](https://github.com/AfzalAshraf/ayshamusic/actions/runs/44535482354/artifacts/14535483760). Artifact **14535483760**, ZIP **5,085,921 bytes**, GitHub archive SHA256 **05fea0548b3e08cc607ca4d4fa37c3c99e8efd621c211f70f4a15d0cf9c6ea89**. ZIP metadata is NOT an extracted APK hash or certificate inspection. CI preview flag remains explicit; artifact is not certified production-signed. Artifact retention14days, not permanent publishing.

Emulator result ZIP artifact **14537136015**, **28,309 bytes**, archive SHA256 **4d2b0ea24d593b1bcebac2e5be1e2246fbdc2c57c7ec7e9c60baad8960a6f601**. Local ignored screenshot `previews/build19/refined-player.png` shows the actual HTML/CSS with abstract fixture art. Lyrics remain deliberately absent, per user decision.
