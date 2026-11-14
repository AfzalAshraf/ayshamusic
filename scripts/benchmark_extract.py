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

from source_stage_trace import StageTrace

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "prototypes/rust-extract/target/release/aysha-extract-prototype"
PROBE = 65536


def group_rss_kib(group, proc_root=Path("/proc")):
    """Sample only our dedicated process group. RSS sums double-count shared pages."""
    total, observed = 0, False
    try:
        paths = list(proc_root.iterdir())
    except OSError:
        return None
    for path in paths:
        if not path.name.isdigit():
            continue
        try:
            # comm can contain spaces/parentheses; fields following the final ')' are fixed.
            fields = (path / "stat").read_text().rsplit(")", 1)[1].split()
            if int(fields[2]) != group:
                continue
            for line in (path / "status").read_text().splitlines():
                if line.startswith("VmRSS:"):
                    total += int(line.split()[1])
                    observed = True
        except (OSError, ValueError, IndexError):
            continue  # Processes can exit during sampling.
    return total if observed else None


def safe_format_counts(payload):
    counts = payload.get("format_counts")
    if not isinstance(counts, dict):
        return None
    keys = ("formats_total", "aac_candidates", "empty_urls", "invalid_urls", "rejected_origins", "usable_urls")
    return {key: counts[key] for key in keys
            if type(counts.get(key)) is int and 0 <= counts[key] <= 100000}


