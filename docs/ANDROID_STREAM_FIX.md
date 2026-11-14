# Standalone APK: fetching works, but no audible song

## What the APK contains

The phone APK embeds the Aysha UI, direct YouTube catalogue/resolver, HTTP media
reader, AAC/MP4 decoder and Android audio output. Its Rust HTTP listener is inside
the phone on127.0.0.1; it does not connect to the Linux music service, Vercel or a
hosted Aysha backend. Internet access and provider availability are still required.
The user's fetching report is not yet proof of receiving/decoding media or sound.

## Reproduced transport defect and patch

The prior Android player fed compressed MP4/AAC through a forward-only ChannelReader
and set with_seekable(false), without a byte length. MP4's moov/index may be AFTER
mdat/audio. A forward-only reader cannot jump to that index and back to audio.
The UI also used an idle HTML audio element's position/duration for the native player,
so its generic Streaming audio label did not prove that the native decoder was ready.

range_media.rs now implements Read+Seek using bounded64KiB HTTPS byte ranges with
known file length, including seek-to-tail/return-to-audio. The decoder receives the
byte length and with_seekable(true). HTTP206, Content-Range offsets/total/body length,
identity encoding and supported content type must match; redirects/plaintext or
arbitrary media hosts aren't accepted. A response that ignores Range returns a
specific error rather than downloading the entire song. Max512MiB media length,
16,384range requests,8sconnection/12sper-range total deadlines; cancellation is
checked before/after reads. URLs are not included in these transport errors.
The media request uses the same client user agent as the successful resolver rather
than always using Android's user agent for every resolved profile. No provider
profile additions/token/auth/visitor retry experiment was made for this fix.

pcm_stream.rs decodes/network-reads on a non-realtime thread, with a bounded8x4096
sample PCM queue plus one priming chunk. The Android audio callback never waits on
HTTP or the decoder: starvation supplies silence and reports buffering, and media
position counts consumed media samples rather than inserted silence. At most2decode
workers are allowed during replacement/cancellation. These buffer sizes are not a
claim of the app's full RAM footprint (codec, HTTP, Kotlin/WebView/platform memory
still exist). No full-song download or disk cache was added by this transport.

player.rs now primes real decoded samples, opens output lazily/retryably, and
acknowledges readiness before /api/play returns playing.40sdecoder/output startup
limit follows the existing bounded resolver requests; no indefinite startup success.
Stop cancels source/late resolver work; generation checks prevent an old decoder
failure overwriting a new track or Stop state. Control queues fail busy rather than
blocking Tokio. Midstream input errors propagate to player state. Decoder acceptance
and output-open success still do NOT prove sound was heard on a physical phone.

The full player uses native position/duration and shows resolving/buffering/errors,
not only the unused browser audio element's generic Streaming audio placeholder.
Linux media.rs/resolved_media.rs, service, installer and running playback are untouched.
Search and icons fixes are retained. Newer origin/main visitor/client acquisition
changes were inspected but not imported: catalogue fetch already works for this
report, and a decoder fix does not justify a new acquisition experiment.

## Tests and acceptance

Android CI generates two local sine-wave AAC/M4A files with FFmpeg: normal tail moov
and faststart/front moov. An offline regression reproduces the old unseekable-tail
failure, then actually demuxes/decodes nonzero samples and duration from BOTH using
the new range source. No YouTube or audio device is contacted by that fixture test.
Other tests exercise real loopback HTTP206/header rejection, wrong ranges/lengths,
seek-back/cancellation/cache bounds, fail-fast controls, stale-generation errors,
nonblocking PCM starvation/position, and native full-player status rendering.

Local137Python/89Node tests pass. CI run43587570184at48c72e3PASSED ALL steps (3m51s): Rust checks
with generated media fixtures set, Android cross-build/Gradle, APK size gate,
artifact upload and Android website contract tests. Linux CI43587570191also passed
all5jobs (coreRust/Python/JS, browser, audio/direct/extract prototypes).

Download: https://github.com/AfzalAshraf/ayshamusic/actions/runs/43587570184/artifacts/14111150675
Artifact ayshamusic-arm64-release, compressedZIP3,687,512bytes (not installed APK size
or RAM), GitHub-reported archiveSHA256
6de2fec9b252d44b1dc102a968d91e2f1728b193e28840c5b8cb8a9ec60707f6.
GitHub sign-in/repository access may be needed; artifact retention14days. The agent's network policy blocks GitHub artifact/log redirect
hosts, so GitHub job/artifact metadata is evidence—not independent APK byte/signature
inspection. No physical phone audibility or total RAM acceptance is fabricated.

## Install-test build

Version1.0.2-preview, application ID com.afzal.ayshamusic.streamfix,
launcher **Aysha Music Stream Fix**, Android8+/ARM64. Installs alongside the earlier
Preview/production app to avoid deleting local preferences over incompatible
ephemeral debug-signing keys. This remains an install-test build, not a production
signed/store-approved release. Do not use the old public apk-latest download.

After downloading/extracting the linked CI ZIP, install app-release.apk and open
**Aysha Music Stream Fix**, NOT an old icon. No Linux commands or server installation.
Search once, tap one song and wait for a specific state. The expected outcome is
advancing native position and audible phone audio, not merely a fetching result.
If it fails, send only the exact new full-player error/state and whether the timer
advances. Avoid repeated provider Test taps or raw signed media URLs. A specific
media_range_http_403/200, audio_decode_failed or audio_output_unavailable identifies
different work; do not weaken validation or add an external fallback to claim success.

Phone RAM remains unmeasured; Linux readings and APK size are not phone memory.
