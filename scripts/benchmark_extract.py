#!/usr/bin/env python3
"""Opt-in network benchmark. No playback, install or production integration."""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import selectors
import signal
import statistics
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "prototypes/rust-extract/target/release/aysha-extract-prototype"
PROBE = 65536


def execute(argv, binary_media=False, deadline=75):
    start = time.monotonic()
    proc = None
    try:
        proc = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                stderr=subprocess.DEVNULL, start_new_session=True, cwd=ROOT)
        data = bytearray()
        first = None
        with selectors.DefaultSelector() as selector:
            selector.register(proc.stdout, selectors.EVENT_READ)
            while time.monotonic() - start < deadline:
                if not selector.select(timeout=0.2):
                    continue
                chunk = os.read(proc.stdout.fileno(), min(4096, PROBE - len(data)))
                if not chunk:
                    break
                if first is None:
                    first = (time.monotonic() - start) * 1000
                data.extend(chunk)
                if len(data) >= PROBE:
                    break
            else:
                return {"status": "fail", "code": "deadline_exceeded"}
        total = (time.monotonic() - start) * 1000
        if binary_media:
            if len(data) != PROBE or data[4:8] != b"ftyp":
                return {"status": "fail", "code": "short_or_non_mp4_probe"}
            return {"status": "pass", "first_byte_ms": round(first, 2), "bytes": len(data),
                    "total_ms": round(total, 2), "peak_rss_kib": None}
        rc = proc.wait(timeout=max(1, deadline - (time.monotonic() - start)))
        result = json.loads(data)
        # Only known numerical fields and fixed codes; never echo arbitrary tool output.
        if rc or result.get("status") != "pass":
            code = result.get("code", "native_failure")
            return {"status": "fail", "code": code if re.fullmatch(r"[a-z_]{1,60}", str(code)) else "native_failure"}
        allowed = ("resolve_ms", "first_byte_ms", "total_ms", "bytes", "itag", "peak_rss_kib")
        if result.get("bytes") != PROBE:
            raise ValueError("invalid native probe")
        metrics = {key: result.get(key) for key in allowed if result.get(key) is None or type(result.get(key)) in (int, float)}
        metrics["native_total_ms"] = metrics.get("total_ms")
        metrics["total_ms"] = round(total, 2)
        return dict(metrics, status="pass")
    except (OSError, ValueError, subprocess.TimeoutExpired):
        return {"status": "fail", "code": "tool_missing_invalid_output_or_timeout"}
    finally:
        if proc is not None:
            # Kill descendants even after their leader exits; no production process is touched.
            try:
                os.killpg(proc.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            proc.wait()
            proc.stdout.close()


def summarize(results):
    summary = {}
    for tool in ("rust", "yt-dlp"):
        rows = [row for row in results if row["tool"] == tool]
        good = [row for row in rows if row["status"] == "pass"]
        summary[tool] = {"successes": len(good), "attempts": len(rows),
                         "median_probe_ms": statistics.median(row["total_ms"] for row in good) if good else None}
    return summary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--video-id", required=True)
    parser.add_argument("--runs", type=int, default=3)
    args = parser.parse_args()
    if not re.fullmatch(r"[A-Za-z0-9_-]{11}", args.video_id) or not 1 <= args.runs <= 5:
        parser.error("Use an 11-character video ID and 1-5 runs")
    if not BINARY.is_file():
        parser.error("Build the isolated Rust extractor release binary first")
    results = []
    for index in range(args.runs):
        for tool in (("rust", "yt-dlp") if index % 2 == 0 else ("yt-dlp", "rust")):
            print(f"Benchmark run {index + 1}/{args.runs}: {tool}; up to 75 seconds", flush=True)
            argv = [str(BINARY), "--video-id", args.video_id] if tool == "rust" else [
                "yt-dlp", "--ignore-config", "--no-cache-dir", "--js-runtimes", "node:/usr/bin/node",
                "--proxy", "", "--no-playlist", "--no-progress", "--socket-timeout", "15", "--retries", "0",
                "-f", "bestaudio[ext=m4a][acodec^=mp4a]", "-o", "-", "--",
                "https://www.youtube.com/watch?v=" + args.video_id]
            row = dict(execute(argv, binary_media=tool == "yt-dlp"), tool=tool, run=index + 1)
            results.append(row)
    report = {"utc": datetime.now(timezone.utc).isoformat(), "prototype_only": True,
              "workload": "same public track, audio-only AAC/MP4, 64KiB prefix; no playback",
              "results": results, "summary": summarize(results),
              "limits": "No faster claim if failures; not identical itag verified. Native resolve/first-byte excludes process startup; compare total_ms for startup-inclusive 64KiB; native_total_ms is internal only. Native RSS is one process only, yt-dlp RSS not measured. Not aggregate memory/decoding/search/production certification. Upstream caches/IP/network may bias runs. No cookies/JS subprocess in native path; Python benchmark is not a server."}
    text = json.dumps(report, indent=2)
    directory = ROOT / "diagnostics"
    directory.mkdir(mode=0o700, exist_ok=True)
    fd, filename = tempfile.mkstemp(prefix="extract-benchmark-", suffix=".json", dir=directory)
    with os.fdopen(fd, "w") as output:
        output.write(text + "\n")
    print("=== BEGIN EXTRACTION BENCHMARK ===\n" + text + "\n=== END EXTRACTION BENCHMARK ===")
    print("Saved locally: diagnostics/" + Path(filename).name + "; review before sharing, no upload.")
    return 1 if any(row["status"] != "pass" for row in results) else 0


if __name__ == "__main__":
    raise SystemExit(main())
