"""Offline regression tests for the smoke checker, not the Rust application."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

spec = importlib.util.spec_from_file_location("test_local", Path(__file__).resolve().parents[1] / "scripts/test_local.py")
smoke = importlib.util.module_from_spec(spec)
spec.loader.exec_module(smoke)


class Response(io.BytesIO):
    def __init__(self, data, headers):
        super().__init__(data)
        self.headers = headers


class MediaTests(unittest.TestCase):
    def test_webm(self):
        self.assertEqual(smoke.media_type(b"\x1aE\xdf\xa3webm"), "audio/webm")

    def test_mp4(self):
        self.assertEqual(smoke.media_type(b"\x00\x00\x00\x20ftypM4A "), "audio/mp4")

    def test_ogg(self):
        self.assertEqual(smoke.media_type(b"OggSdata"), "audio/ogg")

    def test_mp3(self):
        self.assertEqual(smoke.media_type(b"ID3data"), "audio/mpeg")

    def test_unknown_is_not_audio(self):
        self.assertIsNone(smoke.media_type(b'{"error":"failed"}'))

    def test_empty_stream_fails(self):
        with self.assertRaisesRegex(ValueError, "no audio bytes"):
            smoke.check_stream(Response(b"", {}))

    def test_observed_mime_mismatch_fails(self):
        with self.assertRaisesRegex(ValueError, "MIME mismatch"):
            smoke.check_stream(Response(b"\x1aE\xdf\xa3webm", {"Content-Type": "audio/mpeg"}))

    def test_correct_webm_passes(self):
        result = smoke.check_stream(Response(b"\x1aE\xdf\xa3webm", {"Content-Type": "audio/webm"}))
        self.assertIn("audio/webm", result)

    def test_unverified_ranges_fail(self):
        with self.assertRaisesRegex(ValueError, "Range/206"):
            smoke.check_stream(Response(b"ID3data", {"Content-Type": "audio/mpeg", "Accept-Ranges": "bytes"}))

    def test_invalid_base_url(self):
        with self.assertRaises(ValueError):
            smoke.run("file:///etc/passwd")


class HttpTests(unittest.TestCase):
    def test_smoke_checks_against_fixture(self):
        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *args):
                pass

            def do_GET(self):
                fixtures = {
                    "/": b'<html><audio></audio></html>',
                    "/api/status": json.dumps({"is_playing": False, "queue_len": 0, "liked_count": 0}).encode(),
                    "/api/queue": b"[]",
                }
                body = fixtures.get(self.path)
                self.send_response(200 if body else 404)
                self.end_headers()
                self.wfile.write(body or b"not found")

        with ThreadingHTTPServer(("127.0.0.1", 0), Handler) as server:
            worker = threading.Thread(target=server.serve_forever, daemon=True)
            worker.start()
            try:
                output = io.StringIO()
                with contextlib.redirect_stdout(output):
                    result = smoke.run(f"http://127.0.0.1:{server.server_port}")
                self.assertEqual(result, 0, output.getvalue())
                self.assertEqual(output.getvalue().count("PASS "), 3)
                self.assertIn("SKIP audio", output.getvalue())
            finally:
                server.shutdown()
                worker.join(timeout=5)


if __name__ == "__main__":
    unittest.main()
