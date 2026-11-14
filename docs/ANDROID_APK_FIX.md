# APK missing icons and HTTP404 fix

## Report and reproduced source defect

The user likes the installed APK's UI, but icons are invisible and music loading
reports HTTP404 / tap the tab to retry. The Android Rust library's router did not
serve /icons.svg or /api/search; the embedded website references both. Retrying a
nonexistent route cannot help. This identifies a phone-local API contract failure,
not evidence of a provider403, a Linux outage, or a need to start Vercel/systemd.

## Implemented

- Integrated origin/main bf37384's Android app onto the current Arena branch,
  retaining Linux service files, existing frontend changes and diagnostic tooling.
- Embedded existing project-owned SVG sprite with correct SVG MIME type.
- Added phone-local search using a fixed HTTPS provider endpoint with one in-flight
  request,12s total deadline,2MiB response ceiling,200-character query limit,
  bounded traversal, strict video IDs, deduplication and at most20results.
  No shell, yt-dlp, Node, hosted Aysha backend or invented catalogue is used.
- Added bounded200-track queue/history/favorites endpoints and native navigation.
  These are process-memory collections, not a new promise of persistent sync.
  Empty queue/history return a meaningful409instead of404; add songs to the queue
  for Next. No automatic recommendation acquisition was added.
- Native mode is embedded into Android HTML before the first tap; the Linux page
  retains browser mode. Native Play sends JSON + selected track metadata and never
  requests the Linux-only /api/stream path. Next/Previous do not double-start audio.
- Errors invalidate failed Explore loads so tab retry can actually run again.
- MainActivity waits for bounded phone-local API readiness off the UI thread.
- Avoided blocking either Tokio worker on a full media ring; channel pieces are
  at most16KiB. Removed the12s total-song HTTP deadline while retaining connection/
  read idle timeouts; search and resolver requests retain bounded total deadlines.
- Native audio/Telegram dependencies are gated by phone/android features, so default
  Linux builds do not gain ALSA or Telegram runtime requirements. Linux host not
  reinstalled/restarted; reqwest uses the shared main Rustls build configuration.

## Install-test packaging

CI creates version1.0.1-preview, application ID com.afzal.ayshamusic.preview,
launcher name Aysha Music Preview. Kotlin namespace/JNI names remain unchanged.
It installs alongside the existing com.afzal.ayshamusic app, so ephemeral CI debug
signing does not require deleting the old app/preferences. Local data is separate.
This is NOT a production-signed/store-approved release. Production signing remains
an explicit later action; never put keystores or passwords in chat/Git.

Branch CI uploads an artifact named ayshamusic-arm64-release. Despite that inherited
artifact name, AYSHA_PREVIEW=1 marks its APK as a preview. Do not use apk-latest:
that public main release is deliberately NOT overwritten by branch test builds.
Android8+/ARM64 only. Download/extract the artifact ZIP, then install app-release.apk
and open **Aysha Music Preview** (not the older app). No Linux command is necessary.

## Verification and remaining device check

Locally87Node tests and137Python tests pass, plus diffcheck. New frontend tests
exercise native JSON/no-stream playback, one-call native navigation, logical API
errors and actual failed-tab retry. Rust fixture/router tests exercise SVG MIME/
contents, local endpoints not404, invalid searches staying offline, and queue/likes
round trips without provider/audio hardware. Search parsing is tested with fixtures,
not a claim of today's live provider behavior. CI runs phone-feature Rust tests,
Android cross-build, Gradle assembleRelease and APK size gate.

The first corrected build cb0b4bb (Android CI43586093333) passed all steps and
uploaded its artifact; Linux CI43586093327also succeeded. Side-by-side packaging
build800427b CI43586401555also PASSED all steps (2m57s), including Rust phone tests,
Android native+Gradle build, size gate, upload and website contract tests.
Install-test artifact: https://github.com/AfzalAshraf/ayshamusic/actions/runs/43586401555/artifacts/14110452124
Artifact name ayshamusic-arm64-release; compressed archive size3,665,841bytes,
not the APK installed size or phone RAM. GitHub reports archive SHA256
62956cd2817450924cc688e572d4a00883bc393c5ed1e618511569897532a8f1.
The download ZIP contains app-release.apk; sign-in/repository access may be required.
Artifacts expire14daysafter upload. The older public apk-latest is unchanged.
Network policy blocks the workspace's GitHub log/asset redirect hosts (EOF), so the
agent cannot independently download/inspect the generated APK bytes here. GitHub
CI/job/artifact metadata remains accessible. No package signature or on-device
playback/memory verification is fabricated.

On the phone, preferably with the Linux server off:

1. Open Preview and verify Play/Home/Search/heart/Back/Next symbols are visible.
2. Search for a song, confirm actual results and tap one. Verify audible playback,
   Pause/Play/Stop. Add another result to queue, then test Next/Previous.
3. Close/reopen Explore after a failed request; retry must either load results or
   show a specific connection/provider failure, not an unchanged missing-route404.
4. If loading fails, send the exact NEW visible message and whether it happened on
   Home/Search or after tapping a result. Do not repeatedly run the provider Test.
   A502provider refusal is a different layer from the fixed local404; it must not
   be 'fixed' by weakening guards or silently adding an external server.

Real-device audibility, sustained playback/background controls, source reliability,
store eligibility and Android app/WebView RAM are NOT yet accepted by these builds.
Latest supplied Linux figures are not phone memory and have not been remeasured.
