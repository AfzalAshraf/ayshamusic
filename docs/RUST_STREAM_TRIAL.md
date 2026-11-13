# Opt-in Rust audio-transfer trial

## Approval and purpose

The user explicitly approved the staged short-lived resolver -> Rust transfer design,
with the existing playback path preserved and no automatic live-service change.
This is an integrated root-server feature, not another isolated acquisition CLI.
Default/unset MUSIC_STREAM_TRANSPORT remains legacy. Only rust-resolved opts in;
unknown/non-UTF8 values fail closed. No agent host installation or provider request
has occurred. The user must explicitly choose any trial install below.

The earlier host service reached101,171,200bytes. Largest observed process bucket:
yt-dlp71,299,072bytes RSS, Rust13,381,632bytes RSS. These per-label maxima are not
additive and do not identify the exact role of the observed helper. Independently,
source inspection shows the legacy browser stream retains yt-dlp for the entire
HTTP audio transfer. That lifetime is the target of this change.

**Not a below50MB or all-Rust release.** The resolver/optionalNode still runs at
startup; searches/Next may also launch metadata helpers. Startup peaks remain part
of the TOTAL budget. A lower steady reading alone will not meet the requirement.

## Implemented path

1. Accept an unambiguous11character YouTube video identity from supported HTTPS
   watch/share/shorts URLs. Reject playlists, credentials, duplicate video IDs and
   unrelated origins before launching a helper.
2. Acquire one trial stream slot (held through response-body lifetime). Another
   active trial returns429, not another process or an unbounded wait queue.
3. Launch yt-dlp in simulate mode, same bestaudio selector, with a narrow JSON
   projection. Ignore configs/plugins/cache; no account/cookie/login arguments,
   new client profile, generated tokens or persistent URL cache. Existing explicit
   MUSIC_YTDLP_JS_RUNTIME is still optional compatibility tooling, not native Rust.
4. Shared metadata slot,256KiB stdout/30s deadline. Wait for successful exit/reaping
   BEFORE any media GET. This owns/reaps the direct child, not independently every
   possible resolver descendant; optionalNode/escaped workers still count. Helper failure/busy/malformed record is an error, never
   a fallback to the long-lived downloader. No stdout/stderr/signedURL is logged.
5. Validate selected ID, non-live/audio-only directHTTPS protocol, WebMOpus/Vorbis
   or MP4AAC, signed media origin under .googlevideo.com/videoplayback. Allow only
   bounded safe headers; reject auth/cookies/Host/Range/encoding/hop-by-hop values.
6. Native reqwest GET: HTTPS only, normal TLS validation, no proxies/redirects,
   no idle connection pooling,10s connect/20s read/2h overall transfer deadlines.
   Startup (resolver through header acceptance) is capped45s. No whole-song buffer,
   transcoding, resampling or bitrate reduction. Select bestaudio, then validate;
   do not silently switch to a lower-quality supported format.
7. Require full200 response/no Content-Range or compression, acceptable MIME and
   matching12byte container prefix. Bounded chunks up to256KiB, direct body
   backpressure, no disk audio cache. Early drop/EOF/error releases network ownership
   and the stream slot. Errors contain fixed codes, never signed URLs.
8. Suspend speculative browser related prefetch throughout the trial playback mode,
   including when HTTP EOF precedes audible EOF because the browser buffered audio.
   Explicit Next/ended advancement still fetches related on demand. This can add a
   gap; gapless playback is not claimed. Explicit searches/Next can still create
   helper peaks and MUST be measured. The Bluetooth backend is not converted.

Response marker: X-Aysha-Transport: rust-resolved. GET /api/status now includes safe
stream_transport (legacy/rust-resolved/invalid) and rust_stream_active boolean.
These establish configured code path, not audible sound or memory acceptance.

## INSTALL — explicit opt-in HOST TRIAL, not a50MB acceptance claim

This changes/restarts5050 only if YOU run it. Do not run during active listening.
It rebuilds/tests first, saves previous unit/binary backups and checks the configured
mode via the status API. It never installs an isolated prototype. Local sudo may ask
for authorization; never share credentials in chat.

```bash
cd /home/afine/ayshamusic &&
git pull --ff-only origin arena/a850fb0d-ayshamusic &&
bash scripts/install_local.sh --rust-stream-trial
```

