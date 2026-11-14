# Running-server identity and RAM output

The user confirmed audible phone/browser music with the opt-in rust-resolved
transport. Preserve that working path. Slightly slow Play/Previous/Next are reported,
not diagnosed; source has on-demand related discovery/resolution, but no measured
button latency has been supplied. Do not change quality or add speculative helpers
before measuring. Future server-output responses must include the latest supplied
RAM figures with time/scope, or explicitly say not remeasured.

## Latest supplied baseline — NOT a new measurement

| Observation | Value |
|---|---:|
| Configured transport | rust-resolved |
| Service cgroup current |6,451,200bytes =6.4512MB /6.15234375MiB |
| Historical service peak |67,592,192bytes =67.592192MB /64.4609375MiB |
| Service swap |0bytes |
| API playing flag / transfer lease |false /false |
| Hearing |User separately confirms music audible |

This single reading was not captured during a verified playing interval. A false
lease can also mean the phone has buffered audio; do not classify it as silent or
idle solely from that flag. Historical peak is not attributed to one action/song.
It exceeds50MB, so the complete strict50MB target has not been demonstrated.

## DIAGNOSTIC — about one minute, normal listening, no install/rebuild

On the Linux host as afine:

```bash
cd /home/afine/ayshamusic &&
git pull --ff-only origin arena/a850fb0d-ayshamusic &&
python3 scripts/monitor_server.py --samples 30 --interval 2
```

No sudo, raw journal dump, background service, remote upload, provider GET, player
control or build is performed. The monitor only reads systemd/proc/cgroup files and
fixed loopback /api/status. The loaded/installed binary hashes are read once before
the sampling window, not every sample. Run when builds/package updates are finished.
No need to restart the installed music service to use this Python stdlib diagnostic.

While it runs, listen normally. Around10seconds use Next once; around30seconds use
Previous once, if desired. Note approximate tap times and perceived delay; do not
repeatedly tap while a transition is pending. The monitor itself does not cause a
song change or probe upstream media. If you prefer no controls, just measure normal
listening and say which workload was observed. Do not reproduce a failed provider
request merely to collect a log.

It prints one compact progress line per sample and a final SERVER RAM SUMMARY.
The full timestamped timeline is saved privately (0600) to an ignored
`diagnostics/server-memory-*.json` file; only the summary is printed at the end.
No song names, URLs, argv, environment, raw paths, PIDs or arbitrary process names
are included. Hashes and machine state are operational information: review before
sharing. Send the summary plus approximate action times; the full file is available
if a specific sample is needed. No automated upload. Ctrl-C preserves collected rows
when Python receives it; an abrupt SSH/process termination may prevent final save.
This is an on-demand foreground diagnostic, not continuous SSH-independent monitoring.

For a quick future snapshot use `python3 scripts/monitor_server.py --samples 1`.
Bounds:1–60 samples, interval1/2/5seconds, sampling stops around180seconds (an in-flight
bounded status request may finish slightly later). API timeouts can reduce cadence;
use actual recorded elapsed time rather than assuming exact spacing.

## How to read the output

- `running_server`: allowlisted main process, loaded executable SHA256 vs installed
  file SHA256, whether its own bounded fd inventory owns a listening5050socket,
  and identity stability during the read. Permissions/races leave unknown rather
  than requesting sudo. A hash match is NOT a Git/source-revision certification.
- `ram_output`: compact latest service current/peak/swapMB, window maxMB, configured
  transport and observation time. `whole_application_total_MB` remains null until
  attributable shared-service/escaped-worker costs are established—never fabricate it.
- `latest.runtime`: server-reported transport/playing/transfer flags; not a hearing
  test. /api/status request median timing is NOT Play/Next/Previous handler latency.
- `maximum_current_sample`: elapsed time, runtime flags and process buckets at the
  largest sampled service reading. Correlate with user tap times, do not infer cause.
- `process_rss_bucket_maxima`: music-server/yt-dlp/Node/mpv/etc counts and RSS maxima.
  Unknown names become other. Non-atomic/shared-page estimates; NEVER add to cgroup
  totals or each other to claim exact attribution. A missing short-lived helper
  between samples is possible; process enumeration may race and reports incomplete.
- `host_all_workloads`: physical total/available, nonavailable estimate and swap for
  the ENTIRE HOST, including Ubuntu/other apps. Linux MemAvailable is an estimate.
  This is NOT music-only RAM and must never be added to the service total.

Service cgroup includes owned descendants. Shared audio/BlueZ/D-Bus/tunnel services
and escaped workers may be outside it. They are not secretly counted as zero.
`historical_peak_bytes` is retained, not reset or attributed to this sampling window.
Sampling can miss startup spikes. Zero swap/current under50MB does not prove every
phase/listener/recovery peak fits. Do not apply MemoryMax/OOM caps or move helpers
out of accounting to create a pass. The approved compatibility resolver/optionalNode
still costs startup RAM; native acquisition remains separate unfinished work.

Exit1: observed service-current target failure or unexpected/non-listening main
process. Inspect fields; memory failure does not imply the service is dead.
Exit2: partial report, never whole-stack certification. Missing identity reads can
remain unverified even if audio works. Exit3: monitor/private save unavailable; share
that code, not raw tracebacks. A service restart during the window invalidates a
stable-workload conclusion; do not restart merely to reset the peak.

No audible regression fix is justified by the old generic response-rejected hint
alone now that the user hears music. Measure the workload first, preserve playback,
then target a reproduced helper burst/control latency rather than guessing.
