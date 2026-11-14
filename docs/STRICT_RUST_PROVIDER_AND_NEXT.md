# Standalone Rust APK: current playback and recommendations

## Latest device evidence and scope

The latest supplied screenshot identifies **9f7f9d0** and reports:

> audio decode failed: The format of the data has not been recognized.

It shows an open player, but does not prove all crashes are fixed or any sound is
playing. It supersedes the earlier Rust4 `0d5f461` no-audio-only-AAC screenshot as
the latest symptom. Earlier HTTP400 was a different, older failure again.

The user rejected both the Romantic/Melody dropdown and the Reset learning block
on Now Playing. Those are absent. Automatic listening preferences remain in the
background. Errors remain visible, not hidden to make the player look successful.

The latest user additionally approved personal Telegram Saved Messages storage,
automatic saving even after skips, Telegram-first replay with YouTube fallback,
and guided in-app developer-credential setup. See `TELEGRAM_SAVED_MESSAGES.md`.
This is a first-party media source, not a hosted Aysha resolver/helper.

## Current Rust playback correction

The main1.2.1 integration retained its **FIFO/nonseekable decoder input**, leaving
the branch range/PCM modules unwired. Main's separate API/media-client timeout fix
was valuable, but did not solve the FIFO's MP4-index seeking limitation.

The current candidate reconnects `range_media` and `pcm_stream`:

- MP4/AAC decoder gets a real bounded `Read + Seek` range reader, known byte length
  and `with_seekable(true)`. Its index can be at the front or tail of the file.
- Real AAC fixtures are generated in CI for both layouts. The regression reproduces
  failure of a nonseekable input on the tail-index file and verifies non-silent
  decoding through the corrected reader. This reproduces a concrete cause of
  format-recognition failure; it does not prove the unseen phone response bytes
  or the exact selected song have been independently examined here.
- YouTube input uses validated HTTP206 byte ranges (64KiB blocks), with strict
  Content-Range/length/encoding/allowed-host checks. A per-range12second request
  budget is not a whole-track timeout. No multi-minute transfer is capped at12s.
- Decoding/network reads run on dedicated Rust threads, not the real-time audio
  callback. Bounded ready-PCM buffering yields silence during starvation instead
  of blocking device callbacks; only consumed music samples advance position.
- A full PCM queue can be canceled without leaving its decoder worker blocked.
  Generation checks prevent stale decode results overwriting newer state.
- An API success waits for decoder/output preparation, rather than reporting
  successful playback immediately after finding a URL. Hardware audibility remains
  a separate gate. Stop invalidates a pending source handoff, including resolution.
- Telegram input implements the same range-fetch interface using aligned MTProto
  chunks. It uses the same AAC decoder, not a Java player or downloaded-file-only
  imitation of streaming. Startup Telegram failure falls back to YouTube.
- The Android JVM/application-context initialization and process-lifetime GlobalRef
  from `d6fd6a0`/`9f7f9d0` are retained. R8 preserves the Kotlin JNI bridge. No attempt
  to catch a release abort as if it were a recoverable decoder error.

Main parent `7b09cdef26db88a97b03980717acd488c60e9955` remains an ancestor. Its API
client still has a12second total request timeout; the full-file media client has
connect10/read30seconds and **no whole-transfer timeout**. The archive's complete
YouTube download uses that client; a separate15minute operation budget bounds
archive work. The old main FIFO pump is retained only for opt-in diagnostics,
not native playback. Do not conflate its host/Cursor decode evidence with this path.

The main resolver/persistent anonymous visitor remain. There were no new live
watch/client-version/auth/proof-token experiments, helper integrations, bypasses or
codec-quality downgrades. Direct allowlisted AAC is still required from YouTube.
Provider refusal is an error; seekable decoding cannot manufacture a media URL.

## Automatic local Next (no visible learning controls)

- Five-second Rust sampling of the actual player position, not UI polling or wall
  time. Baseline samples, failed resolves, pauses and position jumps are not plays.
- Up to300 track profiles, with bounded title/channel metadata, played seconds,
  short skips, completions and explicit likes. A completion requires observed
  listening of at least80% of estimated duration. Previous is not a short skip.
