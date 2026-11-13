#!/usr/bin/env python3
"""Read-only 10–15 MB service-cgroup audit during user-controlled playback.

No builds, player controls, extraction, upstream requests, sudo, installs or resets.
Python is diagnostic-only, never a production playback/extraction helper.
"""
import argparse
import json
from pathlib import Path
import time

try:
    from scripts import check_health as health
except ModuleNotFoundError:
    import check_health as health

DECIMAL_UPPER = 15_000_000
BINARY_UPPER = 15 * 1024 * 1024


def service_state():
    rc, text = health.command([
        "systemctl", "--no-pager", "show", health.SERVICE,
        "-p", "ActiveState", "-p", "SubState", "-p", "ControlGroup",
        "-p", "MainPID", "-p", "NRestarts",
    ], timeout=10)
    if rc:
        raise ValueError("service probe unavailable")
    props = health.properties(text)
    if props.get("ActiveState") != "active" or props.get("SubState") != "running":
        raise ValueError("service not running")
    if not props.get("MainPID", "").isdigit() or int(props["MainPID"]) == 0:
        raise ValueError("service identity unavailable")
    return props


def group_path(relative, root=Path("/sys/fs/cgroup")):
    base = root.resolve()
    group = (base / relative.lstrip("/")).resolve()
    if not relative.startswith("/") or group == base or base not in group.parents:
        raise ValueError("invalid cgroup")
    return group


def playback_flag():
    # Fixed loopback /api/status only; never returns track metadata or controls audio.
    try:
        value = json.loads(health.http_get("/api/status"))
        flag = value.get("is_playing") if isinstance(value, dict) else None
        return flag if type(flag) is bool else None
    except (OSError, ValueError):
        return None


def evaluate(samples, flags, stable, incomplete):
    report = {
        "status": "incomplete", "code": "budget_not_accepted",
        "measurement_scope": "aysha_music_service_cgroup_and_descendants_only",
        "candidate_upper_bytes": {"15_MB": DECIMAL_UPPER, "15_MiB": BINARY_UPPER},
        "samples_completed": len(samples), "service_identity_stable": stable,
        "process_inventory_incomplete": incomplete,
        "playing_flag_start": flags[0], "playing_flag_end": flags[1],
        "audible_output_verified": False, "download_or_decode_verified": False,
        "native_acquisition_verified": False, "whole_stack_accepted": False,
        "limits": "Service-cgroup lower bound only; external audio/tunnel/shared services and escaped workers are not counted. Playing flags do not prove continuous playback or audibility. Sampling can miss peaks; historical peak is not reset or attributed to this song. No RSS+cgroup summation. No installed/running revision certification.",
    }
    if not samples:
        report["code"] = "no_readable_memory_samples"
        return report
    current = max(s["memory.current"] for s in samples)
    peak = max(s["memory.peak"] for s in samples)
    swap = max(s["memory.swap.current"] for s in samples)
    report.update({
        "sampled_max_current_bytes": current, "historical_peak_bytes": peak,
        "sampled_max_swap_bytes": swap,
        "sampled_max_current_MB": round(current / 1_000_000, 3),
        "sampled_max_current_MiB": round(current / 1048576, 3),
        "sampled_current_exceeds_15_MB": current > DECIMAL_UPPER,
        "sampled_current_exceeds_15_MiB": current > BINARY_UPPER,
        "historical_peak_exceeds_15_MB": peak > DECIMAL_UPPER,
        "historical_peak_exceeds_15_MiB": peak > BINARY_UPPER,
    })
    if current > BINARY_UPPER:
        report.update(status="fail", code="service_cgroup_alone_exceeds_both_15_MB_and_15_MiB")
    elif current > DECIMAL_UPPER:
        report.update(status="fail", code="service_cgroup_exceeds_decimal_15_MB_units_unresolved")
    elif swap:
        report["code"] = "swap_observed_budget_not_accepted"
    elif not stable:
        report["code"] = "service_changed_during_window"
    elif not all(flag is True for flag in flags):
        report["code"] = "playback_not_reported_at_both_endpoints"
    elif peak > DECIMAL_UPPER:
        report["code"] = "sampled_current_under_upper_limit_but_historical_peak_exceeds_it"
    else:
        report["code"] = "sampled_service_under_upper_limit_whole_stack_still_unproved"
    return report


def collect(count, root=Path("/sys/fs/cgroup")):
    before = service_state()
    group = group_path(before.get("ControlGroup", ""), root)
    flags = [playback_flag(), None]
    samples = []
    buckets_max = {}
    incomplete = False
    for n in range(count):
        try:
            values, buckets, partial = health.cgroup_snapshot(group)
            samples.append(values)
            incomplete |= partial
            for label, (process_count, rss) in buckets.items():
                old = buckets_max.get(label, {"max_count": 0, "max_rss_bytes": 0})
                # Keep the diagnostic's process-label allowlist even if the shared
                # helper changes later; never expose arbitrary comm/args/PIDs.
                key = label if label in {"music-server", "yt-dlp", "python3", "node", "mpv", "ffmpeg"} else "other"
                old = buckets_max.get(key, old)
                buckets_max[key] = {"max_count": max(old["max_count"], process_count),
                                    "max_rss_bytes": max(old["max_rss_bytes"], rss)}
        except (OSError, ValueError):
            incomplete = True
        if n + 1 < count:
            time.sleep(2)
    flags[1] = playback_flag()
    try:
        after = service_state()
        stable = all(before.get(k) == after.get(k) for k in ("ControlGroup", "MainPID", "NRestarts"))
    except (OSError, ValueError):
        stable = False
    report = evaluate(samples, flags, stable, incomplete)
    report.update(samples_requested=count, spacing_seconds=2,
                  process_rss_buckets=buckets_max,
                  bucket_limit="Non-atomic per-label maxima; may double-count shared pages. Never add these to the cgroup total.")
    return report


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--samples", type=int, default=20, help="1–30 samples, 2 seconds apart; play/change songs yourself")
    args = parser.parse_args(argv)
    if not 1 <= args.samples <= 30:
        parser.error("samples must be 1–30")
    try:
        report = collect(args.samples)
    except (OSError, ValueError):
        report = {"status": "incomplete", "code": "service_cgroup_unavailable",
                  "whole_stack_accepted": False, "audible_output_verified": False,
                  "native_acquisition_verified": False, "download_or_decode_verified": False}
    report.update(schema=1, candidate="aysha-service-playback-budget-v1",
                  starts_playback=False, upstream_requested=False, builds_run=False)
    print(json.dumps(report, indent=2))
    # Never exit 0: current cgroup-only readings cannot certify the whole stack.
    return 1 if report["status"] == "fail" else 2


if __name__ == "__main__":
    raise SystemExit(main())
