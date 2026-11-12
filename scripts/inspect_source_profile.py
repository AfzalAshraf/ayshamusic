#!/usr/bin/env python3
"""Offline default-profile comparison. No HTTP, cookies, config, playback or builds.

Reads the installed Python yt-dlp package defaults, NOT a live request/session.
Values outside a small fixed field list are never emitted. A Python package may
not match the shell's yt-dlp executable: report that limitation explicitly.
"""
import argparse
import copy
import json
import re
import socket
from unittest.mock import patch

NATIVE = {
    "clientName": "VISIONOS", "clientVersion": "1.02", "deviceMake": "Apple",
    "deviceModel": "RealityDevice17,1", "osName": "visionOS", "osVersion": "26.5.23O471",
    "hl": "en",
}
NATIVE_AGENT = "Mozilla/5.0 (Macintosh; Intel Mac OS X 15_7_3) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15"
FIELDS = tuple(NATIVE) + ("timeZone", "utcOffsetMinutes")


def public_scalar(value):
    # Versions/device labels only; no arbitrary upstream fields, URLs or session IDs.
    if type(value) is int and -1440 <= value <= 1440:
        return value
    if isinstance(value, str) and re.fullmatch(r"[A-Za-z0-9_. ,+-]{1,64}", value):
        return value
    return None


def summarize(defaults, context, headers, version):
    client = context.get("client", {})
    if not isinstance(client, dict):
        raise ValueError("invalid context")
    fields = {}
    for field in FIELDS:
        value = public_scalar(client.get(field))
        fields[field] = {"installed_default": value, "native_v2": NATIVE.get(field),
                         "matches": value == NATIVE.get(field)}
    # Boolean comparison only. Never print user-agent/header/session values.
    return {
        "status": "pass", "code": "offline_default_profile_inspected",
        "package_version": public_scalar(version),
        "network_executed": False, "native_executed": False,
        "fields": fields,
        "client_header_is_101": headers.get("X-YouTube-Client-Name") == "101",
        "user_agent_matches_native_v2": headers.get("User-Agent") == NATIVE_AGENT,
        "origin_header_present_in_default_builder": bool(headers.get("Origin")),
        "origin_header_present_in_native_v2": False,
        "visitor_header_present_with_empty_offline_config": bool(headers.get("X-Goog-Visitor-Id")),
        "default_requires_js_player": defaults.get("REQUIRE_JS_PLAYER") is True,
        "default_requires_authentication": defaults.get("REQUIRE_AUTH") is True,
        "scope": "installed_python_package_defaults_not_live_request_or_shell_executable_identity",
        "limits": "No watch-page/visitor/cookie/session input. Does not identify selected media client, prove request equivalence, explain bot checks, or validate a native fix. Missing/default fields may differ by version.",
    }


class SilentLogger:
    def debug(self, *_args, **_kwargs):
        pass

    warning = error = debug


def blocked_network(*_args, **_kwargs):
    raise RuntimeError("network_disabled_in_offline_profile_probe")


def inspect_installed():
    # Guard all Python socket connections plus the downloader's request entrypoint.
    # This script imports the existing package; it is not a hostile-code sandbox.
    with patch.object(socket.socket, "connect", blocked_network), \
            patch.object(socket.socket, "connect_ex", blocked_network), \
            patch.object(socket, "create_connection", blocked_network):
        from yt_dlp import YoutubeDL
        from yt_dlp.extractor.youtube import YoutubeIE
        from yt_dlp.version import __version__
        with patch.object(YoutubeDL, "urlopen", blocked_network):
            with YoutubeDL({"cachedir": False, "quiet": True, "no_warnings": True,
                            "logger": SilentLogger(), "plugin_dirs": []}, auto_init=False) as downloader:
                extractor = YoutubeIE(downloader)
                # No initialize/extract calls: no consent/auth cookie initialization,
                # page/visitor retrieval, PO token/JS provider or network request.
                defaults = copy.deepcopy(extractor._get_default_ytcfg("visionos"))
                context = extractor._extract_context({}, default_client="visionos")
                headers = extractor.generate_api_headers(ytcfg={}, default_client="visionos")
                return summarize(defaults, context, headers, __version__)


def main(argv=None):
    argparse.ArgumentParser(description=__doc__).parse_args(argv)
    try:
        report = inspect_installed()
    except ImportError:
        report = {"status": "fail", "code": "python_package_unavailable_or_incompatible",
                  "network_executed": False,
                  "next": "Do not install/upgrade anything. Share this fixed error and yt-dlp --version; the shell executable may use another interpreter or standalone package."}
    except Exception:
        report = {"status": "fail", "code": "offline_profile_api_changed_or_network_blocked",
                  "network_executed": False,
                  "next": "Do not retry network or change production. Review the installed package API/version before adjusting this offline probe."}
    print(json.dumps(report, indent=2))
    return 0 if report["status"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
