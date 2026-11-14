"""Bounded, lossy classifier for yt-dlp stderr. Never persists or echoes raw lines.

Diagnostic observations only: log wording can change; absent markers prove nothing.
Not a bot-check solver, packet capture, credential collector or production component.
"""
import re

CLIENTS = frozenset(("web", "web safari", "web embedded", "web music", "web creator",
                    "mweb", "android", "android vr", "android sdkless", "ios", "visionos",
                    "tv", "tv samsung", "tv downgraded", "tv embedded", "tv simply"))
RUNTIMES = frozenset(("node", "deno", "quickjs", "quickjs-ng", "bun"))


class StageTrace:
    MAX_BYTES = 262144
    MAX_LINE = 4096

    def __init__(self):
        self._pending = bytearray()
        self._discard_line = False
        self._seen = 0
        self._clients = set()
        self._runtimes = set()
        self._player_page = False
        self._js_challenge = False
        self._truncated = False

    def feed(self, data):
        remaining = max(0, self.MAX_BYTES - self._seen)
        self._seen += len(data)
        if len(data) > remaining:
            self._truncated = True
        # Drain further process output elsewhere even when classification is capped.
        for byte in data[:remaining]:
            if byte == 10:
                if not self._discard_line:
                    self._line(bytes(self._pending).rstrip(b"\r"))
                self._pending.clear()
                self._discard_line = False
            elif not self._discard_line:
                if len(self._pending) >= self.MAX_LINE:
                    self._pending.clear()
                    self._discard_line = True
                    self._truncated = True
                else:
                    self._pending.append(byte)

    def _line(self, raw):
        line = raw.decode("utf-8", errors="replace")
        if line.startswith("[youtube] "):
            match = re.search(r": Downloading ([a-z ]{1,32}) player API JSON$", line)
            if match and match[1] in CLIENTS:
                self._clients.add(match[1].replace(" ", "_"))
            if line.endswith(": Downloading webpage"):
                self._player_page = True
        # A configured-runtime listing alone is deliberately not an execution signal.
        for match in re.finditer(r"\[jsc:([a-z-]{1,16})\]", line):
            if match[1] in RUNTIMES:
                self._runtimes.add(match[1])
                if "Solving JS challenges" in line:
                    self._js_challenge = True

    def report(self):
        return {"client_request_markers": sorted(self._clients),
                "js_provider_markers": sorted(self._runtimes),
                "js_challenge_marker_seen": self._js_challenge,
                "webpage_request_marker_seen": self._player_page,
                "classification_truncated": self._truncated,
                "scope": "allowlisted_log_markers_only_not_runtime_equivalence_or_selected_client_proof"}
