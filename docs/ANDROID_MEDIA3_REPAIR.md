# Media3-only source repair

> **Latest installable candidate: [build14 source repair](BUILD14_SOURCE_CANDIDATE.md)**,
> source2928821. New single-profile acquisition path; head/tail checks retained.
> Build13 failed physical-phone tail403 acceptance. Build14 has passing build/
> offline component tests, but affected-song/physical-phone acceptance is OPEN.
> The build12 download and evidence below are historical.

## Historical download — sealed build12

**[Android/Media3 Audio12 ZIP](https://github.com/AfzalAshraf/ayshamusic/actions/runs/43744233478/artifacts/14173776736)**

Source**6a17353**, version1.3.4-preview/code12, ARM64/Android8+. ZIP4,968,212bytes,
SHA256bdb7ac90e3347da51cb564def5a3297a3a47b7e3ac2028985283a8518015f069.
Metadata verified; no claim of local APK byte inspection/production signing.
Only one Android Audio artifact; Rust Audio delivery removed. Same supplied icon,
Settings/Telegram receipt correction, design and dedication retained.

Android43744233478 **SUCCESS both jobs** (APK132903895012, emulator132903894729),
Linux43744233521 **SUCCESS all six jobs**. All3actual Media3 instrumentation tests
passed again. Report artifact14174000304 (ZIP SHA256e2a54d8e71d19561f0444e13e6116324d115a8fc7618b6697133a9e27f32700e).
No additional live source run after the successful proof below; production resolver/
range/preflight and Kotlin bridge code are unchanged since that proof.

**Acceptance limit:** the screenshot tracks and user's physical phone/network have
not been retested here. A CI public-video range pass and emulator fixture pass do
not prove every403 resolved. This is a tested corrective build, not whole-app or
phone certification. Retains androidaudio10 package; debug-key signing changes
may reject an update. Uninstalling deletes local Telegram settings/session; do not
prescribe it blindly or ask for credentials in chat.

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
  safety. Local seven-viewport browser run subsequently PASS; no production UI changed.

Initial local regressions105Node/142Python PASS. Rust phone/source fixture tests
passed inside the Android build; actual source and emulator results still pending.

## Skills and project memory

User-requested all314 ECC skills plus620support files are pinned under vendor/ecc,
with license, file hashes and index. Relevant four reviewed; no hooks/installers/
observers executed and nothing added to APK resources/dependencies. See
skills/source-playback-debugging/SKILL.md for durable, evidence-backed lessons.
This is repository guidance, not permanent model learning or a playback fix.


## Verified repair gates — source df47e72

[Android run43743729439](https://github.com/AfzalAshraf/ayshamusic/actions/runs/43743729439)
**SUCCESS all three jobs**; Linux43743729437 **SUCCESS all six jobs**.

### Actual production-path live source proof

Job132902322957 **PASS**, one fixed public video, ANDROID/itag140/AAC, total
3,449,447bytes. First/tail plus2spaced later reads passed strict byte contracts.
**6HTTPS requests /493,636 transport bytes reserved-or-used /420ms**. Includes
fresh visitor metadata and source acquisition, not merely the extractor metadata.
No sign-in/bot/access bypass, credentials or raw source URLs used/retained. Result
is `research/media3-production-source-result.json`. Proof artifact14173701105,
ZIP SHA256069393b0d737e205477db30219ce8523e5353a02641d3fc828b08b70868ba91a.
This proves one CI source path can supply ranges; it is not the screenshot videos,
UAE/phone-network acceptance, sustained complete playback or a before/after proof
that all403s were caused by the corrected redirect/cache defects.

### Installed Android emulator audio-bridge checks

Job132902323164 **PASS**, API29/x86_64, actual production PlatformAudio+Media3:

- `actualMedia3ContinuesPastTwoSecondsPausesResumesAndEnds` **PASS**.
- `pauseBeforeReadyIsHonoredThenResumeActuallyPlays` **PASS**.
- `rejectedInputReportsErrorInsteadOfFalsePlaying` **PASS**.

These installed debug/instrumentation APKs use generated AAC and local HTTP;
JNI/provider backend is intentionally excluded. Report artifact14172808696,
ZIP SHA256403f3fae713b3d1931911f0d7c77f596a8b66bde9a08d148f766f7f23f1e2de7.
No physical-speaker or user's phone-install claim.

### Release APK and reproducibility

Build job132902323255 **PASS**, ARM64 Media3-only artifact14173548053,
4,968,213ZIPbytes, SHA256094c44c227bf0089596316ddc6c460c2108fcbe21ecfa6cd1b35cf74d140b742.
No Rust Audio artifact generated. Release-native/size/host-range tests PASS.
GitHub metadata and annotations verified; no inaccessible blob-download retry or
local APK byte-inspection claim.

Cargo-generated lock delta from the successful proof is exactly two dependency
edges (futures-channel→futures-sink, reqwest→futures-channel), no version changes.
Applied that exact CI diff, restored --locked for production checks/build. The
live proof entry was removed and its CLI hard-sealed after consumption. Later
routine pushes cannot repeat it. Final local106Node/142Python PASS; last full
local Chromium seven-viewports plus six native Settings fixtures PASS.

Physical build12 playback acceptance, background/focus, real Telegram login/
upload/replay and production signing remain OPEN. Source availability may still
vary per video/session/network. QA evidence rating3/5, not whole-app certification.
