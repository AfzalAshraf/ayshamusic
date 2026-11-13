# All-Rust replacement: requested upgrade, not a release-ready server

**Current decision: DO NOT replace the working service with a prototype.**
The user requests replacing every server responsibility with Rust, conditional on
working download/playback within10–15MB. The latest real tests failed both native
acquisition and the working service's memory budget. A successful test harness or
small watch-only process is not an upgraded server. No all-Rust release/install
artifact has been produced; no service switch occurred in this work.

## New target-host evidence at checkout3839d5c

- Normal prototype debug build1.62s PASS.
- V5.2 watch-only reader: watch_visitor_missing, no player/media requests, HWM11104KiB.
  One config/one initial player parsed, neither raw visitor name present there.
  Two of35ignored tails contain a raw visitor name; four contain rawytcfg. Eleven
  attribute-skipped bodies (three nonempty) have zero raw visitor-name hits.
  These counts do not establish usable values, exact syntax or runtime execution.
- Service window1:20samples, startplayingfalse/endtrue, currentmax100,941,824bytes,
  swapmax913,408bytes; historicalpeak218,480,640bytes.
- Service window2:20samples, start/endplayingtrue, serviceidentitystable, complete
  bounded in-cgroup process enumeration, currentmax101,171,200bytes =101.1712MB /
  96.484375MiB; swapmax905,216bytes. Historicalpeak still218,480,640bytes, not reset
  or attributed to this song/window. API flags do not prove continuous audible output.
- Window2 per-label RSS maxima: Rustmusic-server13,381,632bytes, yt-dlp71,299,072bytes.
  Non-atomic/maxima/shared pages: do not sum with each other or cgroup totals to claim
  exact attribution. yt-dlp is the largest observed process bucket, not evidence that
  deleting it alone achieves15MB or that remaining native work costs zero memory.

The service cgroup alone exceeds **both15,000,000 and15,728,640bytes**. Omitted shared
components cannot make this lower bound smaller. This rejects the measured current
implementation, not the hardware's ability to run any future optimized program.
There is no proof a complete future Rust design will fit15MB. Do not promise it.
Only1,618,368bytes remain below decimal15MB after the measured Rust-process RSS alone;
that arithmetic is a warning, not an additive heap model or future decoder/TLS estimate.

The approved watch-only attempt is consumed. No unchanged build, watch request or
memory rerun is recommended. Preserve production while the missing implementation
is developed and measured. Do not save/share raw watch pages, opaque visitors,
signed URLs, credentials or account cookies to accelerate this investigation.

## Actual replacement boundaries inspected in production source

| Responsibility | Current source/dependency | Required native replacement and acceptance |
|---|---|---|
| HTTP/API and approved UI | src/main.rs / src/index.html | Keep stable5050API, approved portraitUI/Back/history/queue/likes; regression test, not a template rewrite |
| Stream acquisition/proxy | src/media.rs open_stream launches yt-dlp | Native permitted-source extraction + bounded TLS streaming, matching identity/playability/privacy guards; actual media bytes, EOF/cancel/reconnect/range behavior |
| Search and related discovery | src/main.rs fetch_related and search spawn yt-dlp | Native metadata/discovery implementation with reviewed source access, bounded responses/concurrency, errors/cancellation; real representative results, no helper/off-host fallback |
| Speaker output and controls | src/main.rs play handler/audio_command uses mpv and socket IPC | Existing isolated Rust audio prototype is a candidate, not integrated: acquired audio decode, audible device output, pause/seek/volume/EOF/queue/recovery |
| Process/lifecycle ownership | pkill -f, dynamic audio command, id -u helper | Native owned handles, cancellation and reaping; no global process-pattern killing or shell helper required by the replacement |
| Bluetooth/device management | src/main.rs bluetoothctl and bash command paths | Reviewed Rust native/platform API clients; actual service-context device selection/reconnect tests, no pairing reset or silent device interference |
| Queue/history/related autoplay | src/playback.rs and src/main.rs | Preserve pure transitions, stop/start/refresh regressions and explicit user interactions; resource-bounded state, EOF tests |
| Release/install/recovery | scripts/install_local.sh currently installs the helper-dependent server | Do not call that installer an all-Rust upgrade. New root-server candidate must be integrated/tested before explicit backed-up switch and verified rollback |

Linux/systemd/audio/Bluetooth platform services remain platform services, not claimed
Rust application code. Their attributable resource boundary must be agreed and counted;
no silent workload offload, hidden Python/Node/mpv/yt-dlp worker, external resolver or
cloud fallback to manufacture a Rust/15MB claim. REA and Graphify remain development
tools only. No new auth/session/device-isolation/source-access architecture is implied
by this responsibility inventory. Existing global-state limitations stay explicit.

## Ordered gates before installation

1. **Fix acquisition with evidence.** Reproduce an ignored static-source shape using
   inert/value-free fixtures, not broad regex extraction or trusting raw marker hits.
   Preserve duplicate/type/auth/context/identity/live/restriction/header guards.
   Require real native media acquisition on representative permitted inputs. Stop
   restriction/signature/token blocks honestly; do not invent visitor/tokens or evaluate JS.
2. **Integrate only working native media/audio.** Native stream transport, decoder,
   audio output and controls must operate together; source access and audio/device
   side effects need their actual approvals. A64KiBprefix is not codec/full-playback proof.
3. **Remove every application helper role above.** Compile/run against the real root
   server API, not merely a separate CLI. Preserve UI and cloud-fallback boundaries.
   No silent functionality removal or deprecated client substitution to pass tests.
4. **Measure the complete release workload.** Idle, extraction/startup, normal playback,
   change-track peaks, EOF/recovery and intended listener count. Both resident and swap
   accounting, owned descendants/escaped workers and agreed shared-service costs.
   Decimal/binary units and simultaneous-device semantics remain to be agreed. No
   hard-OOM cap/counter reset as optimization. Target remains10–15MB, not increased.
5. **Switch explicitly, with rollback.** Only a verified integrated artifact goes to
   /home/afine/music-server under systemd, with backup, installed/running identity,
   health/audio/resource acceptance and recoverability. No prototype installation now.

## What the current patch actually does

Records the supplied native diagnostic and both service observations separately;
consumes the watch-only approval; recomputes the service lower-bound budget failure
in scripts/check_native_acceptance.py rather than trusting claimed status/pass or
small probeRSS. Reports native_replacement_allowedfalse and keep-working-service.
This is a diagnostic/readiness report, not an enforcement hook added to the installer.
Under-limit or malformed observations never certify the entire future Rust stack.
No runtime-performance, visitor-source or playback fix is claimed by an evidence patch.

**Optional OFFLINE review command—no need to repeat previous host tests:**

```bash
cd /home/afine/ayshamusic &&
git pull --ff-only origin arena/a850fb0d-ayshamusic &&
python3 scripts/check_native_acceptance.py
```

Expected blocked/exit1, current_service_budget.statusfail, native_replacement_allowedfalse,
nextoffline_reproduce_ignored_watch_source_before_native_replacement. No native build,
provider request, audio action, install or restart. Missing/invalid evidence means stop
and inspect the fixed code, not a new live trial. This command reviews committed
observations; it does not collect a new workload or certify a deployment.
