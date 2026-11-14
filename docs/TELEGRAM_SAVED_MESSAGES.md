# Telegram account and saved-song playback

## User-approved choices

The user explicitly selected these options in this session:

1. Their own **Saved Messages**, not a bot, public channel or another person's chat.
2. Save every successfully downloaded selected song, **even after skipping**.
3. Try YouTube automatically when a saved song cannot be opened from Telegram.
4. Provide **guided setup** for the user's own Telegram API ID/API hash.

This is an in-app Rust MTProto feature, not an Arena connector. No Telegram
credentials, account codes or passwords should be sent in chat or committed to Git.

## Phone setup

1. Install the identified candidate APK and open **Aysha Music Telegram**.
2. Go to **Library → Telegram account**. These controls are not on Now Playing.
3. Expand **First-time API setup**. Follow the link to the official
   `https://my.telegram.org/apps` in your browser, sign in and create an application
   to obtain your developer API ID and API hash. The APK does not bundle shared
   credentials. Telegram controls whether that setup is available to your account.
4. Enter the API ID and API hash **inside this app** and choose Save API settings.
5. Enter your phone number with country code and request a login code. Check your
   Telegram app; code delivery is controlled by Telegram, not guaranteed SMS.
6. Verify the code. If requested, enter the account's two-step verification password.
   Codes, password and API-hash input fields are cleared after requests/on closing.
7. Refresh status to confirm authorization and check pending/failed uploads.

Developer credentials and the MTProto session are stored only in the app-private
files directory. Auth/session writes are atomic and mode0600 on Android/Unix;
Android backup is disabled. Password/code inputs are not saved in browser storage.
The persisted MTProto session contains authentication material: app-private storage
is not a claim of Android Keystore encryption or protection from a compromised OS.

## Playback and saving

- A validated YouTube video ID is the identity of a song in the archive.
- Replay checks a local Saved Messages message-ID index first, then bounded Saved
  Messages search if needed. Captions must start with the exact `#yt_<video_id>`
  marker; fuzzy search matches and other chat content are not trusted as identity.
- A found Telegram media file is streamed in aligned chunks into the same bounded,
  seekable MP4/AAC reader/decoder as YouTube. It is not downloaded in full into RAM.
- Telegram connection/search is bounded. Missing/deleted/unavailable/unsupported
  saved media at startup falls back to the normal YouTube resolver and range player.
  A read error later in an already playing Telegram stream is surfaced; seamless
  mid-track failover/resume is **not yet implemented**.
- A YouTube track accepted by decoder/output is queued for saving. A single worker
  resolves/downloads a complete copy to temporary app-private disk, validates that
  it decodes, then uploads a document to **Saved Messages** with title/ID caption.
- This worker is independent of playback and intentionally finishes after a skip.
  It may download a second full copy of the audio; this and uploads consume data.
  No claim of a zero-copy archive or mobile-data/Wi-Fi scheduling is made.
- New songs can play without configuring Telegram. Their bounded save queue waits
  for account setup; a full/unavailable queue reports a warning in account status.
- Saved-message IDs persist across restarts. The worker checks for an existing save
  before uploading again. Deleted saves can be recreated on a later YouTube play.
- Files are sent only after complete-length checks and decoder validation. A partial
  `.part` file is never published as a successful save. Old partials are discarded
  on worker startup and overwritten on retry.

## Limits and recovery

- Existing **400MiB total temporary audio cache** retained; per-track guard64MiB.
- At most20 pending/uploading saves. Once full, further plays can still work but are
  not guaranteed a save; account status reports the problem. No unlimited disk use.
- App upload pacing: at most18/hour, minimum12seconds between files; this is our
  limiter, not a promise of Telegram's allowance. Server FloodWait pauses uploads.
- Saves retry at most3 times before becoming failed; retrying playback explicitly
  can requeue a completed/deleted/failed archive record. Pending saves are protected
  from eviction; completed/unneeded cached files make room for new ones.
- Provider rejection, Telegram/API errors, insufficient storage, process death and
  account restrictions can delay or fail a save. Pending metadata survives restart;
  force-stop/background restrictions can suspend work. Background/screen-lock
  operation is not certified by host CI.
- Disconnect pauses work, attempts server session revocation, clears local pending
  saves/message index/session after successful revocation, and stops pending/native
  playback. If revocation fails, the UI reports it; retry while connected. Uploaded
  Saved Messages are **not deleted**. Developer API settings and bounded local
  YouTube cache may remain until app data is cleared.
- Telegram Saved Messages is cloud storage, **not end-to-end encrypted**. Only save
  content you have permission to copy/store and comply with source/service terms.

## Evidence boundary

The latest QA ledger records the exact APK source, CI checks and artifact link.
Offline fixtures test demux/decoding, cancellation, range contracts, queue/index
behavior and credential-file privacy. They do not log into a real Telegram account,
prove real uploads/downloads, satisfy music-provider restrictions or establish
physical-phone audibility. No user account was accessed during implementation.

## Current install-test artifact

[Download **Aysha Music Telegram — 1.3.0-preview**](https://github.com/AfzalAshraf/ayshamusic/actions/runs/43594634691/artifacts/14113683577),
extract `app-release.apk`, install and open **Aysha Music Telegram**. Source99972a3,
code8, packagecom.afzal.ayshamusic.telegram8, Android8+/ARM64. It is separate from
older test apps. APK/host fixture checks passed in Android43594634691; all five
Linux jobs passed in43594634703. This is not real-account or phone-audio acceptance,
not a production-signed store release. Artifact retention14days; GitHub sign-in may
be needed. No external Aysha server is required.
