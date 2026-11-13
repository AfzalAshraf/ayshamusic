#!/usr/bin/env python3
"""Read-only browser playback triage after one listener-initiated Play attempt.

No media/watch request, controls, Bluetooth probe, build, install or restart.
Journal output stays local; only fixed signature IDs/counts leave this process.
Python is development/diagnostic tooling, never a production playback helper.
"""
import argparse
import json

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
    "RUST_RESOLVER_BUSY": ("rust-resolved stream: metadata_busy",),
    "RUST_RESOLVER_FAILED": tuple("rust-resolved stream: " + code for code in (
        "metadata_spawn_failed", "metadata_timeout", "metadata_output_limit", "metadata_exit_failed",
        "metadata_stdout_missing", "metadata_read_failed", "metadata_wait_failed")),
    "RUST_RESOLVER_RECORD_REJECTED": tuple("rust-resolved stream: " + code for code in (
        "resolver_record_limit", "resolver_record_invalid", "resolver_identity_mismatch", "resolver_live_unsupported",
        "resolver_protocol_unsupported", "resolver_codec_unsupported", "resolver_origin_rejected", "resolver_headers_rejected")),
    "RUST_CDN_OR_HEADER_FAILED": tuple("rust-resolved stream: " + code for code in (
        "native_stream_request_failed", "native_stream_response_rejected", "native_stream_container_rejected",
        "native_stream_short_header", "native_stream_read_failed", "native_stream_chunk_limit",
        "native_stream_response_failed", "native_client_unavailable")),
    "RUST_STARTUP_TIMEOUT": ("rust-resolved stream: native_startup_timeout",),
}


def journal_counts(text):
    lines = text.lower().splitlines()
    return {code: sum(any(pattern in line for pattern in patterns) for line in lines)
            for code, patterns in SIGNATURES.items()}


def collect(expected_transport=None):
    rows = []
    transport = "unverified"
    active_stream = None
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
        mode = status.get("stream_transport")
        if isinstance(mode, str) and mode in {"legacy", "rust-resolved", "invalid"}:
            transport = mode
        flag = status.get("rust_stream_active")
        active_stream = flag if type(flag) is bool else None
        rows.append({"id": "PLAYER_STATUS", "status": "observed",
                     "playing_flag": status["is_playing"],
                     "selection_present": isinstance(status.get("current_track"), dict),
                     "audibility_proven": False})
    except (OSError, ValueError):
        rows.append({"id": "PLAYER_STATUS", "status": "fail"})
    mode_status = ("fail" if transport == "invalid" or (expected_transport is not None
                   and transport in {"legacy", "rust-resolved"} and transport != expected_transport)
                   else "unverified" if transport == "unverified" else "observed")
    rows.append({"id": "STREAM_TRANSPORT", "status": mode_status, "configured": transport,
                 "expected": expected_transport, "rust_stream_active": active_stream,
                 "limits": "API-reported mode only, not exact binary identity, audible output or a memory measurement."})
    rc, raw = health.command(["journalctl", "--no-pager", "--quiet", "-u", health.SERVICE,
                              "--since", "5 minutes ago", "-n", "100", "-o", "cat"], timeout=10)
    # A permission warning may accompany rc=0. Do not mistake that for log access.
    unavailable = rc != 0 or not raw.strip() or any(phrase in raw.lower() for phrase in (
        "permission denied", "insufficient permissions", "not seeing messages", "no journal files", "no entries"))
    rows.append({"id": "RECENT_SERVICE_ERROR_HINTS", "status": "unverified" if unavailable else "observed",
                 "signature_counts": {} if unavailable else journal_counts(raw),
                 "scope": "At most 100 recent service lines/1MiB, last 5 minutes; may include other clients; absence is not proof of successful streaming."})
    if any(row["status"] == "fail" for row in rows):
        code = "fix_reported_service_or_deployment_layer_first"
    elif not unavailable and any(journal_counts(raw).values()):
        code = "review_reported_stream_error_hints_no_automatic_retry"
    else:
        code = "need_phone_error_and_listening_result_not_native_acceptance"
    return {"schema": 2, "candidate": "aysha-browser-playback-triage-v2-transport", "code": code,
            "checks": rows, "starts_audio": False, "upstream_requested": False,
            "service_changed": False, "raw_logs_included": False,
            "audibility_verified": False, "memory_budget_accepted": False,
            "limits": "Read-only installed-service triage, not a native probe. UI equality is not complete backend binary identity. Cannot inspect the phone's media element or hear output. No all-Rust/50MB acceptance."}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--expect-transport", choices=["legacy", "rust-resolved"],
                        help="Check the API-reported mode without changing service configuration")
    args = parser.parse_args(argv)
    report = collect(args.expect_transport)
    print(json.dumps(report, indent=2))
    return 1 if any(row["status"] == "fail" for row in report["checks"]) else 2


if __name__ == "__main__":
    raise SystemExit(main())
