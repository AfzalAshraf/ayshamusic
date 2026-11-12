# Native extraction prototype — benchmark before replacing yt-dlp

New separate limited direct-request experiment: [RUST_DIRECT_PROTOTYPE.md](RUST_DIRECT_PROTOTYPE.md).
The failed library candidate below is retained only for reproduction.

Status: **BLOCKED — both tested extraction candidates failed the required AAC probe
on the user's Linux host. Do not promote or recommend another identical rebuild.**
The lockfile migration is now host-verified; native AAC extraction is not working.
No speedup or 10–15 MB compliance is demonstrated. Build/probe instructions below
are retained for reproduction, not a recommended next action for this failure.
The existing Rodio audio experiment remains separate and unchanged. No service,
tunnel or Vercel backend migration occurs. Only access permitted content.

## Actual implementation

Retained failed candidate (for reproduction): `prototypes/rust-extract` pins upstream PR62 revision
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

## Existing-lock resolver failure (not an extraction result)

User's ace6189 update failed during Cargo resolution, before tests/build/media access:
Boa0.21 requires icu_normalizer~2.0.0, while locked idna_adapter1.2.2 requires
icu_normalizer^2.2 (host locked2.4.0). Cargo's minimal lock update kept the incompatible
adapter. The clean-checkout CI pass did not cover this upgrade path. Source verification:
https://github.com/hsivonen/idna_adapter/blob/v1.2.2/Cargo.toml requires ICU2.2;
https://github.com/hsivonen/idna_adapter/blob/v1.2.1/Cargo.toml permits ICU2.x.

Added an explicit idna_adapter=1.2.1 compatibility constraint only to the isolated
prototype manifest. It is a resolver constraint on an existing transitive dependency,
not another runtime worker. Cargo can reconcile the existing host lock in place,
selecting Boa-compatible ICU2.0.x. Do not delete/edit the lock or run a broad cargo
update, change the production manifest, or modify registry caches to work around this.
This is a temporary compatibility pin; review it when Boa/upstream updates. It does
not assert an older dependency is security-audited or that extraction now works.

CI now seeds a clean checkout with the reported conflicting constraint shape using
url2.5.8/idna_adapter1.2.2/ICU>=2.2, then runs the actual prototype tests against that
lock and verifies adapter1.2.1/Boa0.21/ICU-normalizer2.0.x. The seed helper refuses to
overwrite an existing lock and is CI-only. Do NOT run its seed mode on the user host.
It reconstructs the resolver conflict, not the user's entire private lockfile.

After this migration is verified in CI, the normal git-pull/cargo-test/cargo-build
sequence above is still sufficient. Existing host lock files are preserved and
updated normally by Cargo; no separate cleanup/install command is necessary. The
successful build, if obtained, still needs the one native media probe; this resolver
repair provides no live-compatibility, speed or memory acceptance.

Migration verification completed: source4359933, CI43262098027 all4jobsPASS, including
seeded conflicting lock -> actual10prototype tests -> graph/locked-metadata check ->
release/CLI. Actual successful CI annotation: adapter1.2.1, Boa0.21.1, normalizer2.0.1.
The initial verification assertion was too strict (exact Boa0.21.0), despite the
prototype tests passing; corrected to compatible0.21.x. This does not pin all
transitive dependencies or certify host media success. After cargo test updates the
host lock normally, cargo build --release --locked can preserve that resolved graph.


## Final host checkpoint: pinned candidate also fails — stop this retry sequence

User reports source566a812: Cargo successfully reconciled adapter1.2.1/ICU-normalizer
2.0.1/Boa0.21.1 without deleting the lock; ten testsPASS; release buildPASS (3m06s).
The actual pinned native probe then FAILED:

```json
{"candidate":"rusty_ytdl-pr62-b390352a5db4b57aa630e8dc7b427fb560feccd5","code":"all_aac_urls_missing","format_counts":{"aac_candidates":2,"empty_urls":2,"formats_total":23,"invalid_urls":0,"rejected_origins":0,"usable_urls":0},"peak_rss_kib":25664,"production_ready":false,"status":"fail"}
```

The dependency fix is confirmed on target. The client-request compatibility hypothesis
is insufficient to recover AAC for this host/track. Both compatible AAC URLs are empty;
no security guard rejected them. VmHWM25664KiB =26,279,936bytes (~26.3decimalMB), already
above15MB for this failed experiment alone. Compile time, downloaded crate sizes and
build-process RAM are NOT playback runtime memory. No successful Rust timing/memory
comparison exists. Do not optimize based on this failed path or promise a lower peak.

This result does NOT establish the exact upstream reason (missing data in server
response vs parsing/decipher/client behavior), availability of non-AAC URLs, a failure
for all videos, DRM, or an unavoidable need for any particular runtime. Do not guess
which of those occurred or remove the URL validator. No private/raw response dump or
credentials should be collected. The agent sandbox cannot make live YouTube requests;
CI fixtures/compilation cannot substitute for actual host success.

### Next department/design gate

Before implementing another candidate or large rewrite, seek approval for a scoped
Product & Architecture extraction feasibility review. Keep working production intact.
Define a small permitted/public track corpus, accepted codecs/output targets and
resource-accounting boundary; do not hide workers or move extraction remotely to meet
15MB. Evaluate dependency maintenance, licenses, embedded runtime/native-library needs,
response-size limits and cancellation before selection. Prefer a bounded standalone
proof of URL acquisition+media-read before full player integration or user rebuilds.
If investigation can be done with a small read-only release probe, do that before
asking for a multi-minute full build. Gate any new host probe on a concrete mechanism
change and offline tests, not another diagnostic-message rename.

For a bespoke native extractor, byte-bounded HTTP/JSON, measured JS/signature handling,
format support and restriction failures need a maintainable implementation and tests;
"no JS engine" is not an assumed viable design. A different codec may require changes
to the audio prototype and invalidates the earlier AAC-only timing comparison. Search,
related tracks, mobile playback and hardware controls remain separate acceptance gates.
No next architecture is approved or implemented by this checkpoint. Rollback is not
needed for production: only the isolated experiment/docs have changed throughout.
