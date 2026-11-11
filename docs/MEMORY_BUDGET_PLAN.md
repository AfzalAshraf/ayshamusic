# 100 MB concurrent-playback budget — approval and verification plan

Status: requested target, architecture not yet approved or implemented. Active department: QA, Security & Debugging. This updates the earlier 20 MB target; it is not evidence that current playback fits within 100 MB.

## Contract and decisions

- Target: at most 100,000,000 bytes (decimal 100 MB, conservatively below 100 MiB) for the agreed server workload. No supported listener count is promised before it is selected and measured.
- Recommended accounting boundary: the complete aysha-music service cgroup, including Rust, yt-dlp, Node, mpv and all their descendants, not just the Rust process's RSS. Include charged file cache/kernel memory in the measurement, not just application heap.
- Ask whether cloudflared must also fit in the same aggregate budget. It is not deployed today; do not silently exclude it if the user means the entire music-serving stack. Phone browsers and unrelated host services are separate processes; state exclusions explicitly. PipeWire is currently a shared user service, not a child of this unit.
- Ask maximum simultaneous devices and whether listening is independent or synchronized. Recommendation: independent browser playback per device, with the single home Bluetooth speaker as a separate explicitly selected output. This requires approval; shared global state is not independent playback.
- Preserve Rust as the production server and approved mobile design. Existing extraction uses yt-dlp/Node; Rust implementation of scheduling/proxy/session control does not turn those subprocesses into Rust or make their memory free. A requirement to eliminate those tools entirely is a separate compatibility/replacement decision.

## Demonstrated source/host evidence

The user's journal reported a previous service invocation peak of 120.3M. That is a service-accounting figure, not attribution to Rust alone or a trace of concurrent listeners. The old 20 MB target was not demonstrated.

Current code launches one yt-dlp process per media request; repeated and simultaneous clients can multiply expensive work. Search fallback and related-track prefetch also spawn downloaders without a shared admission controller. Search response JSON and subprocess output have no explicit byte budget. Player state is global; browser starts issue a global mpv kill. Merely serving several HTTP streams does not establish isolated multi-device playback.

## Proposed implementation sequence (approval needed)

1. **Account before changing limits.** Sample cgroup-v2 memory.current, memory.peak, memory.stat, memory.events and swap values alongside per-process PSS/RSS during idle, cold extraction, warm playback, search, prefetch and Bluetooth. Record available host memory, service settings, workload and exact revision. Read accounting permissions/errors honestly; missing cgroup information is not zero usage.
2. **Centralize resource admission in Rust.** Bound expensive extraction/search/prefetch work globally, not with a semaphore on /api/stream alone. Prefer one cold extraction at a time initially, sharing an in-flight job for identical tracks. Bound pending requests and their timeouts; reject excess work visibly without stopping another listener. Ensure cancellation terminates/reaps the owned job and releases permits. Count mpv extraction/runtime work too. A one-worker setting is a candidate to measure, not proof that one yt-dlp/Node peak fits the entire budget.
3. **Separate preparation from fan-out.** Evaluate bounded on-disk audio preparation or validated direct-origin streaming so prepared audio is not downloaded through a fresh yt-dlp process for each listener. Rust serves small backpressured chunks; no whole song in a Vec and no unbounded RAM cache. If disk storage is approved, cap total size, per-track size and lifetime, pin active files, evict safely and clean interrupted writes. Disk file cache is still charged memory and must be measured. Do not introduce transcoding or large resident workers without a measured budget. Admission delays cold starts; simultaneous playback need not mean simultaneous extraction.
4. **Keep sessions independent if approved.** Bound active/idle sessions, metadata sizes, queues, history and likes; expire unused sessions. Remove browser control actions that kill the shared physical-speaker process. Test ownership and stale-request races separately from memory limits. Do not silently change likes persistence/sharing.
5. **Use this work to address, not hide, iPhone limitations.** A prepared-file option could serve browser-compatible AAC/M4A when available, with valid HEAD/Range/206 responses. A direct-origin option must preserve correct headers and validate redirects/targets. Either needs browser gesture handling and real iPhone tests; no compatibility guarantee follows from changing a MIME label. The existing iPhone failure remains undiagnosed without its native error/version.
6. **Enforce only with truthful behavior.** After measuring and reducing peaks, apply a soft pressure threshold below the agreed ceiling and a systemd/cgroup hard ceiling as a final backstop for the entire agreed process group. If a single required extraction cannot fit, report that and revise the tool/path/budget, not claim success by hiding the worker in another service. A hard ceiling can cause OOM kills: it is not successful optimization. Do not use swap as hidden extra capacity or enable an automatic restart loop that disrupts all listeners. Acceptance requires no OOM events and continuous playback for the supported workload, not just a configured MemoryMax.

