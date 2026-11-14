# Reusable project prompt — Aysha Music

Copy the text below into a coding-agent conversation working on this repository.
This is a reconstruction of the user's project request, not hidden agent instructions.

---

Continue improving my existing project, **AfzalAshraf/ayshamusic**. Work on the current
branch **arena/a850fb0d-ayshamusic** and inspect the actual repository, latest diffs,
CI results and QA ledger before changing anything. Do not replace the app with a
new template or assume that a successful build means playback works.

## What I want

I want a working, self-contained Android music APK that preserves my approved
Aysha Music website design. It must stream and play music on the phone without
requiring my Linux server, a cloud Aysha backend or a separate helper app.

Keep acquisition, resolution, transfer, decoding and playback **strictly Rust**.
The existing Kotlin Android shell and HTML/JavaScript WebView interface are fine.
Do not add NewPipeExtractor, a Java extractor, Python, Node, yt-dlp, an external
resolver or a proxy fallback without asking me and receiving new approval.

Preserve the existing dark/rose Android-first UI, bottom navigation, mini-player,
Now Playing screen, visible errors and the exact dedication:

**Made with ❤️ for Aysha — with love, from Afzal**

Preserve the separately working Rust/systemd Linux mode. Do not restart or replace
my Linux service while fixing the phone APK. Phone/browser output is the requested
output, not the server's JBL speaker.

## Recommendations

Do **not** show a Romantic/Melody dropdown or make me choose a mood. Automatically
learn my taste from the songs I actually listen to most, played duration, short
skips and explicit likes. Store the learning locally on my phone. Keep it invisible
on the player: no Reset learning button or learning-explanation paragraph there.
Do not count failed playback, search activity, pauses or wall-clock waiting as
listening. Explain honestly if inference uses metadata/channel heuristics rather
than audio analysis or a trained model.

When I press Next with an empty queue, choose another suitable song based on that
aggregate listening profile, not a search for the current title plus 'similar'.
Avoid repeating the same song under another upload, lyrics, cover, remix or slowed
version. Keep explicit queue order first and Previous/history working. Failed
resolution must not silently consume my queue or pretend a track started.

## Current problem and important evidence

An older installed **Rust4 / source 0d5f461** failed at 0:00 with:
'ANDROID: no audio-only AAC stream offered; IOS: no audio-only AAC stream offered.'
This is a resolver/provider failure before media transfer or decoding.

The other agent published main **1.2.1 / commit 7b09cde**, separating the 12-second
API timeout from the media client, which has no whole-track timeout. Preserve that
fix. Its host evidence for one video is not proof that every music video resolves
or that sound plays on my Android device.

That baseline was merged in **efb0613**, and local listening recommendations and
reset contracts were completed in **6083719**. The **1.2.2-preview** candidate was
called **Aysha Music Listening**, package **com.afzal.ayshamusic.learn6**. Android CI
43591553488 and all five Linux CI jobs in 43591553492 passed; that did not establish
phone audibility.

**Latest report: the app unexpectedly closes when I tap Play.** Confirm the exact
installed build if necessary; do not assume the report contains a native crash log.
Treat this as a crash, not merely the old no-AAC message.

Source investigation found that locked CPAL **0.18.2** calls
`ndk_context::android_context()` when opening Android audio. The app had never
initialized that context, and ndk-context **0.1.1** panics if it is absent. The
release profile uses `panic = "abort"`. This is a concrete crash-capable defect,
not yet confirmation of my particular crash without device evidence.

Patch **d6fd6a0** passes the application Context through JNI, pins a GlobalRef,
initializes ndk-context exactly once before Rust server/audio workers start, and
checks the initialization invariant before CPAL opens audio. Follow-up **9f7f9d0**
keeps the Kotlin JNI bridge intact under release shrinking. These preserve Rust
playback, the main media timeout fix and recommendation behavior. Check its latest
CI/artifact results instead of assuming the patch is already device-verified.
The candidate identity in that patch is **1.2.3-preview / code 7**,
**Aysha Music Audio Fix**, package **com.afzal.ayshamusic.context7**.