- Weight: up to20points from seconds/30, +4/completion, +8/like, -3/short skip.
  Nonpositive weights do not create positive preferences.
- Metadata cues infer broad styles; otherwise a favored artist/channel is used.
  Empty history gives generic discovery, not a forced Romantic default. This is a
  transparent metadata heuristic, not acoustic fingerprinting or a trained model;
  channel identity may not identify the real artist or genre.
- Explicit queue first; otherwise one bounded aggregate-preference search, never
  the current title+'similar'. Filter current/recent IDs and same-title alternate
  uploads, covers, remixes, lyrics or slowed versions heuristically. One selected
  candidate is resolved per tap; no blind loop across provider-rejected songs.
- Failed play does not consume queue/replace history. Previous remains session
  history. Natural-end autoplay was not added as part of taste-learning changes.
- Private atomic `listening-preferences.json`, maximum300 entries/1MiB reads,
  Unix0600, periodic and explicit-action flushes. A force-kill can lose recent
  unsaved samples. No model/log upload; normal YouTube search includes the inferred
  style/channel. Telegram song saving is the separately approved content upload,
  not an upload of listening durations/skips or the recommendation model.
- Queue/favorite-list/history display remain process-local; learned like influence
  persists in the profile. No account-sync claim. The internal reset API remains
  but no Reset learning control/explanation is placed on the player or elsewhere.

## Build, delivery and acceptance

Current candidate: **1.3.0-preview/code8**, package
**com.afzal.ayshamusic.telegram8**, launcher **Aysha Music Telegram**,
Android8+/ARM64 only. Separate preview package preserves old apps/data despite CI
signing-key changes. It does not import old app sessions/profiles.

Android CI checks host Rust with real AAC fixtures, Linux binary compilation,
Android cross-compilation/JNI/Gradle/R8, size budget, artifact upload and contracts.
Local Node/Python tests and Linux/browser CI are separate evidence. See the latest
QA ledger for exact successful source/run/artifact IDs before offering an APK.
No branch build overwrites public `apk-latest` or publishes to a store.

For maintainers (**build**, not a phone install requirement):

```sh
cargo test --lib --features phone
cargo check --bin music-server
AYSHA_PREVIEW=1 bash android-app/scripts/build_apk.sh
```

Real decoder fixture tests require `AYSHA_MEDIA_FIXTURES` pointing to generated
`front.m4a` and `tail.m4a`, as set in Android CI. The sandbox has no Rust/JDK/SDK;
CI does that compilation. No live YouTube or real Telegram account is used here.

The APK needs no external Aysha server. Phone acceptance still means remaining
open, actual audible output, advancing time, correct queue/Previous and real
Telegram setup/save/replay/fallback on the user's network. Initial Telegram
connection/search is bounded; a mid-track Telegram read error is surfaced rather
than a claimed seamless source handover. Physical audio, provider coverage,
background/audio-focus behavior, phone RAM and production signing remain open.
No claim of below50MB Android whole-process memory; Linux RAM not remeasured and
Linux source/service remain untouched by these phone changes.

## Verified current candidate download

[**Aysha Music Telegram — 1.3.0-preview**](https://github.com/AfzalAshraf/ayshamusic/actions/runs/43594634691/artifacts/14113683577)

Source **99972a3**, Android **43594634691 PASS**, Linux **43594634703 all five jobs
PASS**, final local **98 Node / 137 Python PASS**. Extract `app-release.apk`.
The decoder correction, removal of player learning clutter and approved Telegram
flow are all in this artifact; earlier context7/Listening APKs do not contain them.

GitHub archive14113683577:4,099,582bytes, ZIP SHA256
`f3c4aad1298836550b87a4b6076e412d123053cd8ed71d0204a51f0fcd43d69d`.
Sign-in/repository access may be needed; retention14days. Artifact redirect download
is blocked in this sandbox, so local binary/signature inspection was not performed.
Phone audio and real Telegram end-to-end behavior are still not independently
verified. A tested source correction is not a guarantee of a fully working release.
