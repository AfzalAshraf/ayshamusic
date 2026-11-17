# Long-track playback follow-up (build13)

## Download and verified results

**[Android/Media3 Audio13 APK — ZIP](https://github.com/AfzalAshraf/ayshamusic/actions/runs/44029768380/artifacts/14305435131)**

Source**3d24f7f**, version1.3.5-preview/code13. ZIP4,969,329bytes,
SHA25654b8f915b51e69ce74dcf573e3ed25a3a592af624235d177eacf31e4b537c8d6.
GitHub metadata verified; not a local APK binary/signature-inspection claim.

Android44029768380 **SUCCESS both jobs**: release133858634390,
emulator133858633925 **all5actual Android Media3 tests PASS**, including:

- `tenMinuteTrackPlaysAllTheWayAtFourTimesSpeed` **PASS** (encoded600s, real4x
  playback to end, actual media bytes read; not600wall-clock seconds).
- `tenSecondSourceGapDoesNotTripTheDefaultEightSecondReadTimeout` **PASS**.
- Existing >2s/pause/resume/end, pre-ready pause and initial rejection tests PASS.

Safe annotation evidence: `research/media3-long-track-emulator-result.json`.
Emulator report artifact14305360956,23,253ZIPbytes,
SHA256bc771142d475c42d4fb0ed3789eb230a0bce11dec4f7e58cfd3f4089c0595661.
Linux44029768439 **SUCCESS all six jobs**, including browser/native Settings.
Rust phone tests (request-scoped redirects, >10MiB full streaming, actual loopback
worker and late seek) PASS in release build job. Local107Node/142Python PASS.
Initial2e4427e Android run44029628665 was superseded/cancelled by the final added
loopback regression, not a source/decoder validation failure. No live provider
probe or archived source experiment re-enabled.

**Physical-device acceptance remains open.** The new error/song has not been
provided, so these code defects are fixed/tested counterexamples, not confirmed
attribution of that exact phone failure. No source-policy/403 bypass is claimed.

## Report and evidence limit

After build12 was delivered, the user reports that music plays only briefly and
long-track selections show an error. The new exact error, affected song/link and
installed source receipt have not been supplied yet. Requested once while starting
work. Do not assume a new403, a hard track-duration cap, or confirmed decoder fault.
Build12 is therefore **not physically accepted for long-song playback**.

## Two concrete defects corrected

### 1. Inactivity timeout mismatch

Official Media3 1.8.0 source:
https://github.com/androidx/media/blob/1.8.0/libraries/datasource/src/main/java/androidx/media3/datasource/DefaultHttpDataSource.java

`DEFAULT_READ_TIMEOUT_MILLIS = 8 * 1000`. Rust allows12s per upstream range,
and its Media3 input worker waits15s. The unconfigured Android data source could
abort a slower block before the Rust operation deadline. PlatformAudio now supplies
an explicit DefaultDataSource/DefaultHttpDataSource media-source factory with20s
read inactivity and8s connect timeout. This is **not a total song-duration timer**;
no unlimited reads, unbounded buffer, broad retries or provider403 bypass was added.

### 2. Request-scoped redirects were incorrectly persisted

Build12 promoted a successful redirect target into HttpFetch's stored URL. A
location may contain a signature/range specific to that single request. Its
success for the beginning does not authorize reuse for a tail/next-range request.
HttpFetch now retains the original source URL and follows each range's own CDN
handoffs. HTTPS/googlevideo/no-userinfo/443 checks,3-hop limit, exact byte-contract
checks and12s per-operation deadline remain. First/tail **bytes** are still cached
(up to128KiB), but temporary redirect **URLs** are not promoted into future origins.

A local HTTP-response fixture models signed targets that reject use for a different
range with403. Five sequential/seek reads must each start at the original URL.
This is a reproducible redirect-scoping counterexample, **not proof that this exact
provider behavior caused the user's unspecified long-song error**.

## Expanded tests

- Rust PreparedFetch streams every byte of a >10MiB generated source through more
  than150later reads, checks offsets/tail/back-to-start and retains <=128KiB probes.
- The actual Rust Media3 loopback worker/channel/Axum body streams >10MiB frame by
  frame (no whole-file collection) and supports a late suffix/seek request.
- Real Android PlatformAudio/Media3 plays a generated **ten-minute AAC track at4x**
  to its actual end, checking middle/end progress and file bytes crossing the
  socket. Acceleration is test-only reflection into the real ExoPlayer, not fake
  position events or a production control/API change. This is not ten wall-clock
  minutes or a copyrighted-song/provider test.
- A generated short AAC track's local HTTP body pauses for10s between chunks;
  the read must survive the old8s threshold and reach actual playback/end.
- The previous pause/resume/end, pre-ready pause and initial403 tests remain.
- Error reports distinguish preparation head/tail failures from streaming failures
  and include the failing stream offset, without source URLs or credentials.

No live source test was repeated in this follow-up. Existing source-proof entry
points remain sealed. Fixtures do not access provider accounts or real Telegram.
Android emulator fixture mode excludes Rust JNI/provider acquisition; the actual
Rust loopback worker is tested separately on host. No end-to-end physical phone,
background, network-change, hour-long real-time or arbitrary provider-policy claim.

## Build identity

Build13/1.3.5-preview, label Aysha Android Audio13, same androidaudio10 package,
port17652, ARM64 Android8+. Media3-only delivery, image9 launcher, Telegram receipt
fix, dedication and player design preserved. Signing remains CI preview/debug
fallback; an update may be rejected if signing keys differ. Do not recommend
uninstalling without warning that local settings/session data would be deleted.

Local107Node/142Python PASS before CI. Final run/artifact outcomes are recorded
below after verification, not inferred from a successful APK compilation.
