# Memory budget — Rust-only target and verification plan

## Current requirement override —2026-11-07

The latest user request supersedes10–15MB: **strictly below30MB TOTAL**, every
application-server responsibility in Rust. Decimal30,000,000bytes is the conservative
ceiling; equality fails, MiB31,457,280bytes is shown separately. Historical evidence
and chronology below retain their original15MB thresholds, not the current target.
The measured service alone101,171,200bytes still fails both30-unit ceilings. No new
measurement or native playback success is inferred. Shared audio/BlueZ/D-Bus/tunnel
costs and simultaneous listener count must not be omitted. See ALL_RUST_UPGRADE_GATES.md.
The current live diagnostic uses candidate v2-30mb; committed v1reports stay immutable.
No unchanged host probe is needed. Native acquisition is still blocked; watch approval
is consumed. Preserve the working service; no prototype install/restart is authorized
by a low-RSS or passing build result.


## Latest user requirement: 10–15 MB and Rust server responsibilities

The user now requests every server responsibility in Rust and a 10–15 MB memory
target, replacing 100 MB as the desired optimization goal. Neither target has been
demonstrated. The sections below retain the previous 100 MB proposal as history,
not the current acceptance ceiling. Clarify decimal MB vs MiB, whole service cgroup
vs process RSS, tunnel/shared audio inclusion, and supported listener workload
before claiming compliance. Do not reinterpret the target as Rust HTTP-only memory
while hiding extraction/audio in another process or remote service. No hard cap
is installed: an OOM kill would not count as working playback.

User-supplied first native-extraction benchmark: Rust0/3 successful probes; sampled
process-group peaks22320–25752 KiB (about22.9–26.4 decimal MB). Even failed attempts
exceed the proposed15 MB ceiling for this candidate alone. yt-dlp2/3 successes,
82976–89872 KiB sampled peaks; successful prefix median3265.42ms. This is neither
a fair successful-path memory comparison nor a whole-stack measurement. Replacing
Python with Rust does not automatically bound TLS/HTTP/JSON/JS-engine/decoder buffers.

Current direction: keep production Rust; diagnose the failed candidate first; then
measure allocations and bounded buffering before selecting/rewriting components.
The production server must eventually remove yt-dlp/Node/mpv if the Rust-only scope
is accepted and proven, not merely rename their launcher. Browser HTML/CSS/JS and
OS audio drivers are not server responsibilities being rewritten. Python is currently
used only for opt-in diagnostic/test tooling as well as the separate cloud fallback;
its benchmark RSS is excluded from this report, not evidence of runtime compliance.
Any requirement to remove development diagnostics too needs explicit scope agreement.

## Target-host direct request: low failed-request RSS is not acceptance

The lightweight v2direct prototype now also returnedplayer_bot_check on the actual
Linux host (412d4c7), eight tests/releasePASS, VmHWM4404KiB =4,509,696bytes. No playable
sample was obtained. This confirms a source-access rejection for that request, not
successful extraction/playback within15MB. Stop identical retries; resolve approved
source-access design before media integration. Production remains unchanged and its
full process/cgroup budget is not represented by this isolated failed-request RSS.

## Latest failed-candidate measurement

User's pinned candidate at source566a812 built and passed10tests after the lock fix,
then failed all_aac_urls_missing (2AAC records, both empty;23formats total; no origin
rejections). VmHWM25664KiB =26,279,936bytes, about26.3decimalMB. This is an unsuccessful
standalone extraction peak, NOT successful media-read/decoding/streaming or full service
memory. The reported compile/download activity is not server runtime RSS. Both tested
AAC extraction candidates are now blocked for this workload; stop identical rebuilds.
Do not introduce a15MB MemoryMax/OOM workaround or move helpers outside accounting.
Next native architecture needs a feasibility/design gate before further integration;
see latest checkpoint in RUST_EXTRACT_PROTOTYPE.md. No performance acceptance or
whole-stack resource promise can be made from the current results.

A subsequent lightweight direct-player prototype (`prototypes/rust-direct`) removes
extractor/JS-engine crates. Its first CI public probe FAILED player_not_playable at
4088KiB VmHWM (~4.19decimalMB). This is not usable extraction, an improvement in
successful-path memory, or full-stack15MB compliance. See RUST_DIRECT_PROTOTYPE.md;
production still uses existing helpers. Keep failures distinct from performance wins.

## Historical 100 MB plan (not achieved)

Status: 100 MB target not yet implemented/verified; concurrent-device architecture still awaits capacity/accounting/session decisions. The user has approved the first isolated audio-only Rust experiment, now implemented in `prototypes/rust-audio`; see [its build/test guide](RUST_AUDIO_PROTOTYPE.md). This does not switch production away from mpv/yt-dlp. This updates the earlier 20 MB target; it is not evidence that current playback fits within 100 MB.

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

