# Standalone Android APK inspection

**Historical inspection:** the user subsequently reproduced missing icons and HTTP404.
The Android code has now been integrated and those API contracts patched; see
[ANDROID_APK_FIX.md](ANDROID_APK_FIX.md) and the latest QA ledger for build evidence.
The gaps below describe the inspected main artifact, not a claim that fixes are absent.

## Requested outcome

An installable Android app preserving the approved Aysha website design, streaming
on the phone without the user's Linux machine, Vercel deployment or hosted Aysha
backend. Preserve current working Linux playback separately. Internet access to the
music provider is still required. An embedded phone-local runtime is not an external
server; a strict prohibition on even an in-app HTTP listener would require replacing
the existing loopback bridge.

## Sources actually inspected

- Current session branch at eca0265: Linux/browser implementation; no Android app.
- Fetched origin/main at bf37384b1db38730a1d0608365b81a6376d7a13f, following
  f4cb1a4 and d523a4b: Android app, Rust JNI library and Android build workflow exist.
- GitHub Android APK run43584788099: success. This is build evidence, not a physical
  Android audio test. No live provider request or host installation performed.
- GitHub release apk-latest identifies bf37384, asset app-release.apk size7,108,929
  bytes (7.11decimalMB), explicitly debug-signed and device playback unverified.
- Download attempted with gh release download; asset redirect host is outside this
  workspace's network allowlist and returned EOF. APK bytes/signature were therefore
  NOT independently inspected. Metadata size is not on-device RAM.

## Existing main architecture

android-app embeds libayshamusic.so via RustBridge, starts a Rust runtime on the
phone at127.0.0.1:5050, and shows the embedded src/index.html in a WebView. This is
self-contained with respect to Aysha hosting; it does not connect to the Linux host.
The Rust library resolves YouTube media directly, streams through bounded channels,
and uses rodio/cpal for device audio. Search/queue/likes are NOT completed by merely
including the page. Telegram authentication/cache modules also exist and require
separate privacy/product review; their existence does not establish working sync.

## Confirmed source-level gaps before 'working website clone' / publication

1. src/lib.rs router has no /api/search, /api/queue, /api/queue_add,
   /api/queue_clear, /api/liked, /api/like, /api/unlike, /api/next, /api/prev,
   or /icons.svg. The shipped page still references these routes. Initial catalogue,
   typed search, icons, library and queue controls therefore have missing contracts.
2. Android foreground-service/headset notification Next/Previous call missing
   /api/next and /api/prev. Compilation cannot validate these interactions.
3. MainActivity immediately loads the page after requesting asynchronous Rust boot;
   there is no readiness wait/retry screen for startup bind/readiness failure.
4. WebView navigation uses prefix matching, not exact scheme/host/port validation;
   mixed content is always allowed, permission requests are granted wholesale,
   and third-party cookies enabled. Loopback APIs lack an app/session token.
   Harden the bridge/navigation/permissions before untrusted/public distribution.
5. Release signing silently falls back to a debug key, versionCode is1, and workflow
   publishes apk-latest even on non-PR arena branch pushes. Separate preview artifacts
   from intentional production release, fail closed without a release key, preserve
   signing identity across updates, and do not put keys/passwords in chat or Git.
6. ARM64-only/minSdk26: compatible devices Android8+ only; no32bit/x86 guarantee.
   Physical playback, background/screen lock, notification controls, audio focus,
   Android target-SDK service restrictions and process recovery need real tests.
7. Direct YouTube client-profile fallback does not prove provider reliability,
   authorized catalogue usage or store compliance. Confirm distribution channel,
   music rights/provider terms and appropriate privacy disclosures before publishing.
8. No measured phone app/WebView total RAM exists; Linux service readings do not
   apply to Android, and ring-buffer size alone is not total process memory.

## Proposed integration contract

Active department: Product & Architecture. Preserve approved rose/dark UI, icons,
quotes and exact dedication; preserve existing Linux deployment. Bring Android work
onto the current session branch without switching branch or discarding newer Linux
fixes; reconcile shared Cargo/UI contracts deliberately, not a blind file overwrite.
First complete missing Android API/UI contracts and deterministic offline tests;
then harden startup/navigation, notifications and packaging. Build a clearly marked
install-test APK through CI. Device-test with the Linux server off, then create a
properly signed production release (and AAB if Google Play is the chosen channel).
Do not overwrite the public latest release from an unapproved preview build.

Acceptance: cold launch with external Aysha server off, populated real search,
select/play/pause/stop/next/previous/queue/likes, preserved design/icons/navigation,
background/lock-screen/headset and network-change recovery, honest errors on provider
refusal, no hidden off-host Aysha service, appropriate signing/versioning/disclosures,
and measured phone RAM. Automated compilation alone meets none of the audio gates.

Pending consequential choice: Google Play publication (release AAB/store requirements)
versus direct APK distribution (signed APK). No new provider probing, signing key,
production publication, main merge or Android device success is claimed by this audit.
Readiness: Android source architecture2/5; standalone feature completeness1/5;
physical-device/release acceptance0/5. Scores reflect inspected gaps, not CI failure.
