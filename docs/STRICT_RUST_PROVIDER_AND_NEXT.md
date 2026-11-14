# Standalone Rust APK: playback baseline and automatic local recommendations

## Latest follow-up: app closes on Play

The user reports an unexpected close when Play is tapped; the exact installed
build and native crash stack have not been supplied. Treat this as a separate
crash report, not just the earlier provider refusal. Prior Listening candidate
compilation is not successful phone acceptance.

Source patch `d6fd6a0` initializes CPAL's required Android JVM/application context
before starting workers, pins a global application reference for process lifetime,
and guards audio opening if initialization is missing. Follow-up `9f7f9d0` keeps
the Kotlin JNI bridge intact during release shrinking. The dependency's actual
missing-context panic combined with release `panic = "abort"` explains a concrete
crash-capable path, but is not a confirmed phone stack trace. See the latest ledger
for exact source references, verification and current candidate download.

Current fix-candidate identity: **1.2.3-preview/code7**, launcher
**Aysha Music Audio Fix**, package **com.afzal.ayshamusic.context7**. Historical
1.2.2/Listening artifact details below describe the previous candidate, not this fix.
No provider/version/codec/media-timeout/recommendation changes in this crash patch.


## Current user correction and device evidence

The user explicitly rejected the Romantic/Melody dropdown. Native Next must learn
from actual listening, skips and likes, without using the current song title as
its search seed. The approved Aysha page and dedication remain unchanged.

The latest device screenshot is **Rust APK 0d5f461**, 0:00, with ANDROID and IOS
reporting **no audio-only AAC stream offered**. That is a failure to obtain a
compatible direct media URL, before transfer or decoding. The old HTTP 400 report
was an earlier failure; it is not the latest symptom. Rust4 is a failed device
candidate, not a recommended install or a publication-ready APK.

The user chose strict Rust over NewPipeExtractor. No Java extractor,
Python/Node/yt-dlp, hosted resolver, token service or external Aysha server has been
added. Existing Kotlin/WebView UI is retained. The phone makes its own provider
requests, transfers, decodes and plays audio; normal internet/provider access is
still required. Linux server playback remains a separate, unchanged path.

## Preserve the other agent's 1.2.1 playback fix

Merge parent `7b09cdef26db88a97b03980717acd488c60e9955` supplies the playback baseline:

- Separate API client (12-second whole-request timeout) and media client
  (10-second connect / 30-second read timeout, **no whole-track timeout**).
- Persistent anonymous visitor identity, acquired and cached by that baseline.
- The baseline's Android/iOS/TV resolver, media pump and rodio FIFO decoder path.
- Direct allowlisted AAC only; provider refusal remains an error, not fake playback
  or a hidden server fallback. No new live profile/watch/auth experiments here.

The other agent's main QA ledger reports a full 3,449,447-byte transfer over 107.6 s,
AAC inspection, complete ffmpeg decode and non-silent rodio samples. The published
main 1.2.1 release and successful build metadata were inspected here. Those host
measurements were **not independently rerun** in this session and are not physical
phone speaker proof. That agent also reports bot screening for four music-catalogue
videos while one test video resolved. No claim of universal music availability.

The timeout fix only applies **after resolution**. It does not establish that the
latest screenshot's no-direct-AAC failure is resolved on the user's phone.

The old branch range/PCM modules remain source files but are not wired into the
current player. Do not cite their front/tail MP4 fixture tests as evidence for this
candidate's FIFO decoder. Earlier Rust4 request-version/terminal-400/visitor contracts
were superseded by the main baseline, not silently presented as retained features.

## Automatic Next: what is implemented

- **No mood dropdown or mood API.** Learning runs in the background. The user
  rejected the learning explanation and Reset learning button on Now Playing;
  both are removed. The internal reset API remains, without a player control.
- A Rust task samples the actual player position every five seconds. UI polling,
  searches, failed resolves and elapsed wall time do not count as listening.
  Initial samples establish a baseline; pauses and position jumps are not credited.
- Bounded profile: at most 300 tracks with title/channel metadata, played seconds,
  short skips, completions and explicit likes. A completion requires observed
  listening of at least 80% of the estimated duration. A successful manual Next or
  replacement shortly after observed playback counts as a skip; Previous does not.
- Weight per track: up to 20 points from seconds/30, +4 per completion, +8 if liked,
  -3 per short skip. Nonpositive evidence does not create a positive preference.
- Metadata cues infer broad styles (for example romantic, melody, dance,
  instrumental or devotional); otherwise a favored artist/channel supplies the
  search seed. No history means generic discovery, not a forced romantic default.
- Next searches once using the inferred style or favored channel and ranks results
  using the aggregate profile and current likes. Current/recent song IDs and
  normalized title variants are excluded. Lyrics/covers/remixes of the same title
  are filtered heuristically; this is not acoustic fingerprinting or a trained
  model, and channels do not reliably identify the true artist or genre.
