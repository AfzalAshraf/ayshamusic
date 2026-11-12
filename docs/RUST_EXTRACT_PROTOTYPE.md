# Native extraction prototype — benchmark before replacing yt-dlp

Status: isolated experimental candidate, NOT production, live success/speed/memory
NOT YET VERIFIED. The first step requested is native extraction; the existing
Rodio audio experiment remains separate and unchanged. No service/tunnel/Vercel
backend migration occurs. Only access content you are permitted to use.

## Actual implementation

Current candidate: `prototypes/rust-extract` pins upstream PR62 revision
`b390352a5db4b57aa630e8dc7b427fb560feccd5` from `restalex/rusty_ytdl`, default
features off, `rustls` only. The published crate0.7.4 candidate is rejected for the
reported host/track (all compatible AAC URLs missing). See latest evidence below.
The fork still calls its package0.7.4; JSON candidate identity includes the revision
so it cannot be mistaken for the failed published artifact.
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
is just that one process; sampled_group_peak_rss_kib samples each dedicated tool group (including JS children)
roughly every20ms plus /proc scan overhead; may miss transient peaks and double-count
shared pages. It excludes the harness and is not service-cgroup accounting. Use the memory
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

## Verification outcome

Source27a0be8: GitHub Actions run43249516804 all4jobs PASS: root Rust/Python/Node,
browser regressions/static preview, existing audio codecs/null output, and new native
extractor5offline tests/release build/strict CLI smoke. No live extraction from CI.
Cargo unavailable in agent workspace; compile evidence is Linux CI, not local/host.
Resolved lock artifact was uploaded by CI, but retrieving it here failed at the
artifact-storage network boundary; transitive dependencies remain unlocked in source.
A follow-up added tested Linux process-group RSS sampling and startup-inclusive
comparison; local diagnostic suite48tests PASS. Live network benchmark remains pending.


## First target-host report: failed native extraction, diagnosis required

User reports all5offline tests and release build passed on /home/afine host.
Supplied report timestamp2026-11-12T15:45:05.519136Z is retained as host evidence,
not the agent's clock. Native0/3: media_origin_rejected, sampled RSS25752/25476/22320KiB.
yt-dlp2/3: successful prefix3658.98/2871.86ms, median3265.42ms; failed run reported
short_or_non_mp4_probe. Baseline sampled RSS82976/89788/89872KiB. No speedup claim;
low memory for unsuccessful extraction cannot establish a working replacement.

The previous guard collapsed empty, malformed, wrong-scheme/credentials/port/host URLs
into media_origin_rejected. Therefore the report does NOT prove an untrusted hostname,
DNS/network failure or working signature extraction. Updated fixed reason codes
separate media_url_missing, media_url_invalid, media_scheme_rejected,
media_credentials_rejected, media_port_rejected, media_host_missing and
media_host_rejected. Same allowlist, no redirects, no metadata/signed URL logging.
No fallback format/tool added to disguise failure. Inspected upstream v0.7.4 utils.rs
parse_video_formats/set_download_url/decipher: formats can be returned with empty URLs;
this is a source-based possibility, NOT established host root cause.

Benchmark now retains native HWM on failure and adds byte count/header-presence/optional
observed exit code for baseline failures (no upstream bytes/error text). These classify
failure layers only; the failed baseline run is still undiagnosed. Run one isolated
native probe after the new offline tests pass to obtain the precise fixed code:

```bash
cargo test --manifest-path prototypes/rust-extract/Cargo.toml &&
cargo build --release --manifest-path prototypes/rust-extract/Cargo.toml &&
timeout --kill-after=2s 75s \
  ./prototypes/rust-extract/target/release/aysha-extract-prototype \
  --video-id Jm7Satp7Zqk
```

The user now wants all server duties in Rust and10–15MB; see MEMORY_BUDGET_PLAN.md
for the updated target and unresolved accounting/workload. Native failed RSS alone
already exceeds15MB. Do not promote or remove production helpers yet.

### Confirmed missing URL and selection correction

User's next host run: six offline tests PASS, release build PASS, final JSON
media_url_missing, VmHWM24956KiB (25,554,944bytes, about25.6decimalMB). This establishes
that the selected AAC format had an empty URL. It does NOT establish that all AAC
formats were empty or why upstream returned that selected record without a URL.

