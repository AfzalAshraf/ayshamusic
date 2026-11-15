# Rust Audio and Android Audio install-test builds

## Approved scope

Two standalone Android APKs, same embedded page, Rust YouTube resolver/catalogue,
Rust Telegram Saved Messages implementation and automatic learning. No hosted
resolver, yt-dlp, Python/Node runtime, NewPipe or replacement extractor added.
Only the **Android Audio** variant uses AndroidX Media3/ExoPlayer for decoding and
audio output (explicitly approved). The **Rust Audio** variant retains rodio/CPAL.

Both have the rounded red launcher music-note icon, adaptive icons for modern
Android launchers, Settings gear navigation, and a prominent **Add Telegram
account** card in Settings. The player Play button has not been recolored. Android
chooses the final adaptive launcher mask; legacy launcher PNGs have a red circle
and transparent corners. Original dedication remains unchanged.

## Install identities

| | Rust Audio | Android Audio |
|---|---|---|
| Launcher label | Aysha Rust Audio | Aysha Android Audio |
| Application ID | `com.afzal.ayshamusic.rustaudio9` | `com.afzal.ayshamusic.androidaudio9` |
| Local API port | 5061 | 5062 |
| Audio engine | Rust rodio/CPAL | AndroidX Media3 ExoPlayer 1.8.0 |
| CI artifact name | `ayshamusic-rust-audio-arm64` | `ayshamusic-android-audio-arm64` |
| APK filename | `app-rustAudio-release.apk` | `app-androidAudio-release.apk` |

Both: version **1.3.1-preview**, code **9**, Android **8+**, **ARM64** only. Each APK
contains its own Rust server and runs on the phone; no external Aysha server is
needed. Both can be installed alongside each other and the previous test APKs.
Their app-private Telegram sessions, queue and preferences are separate; nothing
is copied from another install. These CI install-test builds use the debug-key
fallback when release credentials are absent, not production/store signing.

Use one playback app at a time so Android audio focus does not interrupt the
comparison. Telegram setup remains optional for ordinary YouTube playback.

## Isolation and readiness

Previous APKs all used loopback port5050 and accepted any `/api/status` response.
An older running app could therefore satisfy a newer app's readiness check and
serve the wrong embedded UI. This is a code-supported explanation for an old UI,
**not an established diagnosis of the user's phone**.

New variants use distinct loopback ports. Kotlin generates a process-specific
random instance ID, passes it with engine/port to Rust at startup, and compares
all three returned fields before loading the WebView. Native HTTP controls/events
are disabled until verification. A bind collision now produces a startup failure,
not acceptance of another app's server. Settings and Now Playing identify the
audio engine and compiled source receipt. This is instance matching, not a claim
of authenticated protection against a malicious rooted/local app.

## Media3 boundary

- Rust resolves the existing audio/mp4 source and performs provider/Telegram reads.
  Provider URLs, account credentials and MTProto sessions never go to Media3.
- A random per-play token scopes a loopback media route. It supports GET/HEAD,
  normal/open/suffix byte ranges, correct Content-Length/Content-Range and bounded
  64KiB upstream requests. Compressed media is streamed, not collected in RAM.
- A dedicated bounded-channel worker owns the blocking Rust fetch client, including
  its destruction; no blocking client is dropped on a Tokio async runtime worker.
  At most two preparation/input workers may overlap; stalled replacements can
  report busy rather than spawn unbounded workers. Network reads retain deadlines.
- ExoPlayer handles audio focus, noisy/headphone disconnection and its wake lock.
  A main-thread-only Kotlin adapter handles commands and reports readiness,
  state/position/duration/error to Rust. Existing service transport controls still
  call the Rust API. The Rust variant has no Media3 dependency.
- Rust waits for Media3 STATE_READY before acknowledging preparation. This is
  **not proof of audible output**. Stale play tokens/events cannot reactivate a
  stopped/replaced Rust playback session. Stop revokes the loopback media token.
- Source input failures retain the underlying sanitized Rust range error rather
  than only ExoPlayer's generic HTTP error. Rust decoding now also retains a
  RangeReader health error when decoder construction reports generic IO failure.
- Saved-song lookup, full skipped-track archival, limits and YouTube startup
  fallback stay in Rust. Seamless mid-track failover is still not implemented.

Media3 1.8.0 was selected against its published source constants (minSdk21,
compileSdk35); this app's API floor stays26 because the shared native library
links AAudio. Build success and actual device acceptance are separate gates.

## Build and checks

`bash android-app/scripts/build_apk.sh` builds the common ARM64 native library and
both release flavors. Size checks apply separately to each APK (15MiB hard limit).
The Android workflow uploads **two separate artifacts**, not an arbitrary first
APK found in an output directory. Public main/release APKs are not overwritten.

Offline Rust fixtures cover range routing, HEAD/suffix responses, revoked/stale
tokens, readiness acknowledgement, failure propagation and existing AAC/Telegram
contracts. Node/Python contracts and Chromium native-UI fixtures cover Settings,
account-dialog access, input clearing and history/layout. These do not log into a
real account, probe a live song or establish device/background/speaker behavior.

## Known phone result and acceptance boundary

The previous **99972a3 / 1.3.0-preview** APK failed on the user's physical phone at
0:00: `audio_decode_failed: An IO error occurred while reading, writing, or seeking
the stream.` Its successful host fixture tests did not establish phone playback.
The older artifact is superseded, not a known-working fallback.

Media3 can avoid the Rust decoder/output path, but it cannot make an unavailable
or rejected upstream source work. Neither new variant can be called fixed/audible
until verified on a real phone. Real Telegram login/upload/replay, audio-focus and
screen-lock/background behavior, Android RAM, Linux service deployment and
production signing remain separate unverified gates. Do not request credentials
in chat or repeat the previously consumed live-provider experiment.

Final source/CI/artifact evidence is recorded in `docs/LOCAL_QA_LEDGER.md`. Account
instructions and data/storage cautions: `docs/TELEGRAM_SAVED_MESSAGES.md`.

## Download candidates (source e39acae)

Android run [43717901021](https://github.com/AfzalAshraf/ayshamusic/actions/runs/43717901021)
passed host Rust fixtures, ARM64/JNI, both release/R8 builds, separate size gates,
artifact uploads and Android contract checks.

- [Rust Audio ZIP](https://github.com/AfzalAshraf/ayshamusic/actions/runs/43717901021/artifacts/14163606203)
  — extract/install `app-rustAudio-release.apk`, open **Aysha Rust Audio**.
- [Android Audio ZIP](https://github.com/AfzalAshraf/ayshamusic/actions/runs/43717901021/artifacts/14163108083)
  — extract/install `app-androidAudio-release.apk`, open **Aysha Android Audio**.

Settings → Add Telegram account in either app. Setup is separate per install.
Use one playback variant at a time. ZIP metadata and limits are in the QA ledger;
these are CI candidates, not a claim of audible-device success.
