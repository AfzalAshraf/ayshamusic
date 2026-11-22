# On-device extractor evaluation — 2026-11-07

## Scope decision

The user selected **Allow an on-device Android extractor** after the exact-target
CI source investigation44121609464 stopped at an explicit provider sign-in/bot
challenge. This supersedes the previous Rust-only *extraction* requirement only:
Java/Kotlin extraction is now permitted if validated. Media3 remains the sole
Android playback engine; Rust retains Telegram/backend. Everything stays inside
the APK. No external app, hosted resolver, Python/Node runtime, credential or
bot-check bypass is authorized. No further provider request was made for this
evaluation, and the earlier CI challenge must not be retried through a succession
of client identities.

## Candidate: NewPipe Extractor

Reviewed **v0.27.0**, commit
`7d4d691846acfa3c5af4b2c124955ee4c11cd3e8`, published 2026-11-07.
The API's latest release response points at v0.27.1 dated 2026-11-10, which is
later than this session's date; it is NOT the selected reference.

Sources (retrieved with GitHub API/source archive; reference, not execution):

- https://github.com/TeamNewPipe/NewPipeExtractor/releases/tag/v0.27.0
- https://github.com/TeamNewPipe/NewPipeExtractor/tree/7d4d691846acfa3c5af4b2c124955ee4c11cd3e8
- `LICENSE`, `README.md`, root/extractor Gradle files and `gradle/libs.versions.toml`
- `YoutubeStreamExtractor.java`, `YoutubeStreamHelper.java`,
  `downloader/{Downloader,Request,Response}.java`, `NewPipe.java`

### Findings

- Pure JVM extraction library, used by Android NewPipe; no hosted resolver or
  Python/Node executable is required. Its embedded Rhino JS engine is Java, not
  Node. This is compatible with the revised architectural scope, subject to
  licensing and actual tests.
- This version's stream extractor calls a VISIONOS player path first, then WEB
  metadata and next-page metadata. It is NOT a claim that the previous Android/
  iOS profiles become valid merely by changing languages.
- The VisionOS path obtains normal anonymous visitor data from Innertube. The
  primary stream path checks playability and throws `SignInConfirmNotBotException`
  for a matching LOGIN_REQUIRED reason before fetching WEB metadata. Other
  failures include unavailable, private, paid, country and age restrictions.
  The surrounding downloader must preserve stop semantics, including if a
  secondary optional metadata request swallows an exception internally.
- No token provider, login cookie import, proxy/identity rotation or provider
  challenge workaround is proposed. Signing/deciphering public media metadata is
  different from evading an explicit denial; the latter remains out of scope.
- Gradle library compiles for Java17. Android minSdk26 needs **core library
  desugaring**; upstream README specifies `desugar_jdk_libs_nio` below API33.
  Rhino needs upstream's keep rules. Both affect size/R8/runtime validation.
- Dependency graph includes NewPipe nanojson pinned
  `e9d656ddb49a412a5a0a5d5ef20ca7ef09549996` (JitPack), jsoup1.23.2,
  protobuf-javalite4.36.2, Rhino1.8.1 and jsr3053.0.2. No dynamic version or
  unverified prebuilt binary should be introduced. Sandbox egress excludes
  JitPack/Maven, so a source-pinned CI build is needed for dependency/Android tests;
  none has yet run for this candidate.

## Licensing gate — rejected by owner

**Decision:** user selected `avoid_gpl`: no app relicensing and no NewPipe linking.
The proposal below is historical; NewPipe is excluded. Follow-up permissive
research lives in `prototypes/youtubejs-browser/README.md`; it is isolated,
offline-only and not an APK dependency.


NewPipe Extractor is **GPL-3.0-or-later** (upstream LICENSE and Gradle POM).
The Aysha repository has no root LICENSE. Linking/distributing it in an APK
would generally require distributing the combined work under GPL-compatible
terms, retaining notices, and providing complete corresponding source and build
instructions. A public GitHub repository without a license is not by itself a
GPL licensing grant or complete source-compliance plan. This is a consequential
project decision, not an automatic consequence of permission to use Java/Kotlin.
Ask the owner before changing licensing or integrating/distributing the library.
Existing third-party notices/licenses must remain separate and intact.

## Proposed gated integration, not implemented

1. Confirm licensing choice. If GPL-compatible release is not acceptable, do not
   link NewPipe; evaluate a differently licensed on-device alternative or retain
   Rust and clearly state source limitations. Do not copy GPL implementation code
   into Rust to pretend the license disappears.
2. Build a pinned isolated adapter with a bounded downloader. HTTPS metadata
   endpoints only; validate every redirect/host/method, byte/request/deadline
   budgets, no cookies/credentials, safe error codes, terminal challenge latch,
   cancellable execution off UI thread. Disable raw upstream logger/URL leakage.
3. Add JNI handoff for video-ID-in / validated direct-audio-descriptor-out with
   cancellation/generation tokens. No signed URLs in WebView/status/logs. Rust
   should still validate media host, length and **head plus tail**, and keep its
   immutable signed URL per range. Do not bypass preparation validation.
4. Initially restrict media selection to direct HTTPS audio/mp4 supported by the
   existing Rust range and Telegram path. Explicitly reject HLS/DASH manifests,
   video-only streams, unknown length and non-URL descriptors until implemented.
5. Fixture tests: exact video mismatch, restricted/challenge responses (terminal,
   no subsequent requests), malformed/oversized metadata, unsafe redirects,
   cancellation/stale result, safe serialization, AAC selection, head206/tail403.
   Then actual minSdk/R8 Android integration and Media3 playback, preserving
   Telegram fallback/archive behavior, settings/design/icon/dedication.
6. Source validation must be a separately bounded, justified test and must stop
   on challenges. The previous CI source allowance is sealed. Neither fixture
   tests nor a new library name prove public source access on the user's phone.
   Do not resend build13 or a synthetic-tone-only APK as a song-playback fix.

## Evidence state

- Candidate source/licensing review: **done**, references pinned above.
- NewPipe compile, fixture tests, dependency resolution, Android/R8/APK size: **not run**.
- Exact affected song source access / complete playback via candidate: **not tested**.
- Physical-device/network, Telegram, background/focus and RAM: **not tested**.
- Production APK/extractor/UI: **unchanged** by this evaluation.