Found a local selection bug: maximum bitrate was chosen before validating the URL.
Now selection ranks only compatible AAC records with validated URLs. It can use a
lower-bitrate AAC only when it actually exists and passes the same origin guard;
it never invents a URL, retries another tool, changes codec/container or relaxes
origin restrictions. This fixes a reproducible fixture where an empty high-bitrate
record shadows a usable record. Live-host recovery is NOT yet demonstrated.

A fixed numeric format_counts object now exposes formats_total, aac_candidates,
empty_urls, invalid_urls, rejected_origins and usable_urls (no identifiers/URLs).
If every AAC URL is empty, error is all_aac_urls_missing. If none validate for mixed
reasons, no_validated_aac_url. If no compatible formats, aac_mp4_unavailable. Upstream
format availability is separate from subsequent HTTP/media/decoding success. After
this selection patch is tested, use the same one-probe build command above, not the
full comparison yet. If all AAC URLs are missing, this candidate needs upstream
extraction repair or replacement, not another permissive allowlist patch.


## Published candidate rejected; pinned compatibility experiment

Latest host evidence at source1729086: ten tests/releasePASS; all_aac_urls_missing,
formats_total26, aac_candidates1, empty_urls1, invalid_urls0, rejected_origins0,
usable_urls0, VmHWM24868KiB (25,464,832bytes). AAC metadata exists, but no usable AAC
URL was supplied. Stop re-running the published0.7.4 build or changing local selection
for this result. This candidate failed the required workload; not production-ready.

Upstream review (GitHub API; claims below are source/issue evidence, not live success):

- https://github.com/Mithronn/rusty_ytdl/releases/tag/v0.7.4 — published2024-08-10.
- https://github.com/Mithronn/rusty_ytdl/issues/52 and
  https://github.com/Mithronn/rusty_ytdl/issues/53 — report unreleased compatibility fixes
  and stale crates.io distribution. The release number alone did not establish current
  functionality; initial selection should have checked this gap earlier.
- https://github.com/Mithronn/rusty_ytdl/commit/bfd7fedb8e871fb26b528cada0a4c837e645182a
  — later default android_sdkless client; not in published0.7.4.
- https://github.com/Mithronn/rusty_ytdl/pull/62 — still open, proposes android_vr because
  sdkless later returned403. Contributor reports some standard-video success, with
  child-directed/live cases not established. These reports do not prove our missing-URL
  root cause or today's host success. Do not call it a merged/released upstream fix.

Next experiment pins that PR's head exactly to b390352a5db4b57aa630e8dc7b427fb560feccd5.
Reviewed diff against official main b1c6eb7: only client configuration and selection
changed; inherited source also has visitor-data/client compatibility changes absent
from the published crate. No arbitrary latest-head dependency, no credentials/tokens,
no Python/Node process added, no local signature bypass. Only public permitted content
is in scope. Local media-origin/format/size/time guards remain. No own patch to special
restricted-content handling is authorized. Upstream dependency still contains broader
capabilities than this prototype's accepted scope.

This is a THIRD-PARTY, UNMERGED experiment, not a safe production dependency declaration.
Source dependency changes reqwest0.12 to0.13, Boa0.19 to0.21 and supporting libraries;
Cargo feature is now rustls (not rustls-tls). Direct revision is immutable, transitive
lock is still resolved locally/retained in CI artifact, not committed in source.
Never remove/reset the host lock manually; Cargo updates it for the new manifest.
Offline tests/build must pass first. A host rebuild may be substantially longer because
of dependency changes. If it fails, report that failure; do not install anything.
Only after verified compilation is one same-host probe useful. New candidate string
is emitted on success/failure and allowlisted into benchmark reports.

Alternative assessed, NOT adopted: TeamPiped/rustypipe mirror advertises broad native
Innertube/search support, but inspected manifest has GPL3, rquickjs (native QuickJS,
not wholly Rust), wreq and JS parser dependencies. It is not a drop-in MIT/Apache2,
no-JS-engine or proven15MB replacement. No GPL dependency is introduced here.

Recovery: this experiment only changes the isolated prototype's dependency/source.
Do not install/promote it. The root server, installed service, Vercel preview and
Rust audio prototype remain unchanged; no production rollback needed. Earlier
published-candidate source/evidence is retained in Git history at1729086.