- **mpv's audio-only role:** a candidate is a Rust playback worker using [Rodio](https://github.com/RustAudio/rodio) and [Symphonia](https://github.com/pdeljanov/Symphonia) for playback/decoding of an already resolved audio source. Their public repositories were checked through the GitHub API and describe audio playback and pure-Rust demuxing/tag reading/decoding respectively. Rodio 0.21.1 is now selected for the isolated prototype, with WAV/M4A/MP3 fixture decoding and null-output playback passing CI. Production codec availability, buffering, seek/pause/volume, Linux output-device selection, PipeWire/ALSA integration, reconnect behavior, licensing/features and aggregate memory must still be verified before removing mpv. Native OS audio drivers/services still exist outside Rust. It would not fix phone-browser media delivery by itself.
- **yt-dlp's role:** extraction/search, changing YouTube player/signature logic, format selection and URL acquisition must be replaced, not just its process launcher. A Rust wrapper that still executes yt-dlp or Python is not a Rust extraction replacement and does not eliminate its memory. No native extraction library is verified/selected here. Check actual supported URLs, current YouTube behavior, cancellation, protocol/security bounds and memory before recommending a candidate. A first candidate repository lookup failed with HTTP 404; do not treat it as an available dependency or infer all native alternatives are unavailable.
- **Dependency order:** the current mpv path accepts a YouTube page URL and relies on extraction hooks. A Rust decoder alone cannot consume that page URL as audio; it needs a validated/resolved stream or prepared file. Prototype helper replacements behind explicit development configuration while preserving the working production path. Never silently fall back to an external tool while reporting a Rust-only test result.

The approved first experiment is now implemented: prepare one supported track, play it through the isolated Rust audio-only path in prototypes/rust-audio, measure actual host output/memory and compare to mpv; see RUST_AUDIO_PROTOTYPE.md. Then exercise a native extraction candidate separately. Only switch production when required formats, target hardware and the chosen concurrent-device workload pass. The frontend remains browser HTML/CSS/JavaScript; Rust replaces server responsibilities, not the phone's browser engine. This proposal does not guarantee that replacing either tool alone achieves 100 MB.


## Latest native context experiment — still no successful low-memory extraction

Hostb2efd81directv3 passed11offlinetests/releasebuild, then gotplayer_bot_check with
4404KiBVmHWM (4,509,696bytes). v2hadthesamerejection/HWM. The additionsofOrigin/UTC
were insufficient for this attempt. Do not count eitherfailedrequest as a working
10–15MB extractor, decoder or server. BaselinetracedAACsuccessused98672KiBsampled
processgroupRSS; not cgroup/wholeappbudgetproof. Stopidenticalnativevariants. The
offlinegate scripts/check_native_acceptance.py explicitlykeepsRAM/productionfalse
and rejects buildsuccess/baselinepass as substitutesfornativebytes. Anonymouswatch
bootstrap remains a scopeproposal requiringapproval, not a demonstrated memoryfix.

Latesthostv4bootstrapat120c5fe:21tests/releasePASS, thenstagewatch/watch_literal_invalid,
5764KiB (5,902,336bytes). No playerPOST/mediaGET reached. ThisisfailedwatchparserRAM,
notworkingextraction/wholebudget. Parseronlyv4.1correctionhaspassedofflineCI; runtime
memory/successunproved. Samepayload/deadlineboundsretained, noRAMtargetraised.

Hostv4.1at5dd69ff:25tests/releasePASS, stagewatch/watch_config_invalid,6016KiB
(6,160,384bytes) failedconfigdecodebeforeplayer/media. V4.2nextiswatch-onlydiagnostic,
notacquisition; hostdebugbuildRSSmustnotbecomparedasreleasebudgetproof. All-Rustworking
sourceaccess/decoder/server10–15MBbudgetremainunproved. No capchange/hiddenhelper/OOMfix.

Hostv4.2syntaxdiagnosticat1bae4b4:28tests/debugbuildPASS, stagewatch/
watch_config_json_syntax,player/mediarequestedfalse,10848KiB=11,108,352bytesdebugHWM.
Do not count asreleaseperformance, successfulmedia/decoderorwhole10–15MBbudget.
V4.3fixedsyntaxsubtypecodehaspassedCI; not anacquisitionorRAMfix, no newhelpers/caps.

Hostv4.3at127817ddebugbuildPASS: stagewatch/expected_value, no player/media,10964KiB
(11,227,136bytes) failed diagnosticHWM. V4.4discarded-metadata compatibilityadds atmost
1MiB owned output buffer plus bounded scanner bookkeeping when used; no full Value
objecttree or externalhelper. Input/output logicalcaps are not wholeheap accounting.
All-Rust source acquisition/playback/search/fullserver10–15MB acceptance unproved;
debug failed-parserRSS cannot certify it. FinalofflineCIpassed; runtime resultpending.