## How to finish

1. Reproduce or explain the supported failure path; use the actual crash stack if
   available. Do not guess that a timeout/decoder/provider tweak fixes every issue.
2. Keep the fix narrow. Add useful regression tests, run frontend/Python/Linux Rust
   checks and Android host/cross-build checks. Distinguish static contracts,
   offline tests, CI compilation, provider acceptance and physical phone audio.
3. Provide the exact downloadable APK artifact, version, launcher name, source SHA
   and verified CI results. Preserve old app data; do not force an uninstall merely
   because CI signing keys differ. Do not overwrite the public main release or
   publish to a store without explicit approval and production-signing readiness.
4. Keep me informed without sending me through repeated unchanged reinstall or
   client/profile/Test-button experiments. If a crash remains, obtain its evidence
   rather than repeatedly changing unrelated settings.
5. Update `docs/LOCAL_QA_LEDGER.md` with actual results and open gates. Never claim
   'working', audible, low-RAM or publish-ready from compilation alone. Phone RAM,
   background playback, audio focus, provider coverage and release signing require
   their own evidence. Historical Linux memory numbers are not current APK memory.

The current task is to fix Play without closing the app, then verify real streaming
and sound while retaining automatic local recommendations and the approved design.


### Verified build update for this handoff

Crash-fix source **9f7f9d0**: Android run **43592475037** passed; Linux run
**43592474963** passed all five jobs. Local **94 Node / 137 Python** tests passed.
Artifact **14113285899**, `ayshamusic-arm64-release`:
https://github.com/AfzalAshraf/ayshamusic/actions/runs/43592475037/artifacts/14113285899
Extract `app-release.apk`; open **Aysha Music Audio Fix**. Physical-phone crash
resolution and audibility remain unverified. Recheck current repository/CI if this
prompt is reused later; artifact links expire and are not production releases.

## Subsequent requirements — supersedes the earlier handoff's current task

A later screenshot of9f7f9d0 shows `audio decode failed: The format of the data has
not been recognized.` The Reset learning explanation/button was rejected and
removed. Keep learning invisible on Now Playing.

The user has now explicitly approved a Rust Telegram user-account feature:
**Saved Messages**, save every complete downloaded selected song **even after a
skip**, replay from Telegram first with **automatic YouTube fallback**, and
**guided in-app setup** for the user's own developer API credentials. Do not ask
for API hashes, login codes or passwords in chat. Configuration/login goes in
Library → Telegram account. No bot, another user's chat or hosted Aysha helper.

Sources de0ab69/77f0584/9873765/99972a3 reconnect bounded seekable MP4/AAC range
input and ready-PCM decoding, preserve Android context/R8 and the main media client
fix, add private in-app account setup and Saved Messages upload/search/message-ID
index/range replay, bound cache/retry/connection work, and honor Stop during source
preparation. Full archive downloads may duplicate a transfer and continue after
skips as approved; account status exposes pending/failed work. Do not silently add
unlimited disk/data use. See docs/TELEGRAM_SAVED_MESSAGES.md for exact limits and
startup-vs-midstream fallback behavior. Read the latest QA ledger for current CI
and APK links; older artifacts above do not contain these changes. Physical phone
sound and real account upload/replay still require actual verification.

Latest delivered integration artifact: source99972a3, **Aysha Music Telegram
1.3.0-preview/code8**, Android43594634691PASS, Linux43594634703all5PASS,
98Node/137PythonPASS. Download:
https://github.com/AfzalAshraf/ayshamusic/actions/runs/43594634691/artifacts/14113683577
Phone sound and actual account save/replay remain unverified. No public release
was overwritten. Setup details are in docs/TELEGRAM_SAVED_MESSAGES.md.
