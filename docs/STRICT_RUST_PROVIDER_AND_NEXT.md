# Strict-Rust APK: provider400 and mood-based Next

## User evidence and decision

The screenshot shows ANDROID and IOS HTTP400 / failedPrecondition, followed by an
unplayable TV-client result. This is a rejection of the resolver request BEFORE
native range fetching/MP4 decoding. Real AAC decoder tests from the previous fix
never established a successful live YouTube resolver. Do not call a decoder change
a fix for this400 or declare this app fully working based on CI compilation.

The user explicitly chose strict_rust when asked about replacing the handwritten
resolver with an on-device NewPipeExtractor. Do NOT add NewPipe/Java extraction,
Python/Node/yt-dlp, a token service, proxy or hosted Aysha fallback without a new
explicit decision. Android/Kotlin shell and embedded HTML/JS UI already exist;
'Rust' here describes acquisition/transfer/decode/player core, not Android UI code.
The APK does run these on the PHONE, not the user's Linux machine. Internet and
provider availability are still required. Publication is not accepted yet.

## Compatibility patch, not verified live acceptance

- Replaced the old19.xAndroid/iOS request metadata with matched21.26.364Android /
  21.26.4iOS context/UA/OS fields. Added userAgent to the body, matching HTTP headers.
  Profile facts checked against yt-dlp's current source snapshot:
  https://github.com/yt-dlp/yt-dlp/blob/12ac9aca79671420724227aca37bc888c604a6f8/yt_dlp/extractor/youtube/_base.py
  These are protocol field facts, NOT inclusion of Python or a claim that a newer
  version string solves a provider refusal. This source also describes proof-token
  requirements for media delivery: request acceptance would not prove CDN playback.
- Reuses responseContext.visitorData ONLY if actually returned by ordinary YouTube
  search/player JSON, sane/ASCII/16–1024bytes, in process memory for at most30minutes.
  Matching header/body. No extra identity endpoint/watch probe, synthesized token,
  cookie import, account login, disk identifier or visitor preview/logging added.
- Bounded player JSON1MiB/12sper request, existing search2MiB/12s. Removed old embedded
  TV profile from automatic fallbacks. Legacy explicit TV alias remains for caller
  compatibility; it is not automatically tried or claimed to work.
- Recognizes the supplied400precondition structure as terminal PreconditionFailed;
  returns youtube_precondition_failed instead of repeatedly rotating profiles. Does
  not retry other songs after a provider rejection. No removed content/login/age/
  geo/proof guard or challenge-solving/bypass logic. No quality/codec downgrade.
- Wrapped full-player error region separate from clock. Resolver errors survive idle
  polling instead of disappearing because the player was never started. Build SHA
  receipt identifies the actual installed source; stale player metadata cannot
  replace a pending selection for another ID.

This is a justified compatibility candidate. Exact cause of the provider's400 beyond
the supplied rejection remains unproven; actual acceptance/CDN/audio are NOT fixed
by assertion. If still rejected, stop, retain the exact error, and do not publish or
repeat many Test/profile/version attempts. A fully working all-Rust resolver is still
an open gate until a real device receives, decodes and audibly plays a track.

## Automatic Next: similar mood, not same song name

- Native Now Playing offers **Romantic** (default) and **Melody** controls.
- Explicitly queued songs play FIRST, in user order. Previous uses actual successful
  playback history; automatic mood choice does not replace those manual choices.
- Empty queue: ONE bounded mood-only search ('romantic love melody songs' or
  'soft soothing melody songs'), not '<current title> similar songs'. No private
  liked-song titles are sent as query seeds. Search and playback slots remain bounded.
- Scores mood words and locally liked artist/channel labels; preserves provider
  result order for ties. Likes/history/current must be actual existing local state.
- Filters currentID, normalized same-name alternate uploads/lyrics/covers/remixes/
  slowed variants and last50played identities. No replacement means a clear message,
  not knowingly replaying the seed. At most one selected candidate is resolved per
  Next tap; failed play does not consume queue or mutate current/history.
- This is a metadata heuristic, not audio fingerprinting, inferred emotion, a trained
  recommender or a guarantee across all languages/transliterations. Liked YouTube
  channels are not always the song artist. Queue/likes/mood are process-memory state,
  not persistent account sync. Natural-end autoplay was not added in this patch.

## Install-test build and evidence

Source0d5f461. Android43589877314PASS all steps2m57s: Rust tests including realAAC
front/tail fixtures, request contracts, typed400, recommendation fixtures/route,
Android crosscompile/Gradle,sizegate,artifactupload and websitecontracts.
Linux CI43589877315also PASSED all5jobs. Locally137Python/91NodePASS and
diffcheckPASS. Tests do not call live YouTube.

Version1.0.3-preview/applicationcom.afzal.ayshamusic.rust4/launcherAysha Music Rust4.
Download: https://github.com/AfzalAshraf/ayshamusic/actions/runs/43589877314/artifacts/14111903921
Artifactayshamusic-arm64-release; ZIP3,709,490bytes (not APK installed size or RAM).
GitHub-reported ZIP SHA256341dc12bd3b6c6828cc5834d9f88b716d51f8a1ece8102099504447a34d8ed2f.
Sign-in/repository access may be required;14day retention. Extract app-release.apk,
install and open **Aysha Music Rust4**, not an old Preview/StreamFix/main release.
This separate test package preserves old app data despite ephemeral debug signer;
no production signing or public apk-latest overwrite. Android8+/ARM64 only.

No Linux commands or server installation needed. Search/select one song once.
Success requires audible audio and an advancing native timer. If HTTP400 remains,
the provider gate remains blocked. Send the new wrapped message/build receipt only;
no raw signed URLs, credentials, repeated provider Test taps or build/RAM loop.
If playback succeeds, choose Romantic/Melody, test Next with empty queue and verify
it chooses a genuinely different song. Then verify manual queue ordering separately.

Phone RAM remains NOT MEASURED; old Linux service figures and ZIP size do not apply.
The agent cannot download GitHub's log/artifact redirect hosts from this network;
metadata/CI step states are evidence, not independent APK signature/device inspection.
