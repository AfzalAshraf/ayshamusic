---
name: qa-debugging
description: Reproduce Aysha failures, collect bounded local diagnostic reports with stable check IDs and exit codes, distinguish evidence from assumptions, and verify minimal fixes without changing the host automatically.
---

# QA & Debugging — evidence before fixes

Follow [the main upgrade contract](../main-upgrade/SKILL.md) and the latest [QA ledger](../../docs/LOCAL_QA_LEDGER.md). Use one active department. This skill does not authorize service, firewall, Bluetooth, authentication or architecture changes.

## Collect once, safely

Run `python3 scripts/check_health.py` as the service's normal user on the intended Linux host. It checks the fixed loopback service on port 5050, service/restart/memory properties, checkout revision, built/installed file hashes, served UI identity, runtime versions and shell-visible Bluetooth sinks. It never starts audio or mutates playback state. It saves a private timestamped report under ignored `diagnostics/` and prints it without a pager. No sudo, raw journal/environment/process arguments, song metadata, Bluetooth addresses or automatic upload. Review even allowlisted reports before sharing; source revision, counts and machine state are still operational information.

Use `--code-checks` only when source regression testing is requested: Python, Node and Rust suites run with bounded subprocess timeouts. These tests write caches/build artifacts, may fetch Cargo dependencies and consume CPU/memory; do not measure production memory while they run. No release install or service restart occurs. Full browser tests and audible media decoding are not included. Use `--expect-bluetooth` only when the accepted output is the host speaker; missing Bluetooth is not inherently a browser-playback defect.

## Interpret the packet

- `RETURN_CODE=0`: checked probes passed, not whole-app certification.
- `1`: at least one FAIL, including a recorded memory peak above both candidate 100 MB thresholds.
- `2`: no FAIL, but warnings/incomplete probes. Missing optional tools, unknown memory, disconnected optional Bluetooth and inability to prove the loaded revision must not be reported as passed.
- `3`: reporter could not finish/save; terminal reports that fact without arbitrary exception text.
- Invalid CLI usage uses argparse's conventional exit 2; consult the usage message, not a nonexistent report.

Stable IDs identify the failing layer: SERVICE/RESTARTS, MEMORYCURRENT/MEMORYPEAK, BINARY/RUNNING_REVISION/SERVED_UI, HTTP_UI/HTTP_STATUS/HTTP_QUEUE, NODE/YTDLP, BLUETOOTH_SINK, PYTHON_TESTS/FRONTEND_TESTS/RUST_TESTS. Test subprocess exit 124 means timeout, 126 launch error, 127 missing executable. Raw subprocess output is intentionally not copied; if a test suite fails, request a targeted rerun of that suite and review/redact its output, not an unrestricted log dump.

## Diagnose, do not guess

1. Record exact failing ID, return code, revision, environment and reproduction. Compare host evidence with sandbox/fixture evidence; never substitute one for the other.
2. A `systemctl show` memory peak is service-cgroup history, not Rust RSS or a known song's peak. Above 100 MiB exceeds either interpretation of the user's 100 MB target. Between decimal 100 MB and 100 MiB is a warning until the byte threshold is agreed. Lower current usage does not erase a high peak. Do not reset counters or impose an OOM-producing limit to manufacture a pass. Helpers outside that cgroup remain outside that measurement.
3. Matching built/installed files does not establish that the running process loaded them; source pull alone is not deployment. Exact served-HTML comparison helps detect a stale embedded UI, not the identity of all backend code.
4. A Bluetooth sink visible to an SSH shell is not proof of service-context access or audible output. Disconnection does not imply failed Rust compilation. Do not erase pairings or automatically connect/restart to 'fix' diagnosis.
5. A green HTTP fixture/CI is not real decoding, upstream extraction, iPhone background playback, natural-end autoplay, independent simultaneous sessions, public authentication, reboot recovery or aggregate memory certification.
6. Propose the smallest reproducible fix, obtain architecture approval if boundaries change, add regression coverage, rerun affected checks, then record remaining NOT RUN gates. Never promise the checker detects all possible code defects.

See [local service/report instructions](../../docs/LOCAL_SERVICE.md). End with the build manifest and error/risk ledger. Ratings rise only on acceptance evidence, not installation of this checker.

## Memory-failure follow-up

Use `python3 scripts/check_health.py --memory-samples 30` during ordinary playback
and one listener-controlled song change. Never combine with code builds; the CLI
rejects `--code-checks` in this mode. Obtain device count and approximate song-change
time alongside the packet. Inspect anon/file/kernel/swap and cgroup process buckets,
without summing RSS with cgroup totals or discarding cache. Nested process enumeration
is bounded and flags incomplete reads. Historical peak is retained, external helpers
remain unaccounted, and sampling is not proof of a transient maximum. Track this
work in Linear AFZ-5; close only after cause, approved fix and real workload acceptance,
not merely adding the profiler. See docs/READINESS_ROADMAP.md for tracking boundaries.