Expected: tests/release pass, service active, `Stream transport: rust-resolved`.
Preflight/build failure: stop; the existing service was not replaced. Unreviewed
service drop-ins/duplicate servers are rejected. Post-stop/start/health failure:
retain the printed timestamped backups and report the failing step; no automatic
rollback is promised. Status health does not fetch provider media or prove sound.

Then reload the LIVE5050 app on the phone, raise media volume and tap one ordinary
permitted song. Expected on a compatible source: this-device sound at the selected
encoded quality; the resolver exits before Rust transfers media. No need to run the
native acquisition evidence checker or repeat its consumed watch-only experiment.
A successful request is not a full-codec/EOF/control/resource acceptance result.

If this stream fails, stop retries. In particular, do not rotate clients, add cookies,
generate tokens or activate a hidden fallback. HTTP502 means resolver/record/CDN/
container failure;503 includes busy/config/client availability;504 startup timeout;
429 another active trial. The phone may display a generic media error instead.
No live compatibility result has been collected yet.

### Explicit default-path restoration

To restore legacy transport on this same updated root server:

```bash
cd /home/afine/ayshamusic && bash scripts/install_local.sh
```

This reruns tests/build and restarts the service with legacy transport; it is NOT an
exact previous-binary rollback. Exact unit/binary backups remain available if needed.
It does not reset Bluetooth, change audio quality or pretend legacy fits50MB.

## Evidence and remaining gates

Offline tests cover explicit mode/default/typo behavior, IDs/CDN/header/protocol/live/
codec/record guards, fixture resolver exit, bit-exact byte delivery, fragmented prefix,
redirect/403/206/encoding/MIME/container rejection, oversized chunks, early drop/EOF,
sanitized body errors/permit release and post-network-EOF prefetch suppression.
Fake container headers are fixtures, not real codec/hearing evidence. Installer tests
exercise help/invalid options/template rendering/syntax and source-order checks only;
no sudo/systemctl/install/audio actions occur in CI. The pinned REA-lab yt-dlp source
and offline template evaluator confirmed the JSON projection and default header keys;
that development-only Python inspection made no provider request and is not embedded
in runtime. REA is not the production resolver.

Before a total50MB claim: real permitted-source audio, codec/EOF/control/reconnect
behavior; attributable complete release idle/startup/playback/search/Next/track-change
peaks/swap; agreed listeners and platform costs. Native acquisition replacing the
resolver remains blocked separately at watch_visitor_missing; this trial does not
resolve it or reactivate that experiment. Shared audio/BlueZ/tunnel costs are not
excluded to manufacture a pass. No OOM cap/counter reset/quality cut/hidden fallback.

## Verified code state

-807b409 integrated transfer path: CI43429022364 all5jobsPASS.
-1a38a90 explicit installer/status/defaults and3b53084 buffered-playback prefetch fix.
-3b53084 CI43429899808 all5jobsPASS, including root Rust tests/release, mobile browser,
  extraction/audio/direct offline candidates. This is the final runtime verification.
-116Python/79JavaScript tests and diffcheckPASS locally. Installer unit/template tests
  do not exercise a real systemd switch. No provider/hardware/budget/hearing trial run.

## Latest supplied host receipt — install output incomplete

The user supplied checkout b448cc5,45/45 root tests passing (0.19s), release build
26.66s and the beginning of the saved-service-unit message. The excerpt ends there:
service stop/replacement/start/health/mode completion is NOT confirmed. No audible
trial result or memory measurement was supplied. The two unused-Json warnings were
in tests;8219767 replaces those ignored results with assertions. No runtime behavior
change/reinstall is needed for the warning fix or Python reporter improvement.

The older15-test root build,6-test audio prototype/missing local M4A and V5
watch_visitor_missing output in the same paste are historical. They do not establish
new source access or a new trial. Do not rebuild/repeat those prototypes, apt installs
or the consumed native-watch experiment. SSH Broken pipe is a connection symptom;
it is not evidence that the systemd server stopped or that audio failed.

If the installer is still running, let it finish. After the shell prompt returns,
use only this read-only DIAGNOSTIC (source update, no rebuild/restart/provider request):

