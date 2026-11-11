#!/usr/bin/env python3
"""Read-only local music-server smoke checks. No third-party packages required."""
import argparse
import json
import sys
import urllib.error
import urllib.parse
import urllib.request


def media_type(data):
    if data.startswith(b"\x1aE\xdf\xa3"):
        return "audio/webm"
    if len(data) >= 8 and data[4:8] == b"ftyp":
        return "audio/mp4"
    if data.startswith(b"OggS"):
        return "audio/ogg"
    if data.startswith(b"ID3") or (len(data) >= 2 and data[0] == 255 and data[1] & 224 == 224):
        return "audio/mpeg"
    return None


def check_stream(response):
    # Reading a bounded sample avoids printing binary data and curl/head pipe errors.
    sample = response.read(4096)
    if not sample:
        raise ValueError("stream returned no audio bytes")
    actual = media_type(sample)
    declared = response.headers.get("Content-Type", "").split(";", 1)[0].strip().lower()
    if actual is None:
        raise ValueError("unrecognized audio container; playback needs manual investigation")
    if declared != actual:
        raise ValueError(f"MIME mismatch: response declares {declared!r}, bytes identify {actual!r}")
    if "bytes" in response.headers.get("Accept-Ranges", "").lower():
        raise ValueError("server advertises byte ranges; Range/206 seeking needs a separate verified test")
    return f"{actual}, received {len(sample)} bytes (decoding not tested)"


def check_json(response, expected):
    value = json.loads(response.read(1024 * 1024))
    if not isinstance(value, expected):
        raise ValueError(f"expected {expected.__name__}, got {type(value).__name__}")
    return value


def run(base_url, video_url=None, timeout=30):
    parsed = urllib.parse.urlsplit(base_url)
    if parsed.scheme not in ("http", "https") or not parsed.hostname or parsed.query or parsed.fragment:
        raise ValueError("base URL must be an HTTP(S) server address without query or fragment")
    base_url = base_url.rstrip("/")
    # Do not let shell proxy settings redirect local checks through an external proxy.
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    failures = 0

    def check(label, path, verify):
        nonlocal failures
        try:
            with opener.open(base_url + path, timeout=timeout) as response:
                if response.status != 200:
                    raise ValueError(f"expected HTTP 200, got {response.status}")
                detail = verify(response)
            print(f"PASS {label}: {detail}")
        except (OSError, ValueError, urllib.error.URLError) as error:
            failures += 1
            print(f"FAIL {label}: {error}")

    def root(response):
        page = response.read(1024 * 1024).decode("utf-8")
        if "<html" not in page.lower() or "<audio" not in page.lower():
            raise ValueError("root page must contain HTML and a browser audio element")
        return "HTML and audio element present"

    def status(response):
        value = check_json(response, dict)
        required = {"is_playing", "queue_len", "liked_count"}
        missing = required - value.keys()
        if missing:
            raise ValueError(f"missing status fields: {sorted(missing)}")
        if not isinstance(value["is_playing"], bool):
            raise ValueError("is_playing must be a boolean")
        for key in ("queue_len", "liked_count"):
            if type(value[key]) is not int or value[key] < 0:
                raise ValueError(f"{key} must be a non-negative integer")
        return "status schema valid"

    def queue(response):
        tracks = check_json(response, list)
        for track in tracks:
            if not isinstance(track, dict) or not all(
                isinstance(track.get(key), str)
                for key in ("id", "title", "artist", "thumbnail", "url")
            ):
                raise ValueError("queue contains an invalid track schema")
        return f"{len(tracks)} queued tracks"

    check("UI", "/", root)
    check("status", "/api/status", status)
    check("queue", "/api/queue", queue)
    if video_url:
        query = urllib.parse.urlencode({"url": video_url})
        check("audio transport and metadata", "/api/stream?" + query, check_stream)
    else:
        print("SKIP audio: supply --video-url with a YouTube track you can play")
    print("Manual gates: browser decoding, Bluetooth, fallback, autoplay, reboot and memory remain unverified.")
    return 1 if failures else 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-url", default="http://127.0.0.1:5050")
    parser.add_argument("--video-url")
    parser.add_argument("--timeout", type=float, default=30)
    args = parser.parse_args()
    if args.timeout <= 0:
        parser.error("--timeout must be positive")
    try:
        return run(args.base_url, args.video_url, args.timeout)
    except ValueError as error:
        parser.error(str(error))


if __name__ == "__main__":
    sys.exit(main())
