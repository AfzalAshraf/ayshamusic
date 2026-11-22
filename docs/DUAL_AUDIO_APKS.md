# Rust Audio and Android Audio install-test builds (historical dual delivery)

> **Latest user decision: Media3-only.** Rust Audio flavor/build/upload removed
> starting build12. See [the current repair/evidence](ANDROID_MEDIA3_REPAIR.md).
> The dual-APK identities and links below are historical, not current delivery.

> **Build11:** user-requested APKs with the supplied black/silver launcher icon
> and verified Telegram save feedback. Not a proven playback fix: installed
> code10 failed on source403 and the isolated fuller extractor still returned
> empty AAC URLs. No replacement extractor was promoted into either APK.

## Build11 source and artifacts

### Latest sealed build (recommended download for icon/Telegram update only)

Source**10a46e6**, [Android43740368278](https://github.com/AfzalAshraf/ayshamusic/actions/runs/43740368278)
**SUCCESS**; Linux43740368277 **SUCCESS all six jobs**. No live source job ran;
renewed permission was already consumed and disabled. Both APKs remain build11:

- [Rust Audio11 ZIP](https://github.com/AfzalAshraf/ayshamusic/actions/runs/43740368278/artifacts/14172287438)
  —4,451,276bytes; ZIP SHA256a1acbbda37959bfa6f794335ac395b7f79a86fcabc189004271a259af776bc7c.
- [Android Audio11 ZIP](https://github.com/AfzalAshraf/ayshamusic/actions/runs/43740368278/artifacts/14172127833)
  —4,965,527bytes; ZIP SHA256e942420cfa2defd75d9185b9a3706618eaaa736a1f6a34ae88e567c85df07dde.

GitHub artifact metadata verified; no claim of local binary inspection, valid
production signing, restored YouTube playback or physical-phone acceptance.
Local final regressions:105Node and140Python PASS (includes3 launcher tests).

### Earlier image9 experiment build (historical)

Source938b364, Android run43739826775: **build-apk job132890296348 SUCCESS**,
both ARM64 APKs uploaded and size gates passed. The overall workflow is red because
the separate one-off source research job correctly failed its validation gate.
Linux43739826787: **SUCCESS, all six jobs**. Metadata-verified build11 downloads:

- [Rust Audio11 ZIP](https://github.com/AfzalAshraf/ayshamusic/actions/runs/43739826775/artifacts/14171603178)
  (4,451,280bytes; SHA2560fff9b4dfd03858e3c6d9521bc0056996db812367b947e0df95afd30869346f1).
- [Android Audio11 ZIP](https://github.com/AfzalAshraf/ayshamusic/actions/runs/43739826775/artifacts/14171109380)
  (4,965,533bytes; SHA25619bd03f4401895061e2d648cbf40ac56e7a75647e6fac88bfaa27a92b706f19d).

ZIPs contain the respective APK; use the matching engine. These are not a
recommendation to repeat unsuccessful phone audio diagnostics. The user explicitly
requested fresh APKs despite the source blocker. No device acceptance claimed.
GitHub artifact binary contents have not been re-downloaded/inspected locally.

## Approved scope

Two standalone Android APKs, same embedded page, Rust YouTube resolver/catalogue,
Rust Telegram Saved Messages implementation and automatic learning. No hosted
resolver, yt-dlp, Python/Node runtime, NewPipe or replacement extractor added.
Only the **Android Audio** variant uses AndroidX Media3/ExoPlayer for decoding and
audio output (explicitly approved). The **Rust Audio** variant retains rodio/CPAL.

Both now use the user's **black/silver metallic AYSHA artwork** for the launcher
only (cropped from image9, not AI-redrawn). Adaptive/circular masks preserve the
logo and AYSHA text. The player Play button, Settings account flow, approved
player design and dedication remain unchanged. See `android-app/artwork/README.md`.

## Install identities

| | Rust Audio | Android Audio |
|---|---|---|
| Launcher label | Aysha Rust Audio 11 | Aysha Android Audio 11 |
| Application ID | `com.afzal.ayshamusic.rustaudio10` | `com.afzal.ayshamusic.androidaudio10` |
| Local API port | 17651 | 17652 |
| Audio engine | Rust rodio/CPAL | AndroidX Media3 ExoPlayer 1.8.0 |
| CI artifact name | `ayshamusic-rust-audio-arm64` | `ayshamusic-android-audio-arm64` |
| APK filename | `app-rustAudio-release.apk` | `app-androidAudio-release.apk` |

Both: version **1.3.3-preview**, code **11**, Android **8+**, **ARM64** only. Each APK
contains its own Rust server and runs on the phone; no external Aysha server is
needed. Both can be installed alongside each other and older *different-package* APKs.
Build11 deliberately retains code10 package IDs; it is NOT side-by-side with the
same-engine code10 APK. CI fallback signing keys may differ between runs, which
can prevent an in-place update. Uninstalling erases that app's private data; do not
present uninstall/reinstall as preserving Telegram credentials or sessions.
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

## Code10 correction: physical phone report and actual browser evidence

The e39acae/code9 candidates are **superseded**, not fixed-phone builds. The user
reported Media3 sometimes audible but unable to pause, and supplied two screenshots
with `media3_input_failed: media_range_http_403`; Rust Audio's local page failed.

A concrete startup error was introduced by assigning Rust port5061: Chromium
blocks5061 (SIPS) as unsafe even when a local HTTP server is healthy. We reproduced
`ERR_UNSAFE_PORT` against a real listening server in Chromium, with zero requests
reaching the server. The configured code10 ports17651/17652 both returned200 in
actual Chromium navigation. `127.0.0.1` always means this same phone/process host,
not a remote Aysha deployment. Kotlin's status check succeeding had not proved
WebView could navigate to that port. Main-frame local failures now show an in-app
explanation/retry instead of only Chromium's raw error page.

Other corrections:

- New selection increments a generation ticket and stops existing actual output
  **before** resolving the replacement. Old resolve/timeout futures cannot stop a
  newer selection or acknowledge a late success.
- Media3 replacement and input failure explicitly release the old/current output
  and revoke its media token, including failure before any new Play command.
- Pause/resume commands carry revisions. Delayed older playing/error callbacks
  cannot overwrite a more recent pause; paused buffering is reported as paused.
- Intermediate profile failures during preparation are not rendered as a final
  UI error over recovered playback. A second tap while preparing cancels instead
  of starting another concurrent request.
- Existing Rust client profiles are tried through **actual source + decoder/output
  preparation**, not only metadata resolution. A rejected403 source can fall
  through to the next existing profile. The selected profile's User-Agent is
  retained for media reads, including Telegram archival's HTTP read. No source
  bypass, replacement extractor, new client identity or external resolver added.
- Source attempts have a shared60second deadline (individual native preparation
  cap45s), at most three existing profiles, and stop on cancellation/deadline.
  If all providers reject access, playback fails honestly and output is stopped;
  a pure-Rust ExoPlayer rewrite could not make a403 URL authorized.

Rust Audio remains the primary Rust implementation: Rust resolution, Telegram,
range IO, Symphonia decoding, bounded PCM and rodio/CPAL output inside the APK.
The Media3 variant is retained for comparison, not described as a new Rust clone
or feature-parity implementation of the entire Media3 project.

Code10 uses separate install identities and launcher labels ending **10** to avoid
confusion with older apps and ephemeral CI signing keys. Close/stop older code9
apps before comparing; the new package cannot control another package's audio.
Each install retains separate account data; no session migration is claimed.

Final code10 artifact links and CI gates are recorded below and in
`docs/LOCAL_QA_LEDGER.md`. Phone sound/pause/Telegram acceptance remains a distinct
gate; offline/host fixtures do not prove a universal fix for YouTube rejections.

## Download code10 — source2267dba

| Install | GitHub artifact ZIP | Extract and install |
|---|---|---|
| **Aysha Rust Audio 10** (Rust decode/output) | [Rust Audio](https://github.com/AfzalAshraf/ayshamusic/actions/runs/43734169086/artifacts/14169379011) | `app-rustAudio-release.apk` |
| **Aysha Android Audio 10** (Media3 comparison) | [Android Audio](https://github.com/AfzalAshraf/ayshamusic/actions/runs/43734169086/artifacts/14170310564) | `app-androidAudio-release.apk` |

Both version1.3.2-preview/code10, Android8+/ARM64; all runtime logic is contained
inside each APK. Normal internet access to music/Telegram services is still
required. No separate server installation or remote Aysha helper is used.
GitHub access/sign-in may be needed; CI artifacts expire after14days.

[Android43734169086](https://github.com/AfzalAshraf/ayshamusic/actions/runs/43734169086)
**SUCCESS**, all steps including host phone Rust fixtures, native/JNI, both release
flavors/R8, size gates, uploads and Android contracts.
[Linux43734169078](https://github.com/AfzalAshraf/ayshamusic/actions/runs/43734169078)
**SUCCESS, all five jobs**, including full browser/actual-port checks and existing
Rust/server/prototype checks. Local102Node/137Python checks passed.

Open the **10** launcher, preferably Rust Audio first. Stop/close older versions
before comparing so a different installed package does not keep playing its own
audio. Settings → Add Telegram account remains available with separate app data.
These gates verify implementation/build/fixtures, not a physical-phone guarantee
that every source403 is resolved, pause/background is accepted, or an account
has logged in/uploaded/replayed real media.
