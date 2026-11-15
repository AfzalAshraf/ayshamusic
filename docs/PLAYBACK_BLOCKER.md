# Playback blocked after code10 phone tests

## Latest physical-device evidence

The user supplied `image-7.png` and `image-8.png` for source2267dba:

- Rust Audio10 opens but cannot play. ANDROID and IOS attempts report
  `audio_decode_failed ... input: media_range_http_403`; the TV attempt's displayed
  text is truncated. Do not invent its full outcome from that screenshot.
- Android Audio10 reportedly starts and stops on every song. Its screenshot shows
  `media3_input_failed: media_range_http_403` at0:00.
- The Telegram API-hash field disappears after Save; no evidence yet establishes
  whether that particular credential write failed or succeeded.

Both audio engines share the Rust source-acquisition path. These observations
prove failure to obtain/continue usable source bytes, not a solved decoder issue.
They do not identify exactly which provider requirement caused403. A rejected
request can involve signed URL/client/player-challenge/network policy; do not
claim any single cause without source evidence. The code10 port5061 correction
is separately reproduced and still valid: the Rust page now opens on the phone.

## Stop blind install-test iterations

No further header/client-version guesses, live watch probes, player rewrites or
APK delivery labelled a playback fix are authorized by this report alone.
The current Rust resolver parses direct audio URLs and lacks full JavaScript
player-challenge handling. A fuller extraction implementation needs an explicit
scope decision under the standing Rust-only extraction constraint.

Source-only investigation (no YouTube requests):

- https://github.com/Mithronn/rusty_ytdl — a possible Rust extractor, not integrated.
- Its inspected Cargo manifest includes `boa_engine`0.21.0 (an embedded JavaScript
  engine implemented in Rust). Android compatibility/size/source access not tested.
- https://github.com/Mithronn/rusty_ytdl/issues/53 reports403/download/crates version
  problems; https://github.com/Mithronn/rusty_ytdl/pull/62 proposes another client.
  These are not proof that changing libraries would solve the phone's problem.

Ask whether a Rust-only replacement (possibly Rust-implemented JS runtime) is
approved, or whether an established Android extractor fallback may be considered.
Separately ask permission for one bounded public-video CI source test. Prior
single live-watch experiment approval was already consumed. A CI source success
would not establish phone/network/region acceptance. No credentials/account
access, external Aysha helper, Python/Node runtime or source bypass is implied.

## Telegram save confirmation correction (not a playback fix)

Implemented on the working branch:

- Save writes to app-private storage and verifies the written credentials before
  success. A separate local `GET /api/tg/configuration` returns only configured/
  saved/API-ID information, never the hash/session. It makes no Telegram request.
- The existing cloud authorization/status path is retained for login/media;
  a save confirmation no longer depends on that network status operation.
- UI puts explicit saved/not-confirmed feedback beside Save. The hash is cleared
  only after the save acknowledgement and independent receipt match the API ID.
  On save/read-back failure it remains in the input for retry, never in browser
  storage; closing the dialog still clears secrets. Code/password clearing stays.
- Reopening restores a visible saved receipt and API ID, not the hash. Re-saving
  unchanged, already persisted settings is idempotent even if a Telegram client
  exists; changing a live client's credentials still requires disconnect.
- Offline Rust tests cover private save/restart/read-back/hash-free receipts and
  corrupt-file reporting. Node and Chromium fixtures cover save/error/read-back/
  reopening behavior. They are not real Telegram developer-credential validation,
  login, message delivery, upload or replay acceptance.

Local tests:105NodePASS,137PythonPASS, six native Settings/browser save-confirmation
fixtures plus actual loopback-port regression PASS. CI compilation status must be
recorded separately. No new playback-fix APK is being delivered from this change.
