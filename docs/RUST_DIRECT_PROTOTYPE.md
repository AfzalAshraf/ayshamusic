# Bounded native direct-player experiment

Status: isolated new mechanism; NOT a working production replacement or 15 MB claim.
The old rusty_ytdl/Boa candidates remain failed for the required host/track. This
experiment does not erase that evidence or add a hidden fallback. Production5050,
UI/Vercel, root dependencies, installer and Rodio remain unchanged.

## Why a separate experiment

User approved further fixing/building and asks for terminal compile instructions.
Instead of another large extractor revision, prototypes/rust-direct uses reqwest0.12
(defaults off, rustls/json), single-thread Tokio, Serde/serde_json. No rusty_ytdl,
Boa, QuickJS, Node, Python, yt-dlp, mpv, browser engine or subprocess in this path.
It is Rust application code with ordinary native TLS/OS dependencies, not a claim
that every cryptographic implementation or OS library is written in Rust. It does
not include search, related tracks, full downloading, decoding or playback.

One fixed HTTPS public-player POST uses ANDROID_VR context with client version1.65.10
and a matching agent/header. This differs from the failed library path: no HTML
page/config/visitor extraction, no intermediate downloader parser or JS evaluation.
Protocol parameters were inspected at https://github.com/yt-dlp/yt-dlp/blob/master/yt_dlp/extractor/youtube/_base.py
(android_vr configuration). No extractor code is copied; this is a small independently
written protocol request/parser. Important: that source also describes current proof-
of-origin requirements. No token generation, credential collection, sign-in, age/
content-restriction flags or restriction bypass is implemented. If the public response
cannot be used without such requirements, the result is a failure, not a workaround.
Do not claim the client name means compatibility or exemption from provider controls.

## Bounds and safety

- Strict11-character ID; no arbitrary metadata URL/endpoint input.
- HTTPS-only, no proxy, no redirects; fixed player endpoint, no credentials/cookies.
- Read at most1MiB metadata into an owned byte buffer; oversize/chunked overrun fails.
  Parser skips unknown fields with typed structures. HTTP/TLS internals/allocations
  are not proven heap-bounded by that1MiB payload limit.
- Require playability OK, matching video identity and non-live content. Former live
  content is conservatively rejected too. No sign-in/age/private access is attempted.
- At most256 adaptive formats accepted after parsing; parsing occurs within metadata
  byte cap. Pick highest bitrate AAC/mp4a.40.2/audio-MP4 direct URL, no video dimensions.
- Cipher-only entries fail signature_required_unsupported. Missing URLs fail honestly.
- Media URLs limited to HTTPS googlevideo.com subdomains, no credentials/alternate port;
  no redirects. Hostname validation is not a fully hardened public proxy/SSRF service;
  the CLI is local only, no server endpoint exposed. DNS/network/library behavior still
  needs review before any public integration.
- Read/validate64KiB MP4 prefix and discard it. Require ftyp signature at bytes4..8.
  This is not a full MP4 integrity/decode/seek/audibility test. Range ignored by upstream
  can cause additional buffered/network bytes before cancellation.
- Connect10s, per-request20s, overall60s cooperative deadline plus recommended75s
  external kill deadline. No retry/cache/file output. No URLs/response bodies/provider
  reason text logged; only fixed codes/timings/itag/process HWM.
- Code0=prefix succeeded,1=probe failed,2=arguments invalid. No fallback.

## Build on the actual Linux host

Only after CI compile/tests pass, use the new package path (NOT rust-extract):

```bash
cd /home/afine/ayshamusic &&
git pull --ff-only origin arena/a850fb0d-ayshamusic &&
cargo test --manifest-path prototypes/rust-direct/Cargo.toml &&
cargo build --release --locked --manifest-path prototypes/rust-direct/Cargo.toml &&
timeout --kill-after=2s 75s \
  ./prototypes/rust-direct/target/release/aysha-direct-prototype \
  --video-id Jm7Satp7Zqk
```

No sudo/install/service restart, old lock deletion or OS changes. First cargo test
creates this new package's own lock; release uses --locked. Preserve it locally.
Build artifacts ignored. Cargo lock snapshot retained as CI artifact; transitive
lock is not committed. Compile/download RAM/size is not runtime memory. Pause audio
before performance measurement to reduce contention, without restarting the service.

If the result fails, retain that as a failed experiment. Do not replace production.
If it passes, record cold/warm successes and same-format/host comparison including
full service cgroup before performance claims. The existing benchmark harness still
selects the old rust-extract binary; it does NOT benchmark this candidate. Do not
mislabel its old results. Full song/audio/production integration is not implemented.

## CI evidence model

Job rust-direct-prototype runs six offline tests, release build, CLI validation and
asserts the resolved graph has no extractor/JS-engine crates. A bounded public probe
of the already user-tested ID is recorded as a GitHub notice, not an acceptance gate.
Its process exit and JSON status remain explicit even when upstream fails. Green
build job means code/tests passed, NOT a successful network observation. Cloud runner
IP/region may differ from the actual host. No network result can certify the user's
hardware/aggregate15MB budget. No server secrets or private track corpus is uploaded.

## Recovery and limits

No production rollback required: don't run/integrate this isolated binary. The old
experiment remains for reproduction only. A native direct request cannot decode
signatures, generate integrity tokens, bypass provider restrictions or implement the
whole maintained YouTube extraction ecosystem. If that is required for the accepted
workload, the Rust-only and15MB goals must be evaluated together, not solved by
moving workers outside accounting or quietly enabling yt-dlp/Node fallback.

## Actual first verification outcome — do not call this fixed

Sourceb845189, GitHub Actions43264608690: all5code/regression jobsPASS, including
six direct-prototype unit tests, release build, CLI, noJS-engine dependency audit,
existing root/audio/browser suites. The public compatibility observation separately
FAILED: candidateaysha-direct-v1-android-vr, codeplayer_not_playable, process_exit1,
VmHWM4088KiB (~4.19decimalMB). The low failed-request memory is NOT working extraction
or a15MB full-service acceptance. Provider did not return playabilityOK on that cloud
runner; no raw reason was logged, so do not infer bot/IP/auth/region/video removal as
confirmed cause. Actual Linux-host behavior is unknown, not silently assumed better.
Do not promote this experiment or ask for another home rebuild as a guaranteed fix.

For the user's request to compile the best currently verified working server, use
root package (which still uses the existing yt-dlp/Node/mpv production helpers):

```bash
cd /home/afine/ayshamusic &&
git pull --ff-only origin arena/a850fb0d-ayshamusic &&
cargo test &&
cargo build --release
```

Output target/release/music-server is compiled only; it does NOT replace/restart the
installed service. Do not launch a second5050 instance while production owns the port.
This is not a new Rust-only extraction fix and does not claim100MB or15MB compliance.
The direct-package build command above remains an optional experiment/reproduction,
not the best working production build. Avoid confusing these paths or invoking the
installer merely to compile. Native extraction-to-playback migration remains blocked.
