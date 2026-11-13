#!/usr/bin/env python3
"""Read-only browser playback triage after one listener-initiated Play attempt.

No media/watch request, controls, Bluetooth probe, build, install or restart.
Journal output stays local; only fixed signature IDs/counts leave this process.
Python is development/diagnostic tooling, never a production playback helper.
"""
import json
from pathlib import Path

try:
    from scripts import check_health as health
except ModuleNotFoundError:
    import check_health as health

# Hints, not causal proof: a bounded service log window may include other clients.
SIGNATURES = {
    "UPSTREAM_ACCESS_RESTRICTED": ("sign in to confirm", "login required", "private video", "video unavailable", "not available in your country"),
    "UPSTREAM_HTTP_403": ("http error 403", "403 forbidden"),
    "DOWNLOADER_FORMAT_UNAVAILABLE": ("requested format is not available",),
    "JS_RUNTIME_MISSING": ("no supported javascript runtime", "javascript runtime could not be found"),
    "DOWNLOADER_LAUNCH_FAILED": ("unable to start yt-dlp",),
    "STREAM_NO_AUDIO_HEADER": ("yt-dlp returned no usable audio header",),
}


def journal_counts(text):
    lines = text.lower().splitlines()
    return {code: sum(any(pattern in line for pattern in patterns) for line in lines)
            for code, patterns in SIGNATURES.items()}


def collect():
    rows = []
    rc, raw = health.command(["systemctl", "--no-pager", "show", health.SERVICE,
                              "-p", "ActiveState", "-p", "SubState"], timeout=10)
    props = health.properties(raw) if rc == 0 else {}
    running = props.get("ActiveState") == "active" and props.get("SubState") == "running"
    rows.append({"id": "SERVICE_RUNNING", "status": "pass" if running else "fail"})
    try:
        served = health.http_get("/")
        expected = (health.ROOT / "src/index.html").read_bytes()
        rows.append({"id": "BROWSER_ROUTING_UI_INSTALLED", "status": "pass" if served == expected else "fail"})
    except (OSError, ValueError):
        rows.append({"id": "BROWSER_ROUTING_UI_INSTALLED", "status": "unverified"})
    try:
        status = json.loads(health.http_get("/api/status"))
        if not isinstance(status, dict) or type(status.get("is_playing")) is not bool:
            raise ValueError("invalid status")
        rows.append({"id": "PLAYER_STATUS", "status": "observed",
                     "playing_flag": status["is_playing"],
                     "selection_present": isinstance(status.get("current_track"), dict),
                     "audibility_proven": False})
    except (OSError, ValueError):
        rows.append({"id": "PLAYER_STATUS", "status": "fail"})
    rc, raw = health.command(["journalctl", "--no-pager", "--quiet", "-u", health.SERVICE,
                              "--since", "5 minutes ago", "-n", "100", "-o", "cat"], timeout=10)
    # A permission warning may accompany rc=0. Do not mistake that for log access.
    unavailable = rc != 0 or not raw.strip() or any(phrase in raw.lower() for phrase in (
        "permission denied", "insufficient permissions", "not seeing messages", "no journal files"))
    rows.append({"id": "RECENT_SERVICE_ERROR_HINTS", "status": "unverified" if unavailable else "observed",
                 "signature_counts": {} if unavailable else journal_counts(raw),
                 "scope": "At most 100 recent service lines/1MiB, last 5 minutes; may include other clients; absence is not proof of successful streaming."})
    if any(row["status"] == "fail" for row in rows):
        code = "fix_reported_service_or_deployment_layer_first"
    elif not unavailable and any(journal_counts(raw).values()):
        code = "review_reported_stream_error_hints_no_automatic_retry"
    else:
        code = "need_phone_error_and_listening_result_not_native_acceptance"
    return {"schema": 1, "candidate": "aysha-browser-playback-triage-v1", "code": code,
            "checks": rows, "starts_audio": False, "upstream_requested": False,
            "service_changed": False, "raw_logs_included": False,
            "audibility_verified": False, "memory_budget_accepted": False,
            "limits": "Read-only installed-service triage, not a native probe. UI equality is not complete backend binary identity. Cannot inspect the phone's media element or hear output. No all-Rust/30MB acceptance."}


def main():
    report = collect()
    print(json.dumps(report, indent=2))
    return 1 if any(row["status"] == "fail" for row in report["checks"]) else 2


if __name__ == "__main__":
    raise SystemExit(main())
