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

The user subsequently selected **Rust-only replacement research**, including a
Rust-implemented JS runtime if needed, and **one bounded CI experiment** on one
public video:12requests/8MiB/60seconds/two small media ranges. No Android extractor
fallback was approved. That new experiment is now **consumed**; see the result
below. Prior single live-watch approval was separate and already consumed too.
No credentials/account access, external Aysha helper, Python/Node runtime or
source bypass was approved or used.

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


## Approved experiment result — NOT a working replacement

Source7fe2ab2, [run43737916216](https://github.com/AfzalAshraf/ayshamusic/actions/runs/43737916216),
job132884517889. The fuller Rust candidate plus guard tests compiled. The live
step then failed the gate with `candidate_media_host_rejected`, at metadata stage:

- **3** guarded HTTPS requests; **1,389,875** transport bytes reserved/used;
  **1,453ms** elapsed. **Zero media-range requests** reached program execution.
- The selected URL did not pass the strict HTTPS/googlevideo/no-userinfo/default-
  port check. The report intentionally did not retain its signed URL, so this is
  **not proof of a specific host, empty-URL cause or HTTP403 from this candidate**.
- No first/tail source-range, AAC decode, Android compilation or phone-acceptance
  gate was passed by the candidate. It remains isolated from the APK Cargo graph.
- Sanitized annotation transcribed in `research/rust-source-once-result.json`.
  Artifact14170963008 contains the original JSON and generated Cargo.lock (ZIP
  20,158bytes, SHA256a2e5100529450544f6d5f019cd0b814e7b274dc4aff0a8a96caab15b94374208).
  Metadata verified through the API; no restricted binary-download retry.
- API manual dispatch was denied (403); that call started no live run. A narrowly
  parent-SHA/approval-message gated push was used instead. An initial YAML syntax
  failure had no jobs/network; the corrected run above is the sole live attempt.
- **Do not retry.** The parent/message gate and manual source input have been
  removed from the Android workflow. The reusable live workflow job is disabled.
  `LIVE_APPROVAL_AVAILABLE=false` makes even `--approved-once` return offline.
  Offline guard tests assert that the consumed permission stays disabled.

Telegram correction separately passed Android43736467314 and Linux43736467342
(all five jobs), source540c8c7. These prove build/fixture/read-back behavior, not
that the user's particular credentials were lost, valid, or accepted by Telegram.
No further APK is being represented as a playback fix. The installed code10
artifacts failed device acceptance; do not resend those links as known-working.


## User-requested image9 recheck and APKs — renewed allowance consumed

The user explicitly said "if u need check again and make athe apk" and supplied
image9 as the launcher icon. This authorized one new test under the same public-
video/budget/no-account limits and fresh APKs; it did not authorize disguising
source failure as a successful playback fix. The attached icon supersedes the
prior red launcher-only artwork; player Play/design/dedication remain unchanged.

Source938b364, [run43739826775](https://github.com/AfzalAshraf/ayshamusic/actions/runs/43739826775),
source job132890296562: build/offline guards passed. The **single** renewed live
step returned `candidate_no_allowed_aac_url`:27formats,2AAC entries (139/140),
both with content lengths but **empty extracted URL strings**. It used3requests,
1,383,134transport bytes reserved/used,1,355ms; no media ranges were requested.
Safe evidence: `research/rust-source-image9-recheck-result.json`.

The experiment corrected local selection to exclude invalid URLs before AAC
ranking and added safe per-format URL categories. Neither changed the underlying
extractor/profile or weakened HTTPS/host/account checks. Thus the recheck resolves
the previous ambiguous guard failure to **empty returned URLs**, but still does
not establish whether URL loss originated in provider data, upstream parsing or
challenge handling. No raw response or signed URL was retained; do not invent a
more specific diagnosis. No candidate integration or phone-playback claim.

Both new build11 APKs were explicitly requested and are produced with the image9
launcher and existing Telegram receipt correction. Android build job132890296348
**SUCCESS**, both artifacts present; Linux43739826787 **SUCCESS all six jobs**.
The Android workflow overall is failure only because the independent research
source gate failed. See `DUAL_AUDIO_APKS.md` for exact artifact identities.

Renewed allowance CONSUMED. Exact-parent push gate removed again, reusable live
job disabled, both old and renewed CLI flags/environment tokens ignored by the
sealed binary. New user permission plus a justified code change is required for
any further live source test. Fresh APKs are offered honestly as icon/Telegram
updates, not another "everything fixed" reinstall loop.
