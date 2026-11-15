# Media3-only source repair

## Requested scope

Latest user report is physical build11, receipt `10a46e6`:

- image10: source starts briefly, then `media3_input_failed: media_range_http_403`.
- image11: Android/iOS fail with `media_range_http_302`; TV client unsupported.

The user removed the Rust Audio APK and authorized research/GitHub build/testing.
Keep Rust source acquisition and Telegram; only audio decoding/output uses Media3.
Keep image9 launcher, UI, account setup and exact dedication. Do not introduce an
unapproved Java extractor, yt-dlp/Python/Node runtime or hosted resolver. GitHub
permission does not connect the user's physical phone to this workspace.

## Findings and implemented corrections

1. `HttpFetch` disabled all redirects. A valid CDN handoff therefore failed as302.
   Follow at most3 redirects, validate HTTPS/googlevideo/no-credentials/443 BEFORE
   each request, preserve Range/UA/identity and exact signed query, bound the whole
   operation to12s, reject loops/missing Location/non-CDN/downgrade, do not retry403.
2. Media3 preparation downloaded and discarded the first64KiB, then fetched it
   again. `PreparedFetch` caches first and tail/index64KiB each (128KiB maximum),
   checks total/length, blocks Play until both can be read and serves cached subsets.
   Uncached reads still validate total/length and propagate403 rather than hiding it.
3. Rust Audio flavor/upload/stub removed. Release build explicitly assembles only
   `AndroidAudioRelease` and checks ARM64 `libayshamusic.so` is present. The Linux
   Rust player and the shared Rust source/Telegram implementation are retained.
4. Build12,1.3.4-preview, Aysha Android Audio12; existing androidaudio10 package and
   port17652 retained. Existing ephemeral debug-signing caveat still applies.
5. Actual Kotlin PlatformAudio Android instrumentation added with generated local
   AAC fixture: continuity beyond2s, pause/resume/end, pause-before-ready, initial
   source403 reports error. Emulator fixture mode excludes JNI/provider acquisition
   and refuses Release tasks. It is not a physical-speaker/YouTube acceptance test.

## Research before code

Reviewed through GitHub (references, NOT runtime dependencies):

- https://github.com/TeamNewPipe/NewPipeExtractor/blob/dev/extractor/src/main/java/org/schabi/newpipe/extractor/services/youtube/extractors/YoutubeStreamExtractor.java
  `buildAndAddItagInfoToList` ignores missing URL/cipher and requires n-parameter
  transformation; comments warn untransformed n can return403.
- https://github.com/yt-dlp/yt-dlp/blob/master/yt_dlp/extractor/youtube/_video.py
  Missing URL can indicate SABR-only streaming; advertised AAC metadata is not
  equivalent to a downloadable source.
- https://github.com/yt-dlp/yt-dlp/blob/master/yt_dlp/extractor/youtube/_base.py
  Some VR HTTPS configurations have mandatory token policies; previous PR62
  compilation/metadata output was not a valid working-player proof.

These explain why generic decoder swaps/header guesses are not a demonstrated
403 fix. They do not prove which requirement affected this particular phone.
No provider sign-in/bot/access-control bypass or account credentials were used.

## Test and live-proof boundaries

Routine regressions and browser/emulator fixture work are offline for providers.
A separate exact-parent/first-attempt source job uses the SAME production resolver,
HttpFetch and PreparedFetch with a fixed public video, at most16HTTPS requests,
8MiB total encrypted bidirectional transport and a60s hard external kill. It reads
first/tail and spaced later blocks instead of just accepting metadata. No full
media download, source URLs/provider HTML/JS/account data artifacts or new clients.
A successful bounded source test would still not prove complete playback, UAE
network behavior, screen lock/background/focus or physical-phone audibility.

## CI attempts (append actual outcomes; do not infer success)

- Source64e40b7, Android43743061359: build-apk132900245228 SUCCESS, Media3-only
  artifact produced. Source-proof132900244986 failed **before live execution**:
  existing Cargo.lock needs resolution and `--locked` blocked the offline build.
  Live allowance not consumed by that attempt.
- Emulator132900245249 failed at redundant SDK setup on Ubuntu24.04 before boot or
  instrumentation. No emulator pass claimed. Remote logs redirect to an unavailable
  blob host; no unchanged download retry. Next attempt uses the runner's SDK (as
  emulator-runner normally does), not a second competing SDK bootstrap.
- Linux43743061385: five jobs PASS, browser failed its obsolete assertion expecting
  two flavors/ports. Updated to the single retained Media3 port, not weakened port
  safety. Local browser check is rerunning; no production UI was changed.

Initial local regressions105Node/142Python PASS. Rust phone/source fixture tests
passed inside the Android build; actual source and emulator results still pending.

## Skills and project memory

User-requested all314 ECC skills plus620support files are pinned under vendor/ecc,
with license, file hashes and index. Relevant four reviewed; no hooks/installers/
observers executed and nothing added to APK resources/dependencies. See
skills/source-playback-debugging/SKILL.md for durable, evidence-backed lessons.
This is repository guidance, not permanent model learning or a playback fix.