- Explicit queue wins over recommendations; Previous keeps session history.
  Failed resolution does not consume the queue or replace current/history. One
  selected candidate is resolved per tap; no hidden loop across rejected songs.
  If no different candidate survives, a clear message replaces same-song replay.
- Profile is saved in app-private `listening-preferences.json` using atomic rename,
  bounded reads and mode 0600 on Unix/Android. Flush roughly every six progress
  updates, and on explicit likes/skips/stop/track changes. Force-kill can lose the
  latest unsaved samples. Android backup is disabled by the existing manifest.
- Listening logs are not uploaded. Normal YouTube search sends the inferred style
  or favored channel as a query, not the full history, seconds/skips or private
  favorite titles. Reset clears the saved model, not the queue/current favorites.
- Queue/history/favorite-list display remain process-local, not account sync.
  The learned like contribution persists in the profile; this does not claim a
  fully persistent favorites library. Natural-end autoplay was not added here;
  this change concerns automatic selection when Next is invoked.

## Candidate identity and build procedure

Source merge `efb0613`, follow-up contracts `6083719`. Version **1.2.2-preview**,
code **6**, package **com.afzal.ayshamusic.learn6**, launcher **Aysha Music Listening**.
ARM64 / Android 8+ only. A separate preview package avoids requiring removal of old
apps with a different ephemeral CI signing key. Old app profiles are not imported.

The workflow builds install-test artifacts only; it does not overwrite public
`apk-latest`, sign for production or publish to a store. A candidate build must
pass host Rust tests, Linux compile check, Android cross-compile/Gradle, size gate
and website contracts before delivery. See the latest QA ledger for the actual
run IDs/results and download link, rather than treating a queued build as success.

For maintainers (**build**, not a phone install command):

```sh
cargo test --lib --features phone
cargo check --bin music-server
# With Android SDK/NDK/JDK installed:
AYSHA_PREVIEW=1 bash android-app/scripts/build_apk.sh
```

The sandbox has no Rust/JDK/SDK toolchain; CI performs those checks. Local Node and
Python regressions do not substitute for the Android build. No live provider or
physical phone test was run in this session. Phone RAM and the Linux service RAM
were **not remeasured**.

## Acceptance boundary

No Linux server install is needed. Open the correctly named candidate, verify the
source receipt and play once. Actual acceptance requires audible sound and an
advancing timer, not just a successful API response or downloaded bytes. If the
same no-AAC/provider refusal remains, stop: the provider gate is still unresolved;
do not prescribe repeated reinstalls or blind Test/profile rotations.

Only after playback works can real listening personalize Next. Verify a different
song with an empty queue, explicit queue order, Previous and reset separately.
Production signing, actual phone/background/audio-focus behavior, resource usage
and music-provider coverage remain open gates. Do not call this publish-ready based
only on compilation, offline tests or the other agent's one-track host evidence.

## Verified candidate download

Android run **43591553488** at **6083719** passed all steps; Linux run
**43591553492** passed all five jobs. Local **91 Node / 137 Python** tests passed.

[Download the install-test artifact](https://github.com/AfzalAshraf/ayshamusic/actions/runs/43591553488/artifacts/14112747190),
extract `app-release.apk`, and open **Aysha Music Listening** after installation.
This is a preview, not a newly published production release. GitHub sign-in and
repository access may be required; artifact retention is 14 days.

GitHub-reported ZIP size 3,707,064 bytes; archive SHA256
`f544ca6056db5ab169a8cf91fce6bff1d94e2abcb8b867ff4e666ebe80d75c84`.
These are archive metadata, not phone RAM or installed size. Download from the
sandbox failed at the restricted Azure redirect; independent binary/signature and
physical-device inspection remain unperformed. No claim that green CI resolves
the user's no-AAC error or establishes audibility.

## Latest crash-fix build (supersedes the Listening install candidate)

[Download **Aysha Music Audio Fix — 1.2.3-preview**](https://github.com/AfzalAshraf/ayshamusic/actions/runs/43592475037/artifacts/14113285899).
Source receipt **9f7f9d0**, Android build **43592475037 PASS** and Linux
**43592474963 all five jobs PASS**; local **94 Node / 137 Python PASS**.
Extract `app-release.apk`. Package **com.afzal.ayshamusic.context7**, code7,
Android8+/ARM64. It installs alongside the old candidate, not over a mismatched key.

This corrects the missing Android-context initialization described at the top.
Physical-phone crash confirmation, provider access and audible playback remain
unverified. A successful build is not permission to publish it as working.
GitHub archive14113285899 is3,717,081bytes, ZIP SHA256
`7890f218b77abf7aa21a76e2a7f52efc6e6f8c525f6917a26e99ac009f3f7c38`;
not an APK-size/RAM measurement. See ledger for evidence and optional crash capture.
