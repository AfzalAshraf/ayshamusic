# Completed host stream/budget checks — both gates failed; do not repeat

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


## Latest results and current decision

Both commands below have now been run at checkout3839d5c. They are retained as the
historical procedure, **not instructions for another unchanged run**.

- Watch-onlyv5.2: build1.62sPASS, watch_visitor_missing, no player/media requests.
  Raw visitor-name hits occur in2ignoredtails, not parsedconfig/player/skippedattributes.
- Two20-sample service windows: sampledmax100,941,824 and101,171,200bytes. Second
  start/endplayingtrue, servicestable; yt-dlpRSSmax71,299,072bytes, RustRSS13,381,632bytes.
  Historicalpeak218,480,640bytes remains separate; swap905,216bytes in secondwindow.
- Currentservicealone **fails both15MB and15MiB**. No audibility/outputmode/devicecount
  confirmation or full native acquisition. This is a failure, not a ready upgradedserver.
- Watch-onlyapprovalconsumed. No newrequest/bootstrapretry/install/restart. See
  [all-Rust replacement gates](ALL_RUST_UPGRADE_GATES.md) for the actual fullserver
  responsibilities and ordered native implementation/release requirements.

## What can be established now

User explicitly requests checking whether this server can download/play streams within
10–15MB. The latest actual native attempt still stops at watch_visitor_missing before
player/media requests. Seven coverage fixture tests passed twice on the host, most
recently cached compilation0.10s/tests0.12s,55filtered. These are not live acquisition.

Current production playback remains the existing Rust server with external extraction/
player helpers. Do not describe it as a completed all-Rust stack. Earlier supplied
service readings were roughly87MB and exceeded15MB; those are historical, not fresh
measurements of this requested workload. More physical RAM is not proof that the app
fits a15MB resource budget.

The explicit request is recorded as approval to proceed with the **already prepared
one watch-only diagnostic** plus read-only local running-service measurement. This
minimizes access: no additional endpoint, credentials, cookies, generated visitor/token,
JS, fallback, client rotation, automatic playback or new bootstrap/player/media trial.
No production code/dependency/service or installer change. Python diagnostic-only.

A working all-Rust download→decode→audible-output→controls chain is not yet available
for honest whole-pipeline certification. No command below claims to test that missing
chain. The first check finds missing page coverage; the second can reject the budget
using the actual working service as a measured lower bound, during normal user playback.

## A. BUILD + ONE WATCH-ONLY COVERAGE DIAGNOSTIC

No need to rerun coverage tests. Cargo test built the test harness; explicitly build
the normal CLI to avoid running an old debug executable:

```bash
cd /home/afine/ayshamusic &&
git pull --ff-only origin arena/a850fb0d-ayshamusic &&
cargo build --locked --manifest-path prototypes/rust-direct/Cargo.toml &&
timeout --kill-after=2s 75s \
  prototypes/rust-direct/target/debug/aysha-direct-prototype \
  --video-id Jm7Satp7Zqk --watch-inspect
```

One fixed HTTPS watch GET, page<=2MiB, 60sasync/20srequest/10sconnect deadlines,
no redirects/proxy/persistence/JS. **Never player/media**, even if parsing passes.
Expected identity `aysha-direct-v5.2-coverage-only`, `watch_parser_only:true`, player/
media_requestedfalse, `watch_observation.coverage` with raw-name counts/prefix buckets.
It may still report fail/watch_visitor_missing (exit1): the new coverage fields are
useful diagnostic evidence, not a failed test-suite result. Do not rerun the request.
Raw markers can match strings/comments/dead code or miss escaped names: not JS syntax,
not exact host shape, not source authorization or working extraction.

If build/pull fails, stop and share normal output; preserve Cargo.lock. If restriction,
timeout124 or incomplete coverage, stop; no bootstrap retry or account/token workaround.
Share only normal fixed JSON, not raw page/header/cookie/debug traces.

## B. READ-ONLY BUDGET CHECK DURING YOUR NORMAL PLAYBACK

After A's build and request have finished, play a permitted song in the usual working
app, confirm you can hear it, then run this separately. Change songs once yourself
while the report is collecting. No builds/tests at the same time.

```bash
cd /home/afine/ayshamusic &&
python3 scripts/check_playback_budget.py --samples 20
```

Approximately38seconds of sampling, plus bounded local status/systemctl checks. Does
not start playback, click UI controls, call /api/stream, download anything, reset
counters, impose MemoryMax, install or restart. Only fixed loopback /api/status and
read-only systemctl/cgroup/proc metadata. No track, raw exception, URL, PID, cgroup path,
unknown process name, command line, environment, account or Bluetooth address output.
No automatic save/upload. Review ordinary JSON before sharing.

Report identity `aysha-service-playback-budget-v1`, schema1:

- Sampled service cgroup memory.current max (including descendant cgroups).
- Separate historical memory.peak (not reset, not attributed to this song), swap max.
- DecimalMB and binaryMiB readings; compares both15,000,000 and15,728,640bytes.
  Unit/boundary/listener-count agreement remains open, neither silently redefined.
- Allowlisted process-label RSS/count maxima, bounded64cgroups/256PIDs through existing
  helper. Non-atomic maxima may double-count shared pages: never add to cgroup totals.
- Service identity/restart stability; playing flag only at start/end. These flags do
  not establish uninterrupted downloading/decoding, genuine audible output or recovery.

### Interpret honestly

- `fail`, exit1, current above bothlimits: **the service alone exceeds either15MB
  interpretation**. Adding external/shared components cannot make it fit. This rejects
  the measured implementation's budget; it does not prove hardware cannot run a future
  optimized design. Above decimal but below binary is reported distinctly.
- `incomplete`, exit2: not whole-stack acceptance. Inspect specific code: inactive/
  unavailable cgroup, idle/unknown playback, service changed, swap, historical peak or
  sampled service under upperlimit. A low reading is never automatically success.
- This is a **lower bound**, not the complete proposed Rust stack: escaped workers,
  external/shared PulseAudio/Bluetooth/tunnel/browser services remain unaccounted.
  No hidden-offload or cgroup-only certification. Resource scope still needs agreement.
- Sampledmax can miss transients. Historical peak alone cannot prove this song exceeded
  budget. No all-Rust claim: helper presence is visible through allowlisted buckets.
- The checker always keeps whole_stack_acceptedfalse, audible_output_verifiedfalse,
  download_or_decode_verifiedfalse and native_acquisition_verifiedfalse. Your manual
  audibility observation is separate user evidence, not invented by the checker.

Send both normal JSON reports, plus **audible yes/no**, **browser or Bluetooth output**,
and **number of listening devices**. If budget exceeds15MB, prioritize actual owned
helper/decoder replacements and measured release work, not OOM caps or smaller failed
watch-probeRSS. If coverage identifies a reproducible gap, patch only with inert
fixtures/security negatives before any new native acquisition attempt.

## Verification and limitations

Source891b1c9 CI43417518725 all five jobs PASS (direct job131859082297), including
62Rust tests/release/no-JS graph/offline CLI.100Python tests locally and inCI PASS,
including14new budget tests and a new explicit-approval/
consumed-watch guard regression. Native implementation is unchanged in this budget
patch; optional coverage reader62Rust tests/release/noJSgraph previously verified inCI
and7coverage tests on host. Target-host budget and v5.2live reports have now been received and fail; no audible
output, native acquired media or successful15MB acceptance is established. CI never makes live provider requests or controls audio.