def execute(argv, binary_media=False, deadline=75, trace_stages=False):
    start = time.monotonic()
    proc = None
    sampled_peak = None
    result = {}
    trace = StageTrace() if trace_stages else None
    try:
        proc = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE if trace else subprocess.DEVNULL, start_new_session=True, cwd=ROOT)
        data = bytearray()
        first = None
        with selectors.DefaultSelector() as selector:
            selector.register(proc.stdout, selectors.EVENT_READ, "stdout")
            if trace:
                selector.register(proc.stderr, selectors.EVENT_READ, "stderr")
            complete = False
            while time.monotonic() - start < deadline:
                rss = group_rss_kib(proc.pid)
                if rss is not None:
                    sampled_peak = max(sampled_peak or 0, rss)
                events = selector.select(timeout=0.02)
                # Drain stderr concurrently so verbose output cannot deadlock stdout.
                for key, _ in sorted(events, key=lambda event: event[0].data == "stdout"):
                    if key.data == "stderr":
                        chunk = os.read(key.fileobj.fileno(), 4096)
                        if chunk:
                            trace.feed(chunk)
                        else:
                            selector.unregister(key.fileobj)
                        continue
                    chunk = os.read(proc.stdout.fileno(), min(4096, PROBE - len(data)))
                    if not chunk:
                        selector.unregister(proc.stdout)
                        continue
                    if first is None:
                        first = (time.monotonic() - start) * 1000
                    data.extend(chunk)
                    if len(data) >= PROBE:
                        complete = True
                        break
                if complete or not selector.get_map():
                    break
            else:
                result = {"status": "fail", "code": "deadline_exceeded"}
                return result
        total = (time.monotonic() - start) * 1000
        if binary_media:
            if len(data) != PROBE or data[4:8] != b"ftyp":
                result = {"status": "fail", "code": "short_or_non_mp4_probe",
                          "bytes": len(data), "mp4_header": data[4:8] == b"ftyp",
                          "exit_code": proc.poll()}
                return result
            result = {"status": "pass", "first_byte_ms": round(first, 2), "bytes": len(data),
                      "total_ms": round(total, 2), "peak_rss_kib": None}
            return result
        rc = proc.wait(timeout=max(1, deadline - (time.monotonic() - start)))
        payload = json.loads(data)
        if not isinstance(payload, dict):
            raise ValueError("invalid native metrics")
        result = payload
        counts = safe_format_counts(payload)
        candidate = payload.get("candidate")
        if candidate not in ("rusty_ytdl-0.7.4", "rusty_ytdl-pr62-b390352a5db4b57aa630e8dc7b427fb560feccd5"):
            candidate = None
        # Only known numerical fields and fixed codes; never echo arbitrary tool output.
        if rc or result.get("status") != "pass":
            code = result.get("code", "native_failure")
            peak = result.get("peak_rss_kib")
            result = {"status": "fail", "code": code if re.fullmatch(r"[a-z_]{1,60}", str(code)) else "native_failure",
                      "peak_rss_kib": peak if type(peak) is int and peak >= 0 else None,
                      "format_counts": counts, "candidate": candidate}
            return result
        allowed = ("resolve_ms", "first_byte_ms", "total_ms", "bytes", "itag", "peak_rss_kib")
        if result.get("bytes") != PROBE:
            raise ValueError("invalid native probe")
        metrics = {key: result.get(key) for key in allowed if result.get(key) is None or type(result.get(key)) in (int, float)}
        metrics["native_total_ms"] = metrics.get("total_ms")
        metrics["total_ms"] = round(total, 2)
        result = dict(metrics, status="pass", format_counts=counts, candidate=candidate)
        return result
    except (OSError, ValueError, subprocess.TimeoutExpired):
        result = {"status": "fail", "code": "tool_missing_invalid_output_or_timeout"}
        return result
    finally:
        result["sampled_group_peak_rss_kib"] = sampled_peak
        if trace:
            result["stage_trace"] = trace.report()
        if proc is not None:
            # Kill descendants even after their leader exits; no production process is touched.
            try:
                os.killpg(proc.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            proc.wait()
            proc.stdout.close()
            if proc.stderr is not None:
                proc.stderr.close()


def summarize(results):
    summary = {}
    for tool in ("rust", "yt-dlp"):
        rows = [row for row in results if row["tool"] == tool]
        good = [row for row in rows if row["status"] == "pass"]
        summary[tool] = {"successes": len(good), "attempts": len(rows),
                         "median_probe_ms": statistics.median(row["total_ms"] for row in good) if good else None}
    return summary


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--video-id", required=True)
    parser.add_argument("--runs", type=int, default=3)
    parser.add_argument("--baseline-only", action="store_true",
                        help="Diagnostic only: run existing yt-dlp, never build/run a native candidate")
    parser.add_argument("--trace-stages", action="store_true",
                        help="Baseline-only: classify bounded stderr into fixed stage markers; discard raw text")
    args = parser.parse_args(argv)
    if args.trace_stages and not args.baseline_only:
        parser.error("--trace-stages requires --baseline-only; no native retries")
    if not re.fullmatch(r"[A-Za-z0-9_-]{11}", args.video_id) or not 1 <= args.runs <= 5:
        parser.error("Use an 11-character video ID and 1-5 runs")
    if not args.baseline_only and not BINARY.is_file():
        parser.error("Build the isolated Rust extractor release binary first")
    results = []
    for index in range(args.runs):
        tools = ("yt-dlp",) if args.baseline_only else (("rust", "yt-dlp") if index % 2 == 0 else ("yt-dlp", "rust"))
        for tool in tools:
            print(f"Benchmark run {index + 1}/{args.runs}: {tool}; up to 75 seconds", flush=True)
            argv = [str(BINARY), "--video-id", args.video_id] if tool == "rust" else [
                "yt-dlp", "--ignore-config", "--no-cache-dir", "--js-runtimes", "node:/usr/bin/node",
                "--proxy", "", "--no-playlist", "--no-progress", "--socket-timeout", "15", "--retries", "0",
                "-f", "bestaudio[ext=m4a][acodec^=mp4a]", "-o", "-", "--",
                "https://www.youtube.com/watch?v=" + args.video_id]
            if args.trace_stages:
                argv.insert(1, "--verbose")
            options = {"binary_media": tool == "yt-dlp"}
            if args.trace_stages:
                options["trace_stages"] = True
            row = dict(execute(argv, **options), tool=tool, run=index + 1)
            results.append(row)
    report = {"utc": datetime.now(timezone.utc).isoformat(), "prototype_only": True,
              "mode": "baseline_only" if args.baseline_only else "legacy_comparison",
              "native_executed": not args.baseline_only,
              "stage_trace_requested": args.trace_stages,
              "workload": "same public track, audio-only AAC/MP4, 64KiB prefix; no playback",
              "interpretation": ("Baseline pass means unauthenticated AAC acquisition worked for this attempt; investigate missing native source-access capabilities, not a general outage. Baseline failure does not prove a general outage or explain the native bot check. No native performance comparison was run."
                                 if args.baseline_only else "Compare successful equivalent workloads only."),
              "results": results, "summary": summarize(results),
              "limits": "No faster claim if failures; not identical itag verified. Native resolve/first-byte excludes process startup; compare total_ms for startup-inclusive 64KiB; native_total_ms is internal only. peak_rss_kib is native-only HWM; sampled_group_peak_rss_kib samples each tool group incl. JS children, may miss spikes and double-count shared pages. Not aggregate memory/decoding/search/production certification. Upstream caches/IP/network may bias runs. No cookies/JS subprocess in native path; Python benchmark is not a server."}
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
