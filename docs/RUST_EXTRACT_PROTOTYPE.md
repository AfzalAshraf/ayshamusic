# Native extraction prototype — benchmark before replacing yt-dlp

Status: isolated experimental candidate, NOT production, live success/speed/memory
NOT YET VERIFIED. The first step requested is native extraction; the existing
Rodio audio experiment remains separate and unchanged. No service/tunnel/Vercel
backend migration occurs. Only access content you are permitted to use.

## Actual implementation

`prototypes/rust-extract` pins rusty_ytdl0.7.4, default features off, rustls-tls only.
This is native extraction with its embedded Boa JavaScript engine (written in Rust),
not a Rust wrapper invoking yt-dlp/Python/Node. Embedded JS is still execution work
and may use substantial memory. No live/ffmpeg/search feature or hidden subprocess
fallback is enabled. Current YouTube signatures/bot checks/IP restrictions can break
this candidate. A library's existence is not evidence it works on today's tracks.

Input is an11-character video ID, not an arbitrary URL. Resolve only public non-live
audio-only AAC/MP4, then accept HTTPS subdomains of googlevideo.com with no credentials
or explicit alternate port; redirects/proxies disabled. First64KiB is validated by
MP4 ftyp signature and discarded. Logs contain fixed error codes/metrics, not video
metadata, cookies or signed URLs. CLI returns0 on success,1 extraction/media failure,
2 argument error. Other formats/search/playlists/private/age-restricted content are
outside this first prototype. Do not bypass restrictions or silently fall back.

The cooperative60s timeout does not preempt synchronous dependency/JS work; use an
external process deadline for hard isolation. Request timeout20s/connect10s, no
retry. Dependency metadata/player parsing does not have a proven heap bound; no100MB
claim. Probe stops after64KiB consumed, but upstream could ignore Range and deliver
more to network buffers before cancellation. It is not full decoding or first sound.

## Build and compare on the confirmed Linux host

Pause production audio first to avoid contention (do not stop/reinstall its service).
The root build/installer does not build this package. No ALSA headers required for
extraction-only tests. Use the installed Rust toolchain:

```bash
cd /home/afine/ayshamusic &&
git pull --ff-only origin arena/a850fb0d-ayshamusic &&
cargo test --manifest-path prototypes/rust-extract/Cargo.toml &&
cargo build --release --manifest-path prototypes/rust-extract/Cargo.toml
```

Only after both Cargo commands succeed:

```bash
python3 scripts/benchmark_extract.py --video-id Jm7Satp7Zqk --runs 3
```

The benchmark uses existing yt-dlp/Node only as an explicitly labelled comparison.
It performs sequential, alternating-order runs (max5),75s external deadline each,
kills/reaps its own process group, validates both64KiB MP4 probes, and automatically
prints/saves a private report under ignored diagnostics/. No automatic upload.
Python is a stdlib diagnostic harness, not the production server. Raw upstream errors
are suppressed. Compare success count first, then median total_ms (includes process
startup and reading64KiB for both tools). Rust resolve_ms/first_byte_ms/native_total_ms
are internal timings; baseline first_byte_ms includes process startup. AAC/MP4 selection
constraints match, exact itag/bitrate is NOT yet proven identical. Upstream caches,
IP/client behavior and network conditions affect results. Both tools disable proxy
use here; benchmark ignores yt-dlp config/cache. Never compare failure speed to a
successful download or claim whole-song speed from prefix latency. Rust peak RSS
is just that one process; baseline aggregate RSS is NOT measured. Use the memory
profiler separately and include subprocesses for real resource acceptance.

## Optional native extraction to existing audio prototype

If the probe succeeds, prepare a NEW file outside Git (max64MiB):

```bash
mkdir -p /home/afine/.local/share/aysha-audio-prototype
timeout --kill-after=2s 75s \
  ./prototypes/rust-extract/target/release/aysha-extract-prototype \
  --video-id Jm7Satp7Zqk \
  --output /home/afine/.local/share/aysha-audio-prototype/native-test.m4a
```

Existing output is refused, never overwritten. In-process errors/timeouts remove
the incomplete newly created file; SIGKILL or crash may leave one. Inspect/remove
that one experiment file manually before retrying; never run broad cleanup. Output
header verification is not a complete MP4 integrity check. Do not pipe signed URLs
into chat. Then use RUST_AUDIO_PROTOTYPE.md to decode-only first and test audible
playback on the intended Linux output. The64MiB file cap remains a disk/input cap,
not peak memory certification.

## Upgrade sequence and production gates

1. **Extraction now:** compile/offline safety tests, then real public-track success
   and repeatable same-host benchmark. Reject candidate if compatibility is poor;
   do not add cache tricks to disguise failure. No universal yt-dlp replacement claim.
2. **Audio prototype next:** keep current working local-file baseline; add structured
   worker protocol/error codes, owned cancellation, predictable EOF/natural-end,
   output-device selection and reconnect/error behavior; tests for malformed commands,
   unsupported/corrupt codecs, seek and cleanup. Device switching must be explicit.
   WebM/Opus remains unsupported until deliberately implemented/tested.
3. **Joint experiment:** native extraction-to-file-to-Rodio, real AAC decode/audibility,
   measured cold/warm startup, CPU and aggregate memory. Any bounded disk-cache design
   needs size/lifetime/eviction/permission approval and page-cache accounting.
4. **Production only after evidence:** approve device/session semantics and resource
   boundary, backpressure/admission/cancellation ownership, API contracts/auth,
   concurrency/real mobile tests, service supervision and rollback. No global kills,
   hidden fallback, unmeasured100MB promise or replacing production from a prototype.

## Hosting answer

Vercel has a Rust function runtime, but a copied local systemd daemon is not an
approved drop-in deployment. Function streaming has maximum duration; cloud execution
cannot address the physical Bluetooth adapter on the user's Linux host. A cloud
browser-only architecture could be evaluated separately, including current runtime,
streaming/provider media suitability, extraction compatibility on cloud IPs, auth,
state, limits and costs. No such migration is implemented here. Recommended current
path remains Vercel frontend + authenticated access to the existing Linux Rust backend;
that connection is still pending security/design approval, not unlocked by this CLI.

## Research register

- https://github.com/Mithronn/rusty_ytdl/tree/v0.7.4 — inspected tag Cargo features,
  Video/get_info API, VideoOptions/RequestOptions/VideoFormat and reqwest re-export.
  Main branch differs materially; do not infer0.7.4 APIs from main. License MIT/Apache2.
- https://vercel.com/docs/functions/runtimes — official runtimes and streaming limits.
- https://vercel.com/docs/limits — current function duration/resource constraints;
  recheck before architecture commitments, not an unlimited daemon guarantee.
