#!/usr/bin/env python3
"""Bounded, local-only diagnostics; never starts/stops the Rust service or uploads data."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
SERVICE = "aysha-music"
INSTALLED = Path("/home/afine/music-server/target/release/music-server")


def command(argv, timeout=15):
    """No shell, pager, sudo or unbounded captured output; kill timed-out children too."""
    env = dict(os.environ, SYSTEMD_PAGER="cat", SYSTEMD_COLORS="0", PAGER="cat", NO_COLOR="1")
    try:
        with tempfile.TemporaryFile() as output:
            proc = subprocess.Popen(argv, cwd=ROOT, env=env, stdin=subprocess.DEVNULL,
                                    stdout=output, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                proc.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                os.killpg(proc.pid, signal.SIGKILL)
                proc.wait()
                return 124, ""
            output.seek(0)
            return proc.returncode, output.read(1024 * 1024).decode("utf-8", errors="replace")
    except FileNotFoundError:
        return 127, ""
    except OSError:
        return 126, ""


def properties(text):
    return dict(line.split("=", 1) for line in text.splitlines() if "=" in line)


def memory_result(value):
    """Threshold choice is unresolved. Do not silently treat MB as MiB."""
    if not value.isdigit():
        return "WARN", "unavailable; memory accounting not verified"
    size = int(value)
    detail = f"{size} bytes ({size / 1000000:.2f} MB / {size / 1048576:.2f} MiB)"
    if size > 100 * 1048576:
        return "FAIL", detail + "; exceeds both 100 MB and 100 MiB"
    if size > 100000000:
        return "WARN", detail + "; exceeds decimal 100 MB; byte threshold needs agreement"
    return "PASS", detail + "; this reading only, not concurrent-load certification"


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def http_get(path):
    # Fixed loopback origin only; ignore proxy environment and reject redirects.
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
    with opener.open("http://127.0.0.1:5050" + path, timeout=5) as response:
        if response.status != 200:
            raise ValueError("unexpected HTTP status")
        body = response.read(1024 * 1024 + 1)
        if len(body) > 1024 * 1024:
            raise ValueError("oversized response")
        return body


class Report:
    def __init__(self):
        self.rows = []

    def add(self, level, code, detail):
        self.rows.append((level, code, detail))

    def exit_code(self):
        if any(level == "FAIL" for level, _, _ in self.rows):
            return 1
        return 2 if any(level == "WARN" for level, _, _ in self.rows) else 0

    def render(self):
        lines = ["=== BEGIN AYSHA DEBUG REPORT v1 ===", "UTC: " + datetime.now(timezone.utc).isoformat(),
                 "Local checks only. No audio started, service changes, raw logs or upload."]
        lines.extend(f"[{level}] {code}: {detail}" for level, code, detail in self.rows)
        lines += [f"RETURN_CODE={self.exit_code()}",
                  "0=checked probes passed; 1=failure found; 2=warnings/incomplete checks; 3=reporter error",
                  "Not certified: audible decoding, autoplay, iPhone/background behavior, reboot,",
                  "multi-device isolation, security/tunnel and aggregate concurrent-load memory.",
                  "=== END AYSHA DEBUG REPORT ==="]
        return "\n".join(lines) + "\n"


def collect(code_checks=False, expect_bluetooth=False):
    report = Report()
    rc, out = command(["git", "rev-parse", "HEAD"])
    revision = out.strip()
    report.add("PASS" if rc == 0 and re.fullmatch(r"[0-9a-f]{40}", revision) else "WARN",
               "SOURCE", revision if re.fullmatch(r"[0-9a-f]{40}", revision) else "revision unavailable")

    rc, out = command(["systemctl", "--no-pager", "show", SERVICE,
                       "-p", "ActiveState", "-p", "SubState", "-p", "NRestarts",
                       "-p", "MemoryCurrent", "-p", "MemoryPeak"])
    props = properties(out) if rc == 0 else {}
    running = props.get("ActiveState") == "active" and props.get("SubState") == "running"
    report.add("PASS" if running else "FAIL", "SERVICE",
               "active/running" if running else f"not confirmed running (probe rc={rc}); inspect systemctl --no-pager status aysha-music")
    restarts = props.get("NRestarts", "")
    report.add("PASS" if restarts == "0" else "WARN", "RESTARTS",
               f"{restarts} automatic restarts in current accounting interval" if restarts.isdigit() else "unavailable")
    for key in ("MemoryCurrent", "MemoryPeak"):
        level, detail = memory_result(props.get(key, ""))
        report.add(level, key.upper(), detail)

    # Compare the built and installed on-disk binaries, not Git source to running memory.
    try:
        def digest(path):
            with path.open("rb") as source:
                return hashlib.file_digest(source, "sha256").hexdigest() if hasattr(hashlib, "file_digest") else _digest(source)
        same = digest(ROOT / "target/release/music-server") == digest(INSTALLED)
        report.add("PASS" if same else "WARN", "BINARY", "built/installed files match" if same else "built/installed files differ; no install performed")
    except OSError:
        report.add("WARN", "BINARY", "built/installed comparison unavailable")
    report.add("WARN", "RUNNING_REVISION", "on-disk hash comparison cannot prove which binary is loaded; source pull alone is not installation")

    for path, label in (("/", "HTTP_UI"), ("/api/status", "HTTP_STATUS"), ("/api/queue", "HTTP_QUEUE")):
        try:
            body = http_get(path)
            if label == "HTTP_UI":
                if b"<html" not in body.lower() or b"<audio" not in body.lower():
                    raise ValueError("invalid page")
                report.add("PASS", label, "HTML/audio element present")
                same = body == (ROOT / "src/index.html").read_bytes()
                report.add("PASS" if same else "WARN", "SERVED_UI", "matches checkout exactly" if same else "differs from checkout; may be stale or another build")
            else:
                value = json.loads(body)
                if label == "HTTP_STATUS":
                    if (not isinstance(value, dict) or type(value.get("is_playing")) is not bool
                            or any(type(value.get(k)) is not int or value[k] < 0 for k in ("queue_len", "liked_count"))):
                        raise ValueError("invalid status")
                    detail = "schema valid; playing=" + str(value["is_playing"]).lower()
                else:
                    if not isinstance(value, list) or any(not isinstance(t, dict) or any(not isinstance(t.get(k), str) for k in ("id", "title", "artist", "thumbnail", "url")) for t in value):
                        raise ValueError("invalid queue")
                    detail = f"schema valid; {len(value)} queued tracks"
                report.add("PASS", label, detail)
        except (OSError, ValueError, urllib.error.URLError):
            report.add("FAIL", label, "loopback probe failed (timeout, HTTP error, unreadable source or invalid response); no response contents included")

    for tool, argv, pattern in (
        ("NODE", ["/usr/bin/node", "--version"], r"v\d+\.\d+\.\d+"),
        ("YTDLP", ["/home/afine/.local/bin/yt-dlp", "--version"], r"\d{4}\.\d{2}\.\d{2}"),
    ):
        rc, out = command(argv)
        version = out.strip()
        valid = rc == 0 and re.fullmatch(pattern, version)
        supported = valid and (tool != "NODE" or int(version[1:].split(".")[0]) >= 22)
        report.add("PASS" if supported else "WARN", tool, version if valid else f"version unavailable (probe rc={rc})")
    rc, out = command(["pactl", "list", "sinks", "short"])
    connected = rc == 0 and any("bluez_output." in line for line in out.splitlines())
    report.add("PASS" if connected else "FAIL" if expect_bluetooth else "WARN", "BLUETOOTH_SINK",
               "Bluetooth sink visible to shell user; audibility/service access not proven" if connected else "no Bluetooth output sink visible; not required for browser mode")

    if code_checks:
        for label, argv in (
            ("PYTHON_TESTS", [sys.executable, "-m", "unittest", "discover", "-s", "tests", "-p", "test_*.py"]),
            ("FRONTEND_TESTS", ["node", "--test", "tests/frontend.test.cjs"]),
            ("RUST_TESTS", ["cargo", "test"]),
        ):
            rc, _ = command(argv, timeout=300)
            report.add("PASS" if rc == 0 else "WARN" if rc in (124, 126, 127) else "FAIL", label,
                       f"exit={rc}; 124=timeout, 126=launch error, 127=missing tool; raw test output omitted")
    else:
        report.add("WARN", "CODE_TESTS", "not run; use --code-checks (writes build caches, may fetch Cargo dependencies)")
    return report


def _digest(source):
    digest = hashlib.sha256()
    for chunk in iter(lambda: source.read(1024 * 1024), b""):
        digest.update(chunk)
    return digest.hexdigest()


def save_report(text):
    folder = ROOT / "diagnostics"
    folder.mkdir(mode=0o700, exist_ok=True)
    fd, name = tempfile.mkstemp(prefix=datetime.now(timezone.utc).strftime("health-%Y%m%dT%H%M%SZ-"), suffix=".txt", dir=folder)
    with os.fdopen(fd, "w") as target:
        target.write(text)
    return Path(name)


def cgroup_snapshot(group, proc_root=Path("/proc")):
    """Read cgroup-v2 totals plus allowlisted process RSS; no args/URLs/names leak."""
    values = {}
    for name in ("memory.current", "memory.peak", "memory.swap.current"):
        raw = (group / name).read_text().strip()
        if not raw.isdigit():
            raise ValueError("invalid memory counter")
        values[name] = int(raw)
    stats = dict(line.split() for line in (group / "memory.stat").read_text().splitlines())
    for key in ("anon", "file", "kernel"):
        raw = stats.get(key, "")
        values[key] = int(raw) if raw.isdigit() else None
    # Parent memory totals include descendants. Bound enumeration and disclose gaps.
    pids = set()
    incomplete = False
    for index, (folder, dirs, _) in enumerate(os.walk(group)):
        dirs.sort()
        if index >= 64:
            incomplete = True
            break
        try:
            raw = (Path(folder) / "cgroup.procs").read_text().split()
            pids.update(int(pid) for pid in raw if pid.isdigit())
        except OSError:
            incomplete = True
        if len(pids) > 256:
            incomplete = True
            break
    buckets = {}
    for pid in sorted(pids)[:256]:
        try:
            comm = (proc_root / str(pid) / "comm").read_text().strip()
            label = comm if comm in {"music-server", "yt-dlp", "python3", "node", "mpv", "ffmpeg"} else "other"
            status = (proc_root / str(pid) / "status").read_text()
            match = re.search(r"^VmRSS:\s+(\d+) kB$", status, re.MULTILINE)
            if not match:
                incomplete = True
                continue
            count, rss = buckets.get(label, (0, 0))
            buckets[label] = (count + 1, rss + int(match[1]) * 1024)
        except OSError:
            incomplete = True  # Processes may finish between these reads.
    return values, buckets, incomplete or not buckets


def profile_memory(report, samples, cgroup_root=Path("/sys/fs/cgroup")):
    rc, out = command(["systemctl", "--no-pager", "show", SERVICE, "-p", "ControlGroup", "--value"])
    relative = out.strip()
    base = cgroup_root.resolve()
    group = (base / relative.lstrip("/")).resolve()
    if rc or not relative.startswith("/") or group == base or base not in group.parents:
        report.add("WARN", "MEMORY_PROFILE", "service cgroup unavailable/invalid; no sampling performed")
        return
    report.add("WARN", "MEMORY_SCOPE", "service cgroup only; outside helpers excluded. RSS buckets can double-count shared pages; do not sum with cgroup totals. No counters reset.")
    report.add("PASS", "MEMORY_WINDOW", "start_utc=" + datetime.now(timezone.utc).isoformat() + f"; requested_samples={samples}; spacing_seconds=2")
    for index in range(samples):
        try:
            values, buckets, incomplete = cgroup_snapshot(group)
            level, detail = memory_result(str(values["memory.current"]))
            counters = "; ".join(f"{key}={value if value is not None else 'unknown'}" for key, value in values.items())
            report.add(level, f"MEM_SAMPLE_{index + 1:02d}", counters + "; " + detail)
            peak_level, peak_detail = memory_result(str(values["memory.peak"]))
            report.add(peak_level, f"MEM_PEAK_{index + 1:02d}", peak_detail)
            processes = "; ".join(f"{key}:count={count},rss_bytes={rss}" for key, (count, rss) in sorted(buckets.items()))
            report.add("WARN" if incomplete else "PASS", f"MEM_PROCS_{index + 1:02d}",
                       (processes or "no readable processes") + ("; incomplete/racing process reads" if incomplete else "; non-atomic RSS observations"))
        except (OSError, ValueError):
            report.add("WARN", "MEMORY_PROFILE", "cgroup counters unavailable/invalid; sample skipped")
        if index + 1 < samples:
            time.sleep(2)
    report.add("WARN", "MEMORY_PROFILE_LIMIT", "sampled observations may miss transient peaks; historical cgroup peak retained; cause not automatically established")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--code-checks", action="store_true", help="also run Python/Node/Rust tests, up to 5 minutes each; may fetch dependencies")
    parser.add_argument("--expect-bluetooth", action="store_true", help="treat missing Bluetooth sink as failure")
    parser.add_argument("--memory-samples", type=int, default=0, metavar="N", help="1-30 service-cgroup samples spaced 2 seconds apart; no builds")
    args = parser.parse_args()
    if not 0 <= args.memory_samples <= 30 or (args.memory_samples and args.code_checks):
        parser.error("memory samples must be 0-30 and cannot be combined with --code-checks")
    try:
        report = collect(args.code_checks, args.expect_bluetooth)
        if args.memory_samples:
            print("Sampling memory; keep normal playback running. No test builds will run.", flush=True)
            profile_memory(report, args.memory_samples)
        text = report.render()
        saved = save_report(text)
    except Exception:
        # Never dump arbitrary exception text/response data, environment or a traceback.
        print("[FAIL] REPORTER: could not finish/save diagnostics. RETURN_CODE=3", file=sys.stderr)
        return 3
    print(text, end="")
    print("Saved locally: diagnostics/" + saved.name)
    print("Review before sharing; copy the report between BEGIN/END markers above. No automatic upload.")
    return report.exit_code()


if __name__ == "__main__":
    sys.exit(main())