No hard cap, daemon change, automatic update, cache or new playback architecture is applied by this document. Do not ask the user to reinstall for this proposal.

## Acceptance matrix

Choose the maximum device count first, then exercise:

- Idle, one listener and the agreed maximum, with same-track and distinct-track playback.
- Cold starts, warm starts, search during playback, related autoplay, rapid skip/stop, reconnect, refresh and abandoned requests.
- Slow clients and attempted excess sessions/jobs: bounded pending work and honest retry/busy responses, without terminating healthy streams.
- Bluetooth simultaneously only if included in the target; include the configured extraction runtime and shared-audio-service accounting boundary explicitly.
- iPhone and Android actual audio, seek/reload behaviors and independent controls (if chosen).
- Long-duration run plus restart recovery, cache cleanup and bounded metadata growth; capture cgroup peak/events rather than relying only on interval samples.

Pass criteria: memory remains within 100,000,000 bytes under the agreed accounting boundary throughout accepted load; no OOM/OOM-kill increments, no hidden out-of-budget workers/swap dependence, no lost healthy streams, no cross-device control interference if independent mode is approved. Record actual capacity and test duration; do not promise arbitrary/unlimited devices within a fixed budget. Overall readiness scores do not increase on a proposal alone.

## Native Rust replacements: feasibility, not a completed rewrite

The user asks whether mpv or yt-dlp can be replaced with Rust. Separate their responsibilities:

- **mpv's audio-only role:** a candidate is a Rust playback worker using [Rodio](https://github.com/RustAudio/rodio) and [Symphonia](https://github.com/pdeljanov/Symphonia) for playback/decoding of an already resolved audio source. Their public repositories were checked through the GitHub API and describe audio playback and pure-Rust demuxing/tag reading/decoding respectively. This is not a tested dependency selection: codec availability, bounded buffering, seek/pause/volume, Linux output-device selection, PipeWire/ALSA integration, reconnect behavior, licensing/features and memory must be verified before removing mpv. Native OS audio drivers/services still exist outside Rust. It would not fix phone-browser media delivery by itself.
- **yt-dlp's role:** extraction/search, changing YouTube player/signature logic, format selection and URL acquisition must be replaced, not just its process launcher. A Rust wrapper that still executes yt-dlp or Python is not a Rust extraction replacement and does not eliminate its memory. No native extraction library is verified/selected here. Check actual supported URLs, current YouTube behavior, cancellation, protocol/security bounds and memory before recommending a candidate. A first candidate repository lookup failed with HTTP 404; do not treat it as an available dependency or infer all native alternatives are unavailable.
- **Dependency order:** the current mpv path accepts a YouTube page URL and relies on extraction hooks. A Rust decoder alone cannot consume that page URL as audio; it needs a validated/resolved stream or prepared file. Prototype helper replacements behind explicit development configuration while preserving the working production path. Never silently fall back to an external tool while reporting a Rust-only test result.

Recommended first experiment after architecture approval: resolve/prepare one supported track, play it through a bounded Rust audio-only path, measure the complete process group and compare to mpv. Then exercise a native extraction candidate separately. Only switch production when required formats, target hardware and the chosen concurrent-device workload pass. The frontend remains browser HTML/CSS/JavaScript; Rust replaces server responsibilities, not the phone's browser engine. This proposal does not guarantee that replacing either tool alone achieves 100 MB.