## Consolidated v5 data reader; resource gates remain real acceptance work

Latest host watch-only failure at5142200 was a single-quote-leading value, debug HWM
10,928KiB (11,190,272bytes), no player/media request. V5 consolidates static literal
parsing and synthetic pipeline tests; it is not a demonstrated extraction, playback
or memory fix. Strict JSON is borrowed; canonical non-JSON output <=1MiB plus temporary
input-bounded strings/parser state, no Value object tree. HTTP/TLS/Serde/allocator and
complete audio/server costs are still not proven within10–15MB. No hidden subprocess,
worker offload, MemoryMax/OOM policy, swap trick or resource target increase. Native
acquisition must pass before replacing production extraction/player helpers; then
actual decoder/control/search/whole-stack release/device tests, not a pass inferred
from successful syntax fixtures or failed debug-process RSS.

### V5 target-host result and v5.1 wrapper correction

Host39dd9fe debug build1.64s PASS, watch_visitor_missing before player/media, HWM10884KiB
(11,145,216bytes). Not successful acquisition or whole-server/release15MB compliance.
V5.1 fixes a known archived static-wrapper parsing gap; no confirmed current-host cause
or live media/memory pass. Fixed per-invocation count/boolean observations add no raw
page storage; wrapper parsing has fixed token/state overhead and reuses bounded1MiB
config/message readers sequentially. Still no complete heap/TLS/audio/server budget
certification, hidden helpers/offload/OOM acceptance or production switch.

### V5.1 failed-host peak is not a complete memory pass

Latest04e3f8b host debug3.16sPASS/watch_visitor_missing, player/mediafalse, HWM10632KiB
(10,887,168bytes). Config1/player1 recognized but allvisitor-pathflagsfalse, wrapper0.
No native prefix, decoder/audio output or complete release budget acceptance. Graphify
code-only graph was built in an isolated agent-workspace Python dev environment, never
as a production/helper process; its graph/library/cache remain outside Git/runtime. It
maps source, not working memory or provider-page contents. No extra live trial scheduled.

### Optional coverage inventory is not a resource-budget fix

Latesthostefde115ran onlyofflineguard: no native/networkexecution or newRSS evidence.
Preparedwatch-onlycoverageinventory owns fixedcounters, scans boundedexistingpage
slices withoutstoringvalues, and isdisabledinotherprofiles.62Rusttests/releaseCIpassed,
but no newhostwatchdiagnosticapproved/run. SameTLS/allocator/decoder/server whole-memory
uncertainties; noclaim10632KiBfaileddebugpeakdemonstratesall-Rust10–15MBstack.

### Actual-playback service lower-bound audit now available, not100MB legacy health acceptance

Userexplicitlyrequestsrealserverstream/budgetcheck. Newreadonlydiagnostic
scripts/check_playback_budget.py samplescurrentservicecgroup+descendants duringuser-
controllednormalplayback, separatehistoricalpeak/swap/allowlistedRSSbuckets andAPIflags.
20samples2sapartdefault. Compares15,000,000vs15,728,640bytesexplicitly. Currentabove
upperlimitrejectsservicebudget(lowerboundenough); belowlimitnevercertifieswholestack,
audibility/nativeacquisition or outsideaudio/tunnel/browser/escapedworkers. Countersnot
reset/RSSnotsummed/noMemoryMax/swaptrick/helperoffload. No servicecontrol/build during
measurement.100Python/62Rust/all5CI43417518725PASS; no hostmeasurement yet. See
STREAM_BUDGET_CHECK.md for orderedonewatch-onlydiagnosticthenusualplaybackaudit.

### Measured current service fails15MB/15MiB, not just historical100MB health warnings

Newhost3839d5c two20-samplewindows: currentmax100,941,824 then101,171,200bytes
(101.1712MB/96.484375MiB). SecondAPIplayingtrueatbothends/stable/in-cgroupinventory
complete. Histpeak218,480,640bytes separate/unaltered; secondswap905,216bytes.
RustRSSmax13,381,632/yt-dlp71,299,072 non-atomicnotsummedwithcgroup. Servicealone
failsboth15,000,000and15,728,640upperlimits; outside/sharedcostscannotreduceit.
Nativev5.2watchonlystillvisitor_missing/no player/media and11104KiBfailedprocessHWM
notworkingstack. Recordbudgetfailureandreplacementblocked; no moreunchangedmeasurements.
No proofhardware/futureRustcannotmeetbudget, norpromiseitcan; removalofyt-dlpalone
notall-Rust/audio/discovery/device/wholememoryacceptance. Fullresponsibilitygatesin
ALL_RUST_UPGRADE_GATES.md, no OOM/reset/offload/targetincrease/fakepass.