```bash
cd /home/afine/ayshamusic &&
git pull --ff-only origin arena/a850fb0d-ayshamusic &&
python3 scripts/check_browser_playback.py --expect-transport rust-resolved
```

Expected SERVICE_RUNNINGpass, STREAM_TRANSPORTconfiguredrust-resolved/observed.
Exit2 means incomplete sound/budget acceptance by design, not an installation failure;
exit1 means a detected service/UI/status/mode failure. Legacy/invalid mode is not the
requested trial; absent mode stays unverified, never inferred from UI equality.
rust_stream_activefalse is normal before a song or after HTTP buffering ends, even
if the browser is still audibly playing. Do not require that flag to be true as a
hearing test. Logs produce fixed categorized counts only, not raw signed URLs/errors.
No entries/inaccessible logs are unverified; recent hints may refer to another client.

If service or transport fails, stop and share this JSON; do not blindly rerun the
installer or stop/restart Bluetooth. If active in rust-resolved mode, open the live
app on the phone and make one ordinary permitted-song Play attempt. No repeated
attempt is needed if one was already made: report whether sound was heard and the
exact visible player error if any. Native extraction/all-Rust/50MB remain unaccepted.

Reporter/test-warning patch8219767:120Python tests/diffcheckPASS locally; CI43462122764
all5jobsPASS including rootRusttests/release. No reason to rebuild/reinstall the host
just for this diagnostic/test-only patch.

## New host receipt — trial install and running mode confirmed

The user supplied the install completion message `Stream transport: rust-resolved`
and v2 diagnostic: SERVICE_RUNNINGpass, UI comparisonpass, API mode rust-resolved,
playingflagtrue/selectionpresent/streamslotheldtrue. This supersedes the earlier
incomplete-install receipt. The transient curl connection refusal was followed by
successful retry/status-mode validation; do not reinstall/restart for that line.
API mode is not exact backend revision identity; no actual hearing result supplied.

One RUST_CDN_OR_HEADER_FAILED hint was observed over the prior5minute log window.
It may be a previous request, including a different client. The active slot begins
BEFORE resolution and can outlive startup into transfer; it is not proof of received
media bytes. Zero UPSTREAM_HTTP_403 markers does NOT exclude a CDN403, because the
installed native response guard reports a generic fixed response-rejected code.
No guard weakening, fallback, token/auth/client change or provider retry is justified.

The v3 reporter now extracts only exact known whole-line Rust error codes from
EXISTING logs, plus one service memory snapshot using the existing systemctl query.
It never emits arbitrary suffixes, URLs, headers, raw log text or exceptions. A
lookback choice5/30/120minutes is bounded to100entries/1MiB; last-in-window is NOT
correlated with the current slot. Missing/expired/inaccessible entries remain unknown.
Memory current/peak/swap are separate; historical peak not attributed to this song,
RSS not summed, omitted platform/escaped costs cannot be silently excluded. Below
50MB current alone never accepts total workload; at/above50MB is a lower-bound fail.

**DIAGNOSTIC ONLY — no rebuild/restart/new media request or need to press Play again:**

```bash
cd /home/afine/ayshamusic &&
git pull --ff-only origin arena/a850fb0d-ayshamusic &&
python3 scripts/check_browser_playback.py --expect-transport rust-resolved --journal-minutes 120
```

Share only the JSON (especially rust_error_codes and SERVICE_MEMORY_SNAPSHOT) and
whether music was heard/the exact phone player error. Exit1 means a service/mode/UI
failure OR a current memory snapshot already at/above50MB; inspect the rows, not
just the exit code. Exit2 is partial evidence, never hearing/budget acceptance.

Branches: native_stream_request_failed -> connection/request layer, no raw reqwest
error capture; native_stream_response_rejected -> response status/MIME/encoding/range
policy ambiguity, do not assume403 or loosen guards; native_stream_container_rejected
-> selected/container mismatch; short_header/read_failed/chunk_limit -> startup/body
read acceptance. Resolving a generic response guard further would need a justified
instrumentation patch, not an unchanged rebuild or automatic request. No fixed-code
entry -> stop collecting; do not recreate a failed provider request just for the log.
If sound was heard, a historical hint alone is not reason to interrupt it. Full
startup/interaction/steady/EOF/recovery resource acceptance remains unverified.
