"""Offline reporter tests; never probe or change the developer's real service."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import stat
import sys
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("health", Path(__file__).resolve().parents[1] / "scripts/check_health.py")
health = importlib.util.module_from_spec(spec)
spec.loader.exec_module(health)


class HealthTests(unittest.TestCase):
    def test_memory_peak_from_user_is_failure(self):
        level, detail = health.memory_result("119746560")
        self.assertEqual(level, "FAIL")
        self.assertIn("119.75 MB", detail)
        self.assertIn("114.20 MiB", detail)
        self.assertEqual(health.memory_result("21569536")[0], "PASS")

    def test_memory_boundaries_missing_and_decimal_binary_ambiguity(self):
        for size, expected in (("[not set]", "WARN"), ("infinity", "WARN"), ("-1", "WARN"),
                               ("100000000", "PASS"), ("100000001", "WARN"),
                               ("104857600", "WARN"), ("104857601", "FAIL")):
            with self.subTest(size=size):
                self.assertEqual(health.memory_result(size)[0], expected)

    def test_exit_code_precedence_and_markers(self):
        report = health.Report()
        report.add("PASS", "ONE", "ok")
        self.assertEqual(report.exit_code(), 0)
        report.add("WARN", "TWO", "incomplete")
        self.assertEqual(report.exit_code(), 2)
        report.add("FAIL", "THREE", "bad")
        self.assertEqual(report.exit_code(), 1)
        self.assertIn("RETURN_CODE=1", report.render())
        self.assertIn("END AYSHA DEBUG REPORT", report.render())

    def test_saved_reports_are_unique_private_and_exact(self):
        with tempfile.TemporaryDirectory() as tmp, patch.object(health, "ROOT", Path(tmp)):
            one = health.save_report("sample report\n")
            two = health.save_report("second\n")
            self.assertNotEqual(one, two)
            self.assertEqual(one.read_text(), "sample report\n")
            self.assertEqual(stat.S_IMODE(one.stat().st_mode), 0o600)

    def test_missing_command_and_timeout_have_codes(self):
        self.assertEqual(health.command(["/nonexistent/aysha-tool"])[0], 127)
        self.assertEqual(health.command([sys.executable, "-c", "import time; time.sleep(5)"], timeout=0.05)[0], 124)

    def test_failure_output_is_not_echoed(self):
        rc, _ = health.command([sys.executable, "-c", "raise SystemExit(7)"])
        self.assertEqual(rc, 7)

    def fake_command(self, argv, timeout=15):
        if argv[0] == "git":
            return 0, "a" * 40
        if argv[0] == "systemctl":
            return 0, "ActiveState=active\nSubState=running\nNRestarts=0\nMemoryCurrent=21569536\nMemoryPeak=119746560\n"
        if argv[0] == "/usr/bin/node":
            return 0, "v22.22.1\n"
        if "yt-dlp" in argv[0]:
            return 0, "2026.08.19\n"
        if argv[0] == "pactl":
            return 0, "127 alsa_output.foo SUSPENDED\n"
        return 0, "private test output https://secret.invalid/token\n"

    def fake_http(self, path):
        if path == "/":
            return (health.ROOT / "src/index.html").read_bytes()
        if path == "/api/status":
            return json.dumps({"is_playing": False, "queue_len": 9, "liked_count": 0,
                               "status_msg": "secret-user-token"}).encode()
        return b"[]"

    def test_realistic_fixture_has_memory_failure_no_private_payloads(self):
        with patch.object(health, "command", self.fake_command), patch.object(health, "http_get", self.fake_http):
            report = health.collect(code_checks=True)
        text = report.render()
        self.assertEqual(report.exit_code(), 1)
        self.assertIn("[FAIL] MEMORYPEAK", text)
        self.assertIn("[WARN] BLUETOOTH_SINK", text)
        self.assertIn("[PASS] SERVED_UI", text)
        self.assertIn("[PASS] RUST_TESTS", text)
        self.assertNotIn("secret", text)
        self.assertNotIn("https://", text)

    def test_failed_commands_and_http_probes_continue(self):
        with patch.object(health, "command", return_value=(124, "token-secret")), patch.object(health, "http_get", side_effect=ValueError("token-secret")):
            text = health.collect().render()
        self.assertIn("[FAIL] SERVICE", text)
        self.assertIn("[FAIL] HTTP_STATUS", text)
        self.assertIn("[FAIL] HTTP_QUEUE", text)
        self.assertIn("[WARN] CODE_TESTS", text)
        self.assertNotIn("token-secret", text)

    def test_code_suite_failure_and_timeout_are_reported(self):
        def commands(argv, timeout=15):
            if argv[0] == "cargo":
                return 124, "secret timeout log"
            if argv[0] == "node":
                return 7, "secret failed-test log"
            return self.fake_command(argv, timeout)
        with patch.object(health, "command", commands), patch.object(health, "http_get", self.fake_http):
            text = health.collect(code_checks=True).render()
        self.assertIn("[FAIL] FRONTEND_TESTS: exit=7", text)
        self.assertIn("[WARN] RUST_TESTS: exit=124", text)
        self.assertNotIn("secret", text)

    def test_explicit_bluetooth_expectation_is_failure(self):
        with patch.object(health, "command", self.fake_command), patch.object(health, "http_get", self.fake_http):
            text = health.collect(expect_bluetooth=True).render()
        self.assertIn("[FAIL] BLUETOOTH_SINK", text)

    def test_bad_queue_schema_does_not_print_track_data(self):
        def response(path):
            return b'[{"title":"private title"}]' if path == "/api/queue" else self.fake_http(path)
        with patch.object(health, "command", self.fake_command), patch.object(health, "http_get", response):
            text = health.collect().render()
        self.assertIn("[FAIL] HTTP_QUEUE", text)
        self.assertNotIn("private title", text)

    def test_main_prints_and_saves_same_report_with_failure_exit(self):
        with tempfile.TemporaryDirectory() as tmp, patch.object(health, "ROOT", Path(tmp)), patch.object(health, "command", return_value=(127, "")), patch.object(health, "http_get", side_effect=OSError()), patch.object(sys, "argv", ["check_health.py"]):
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                rc = health.main()
            self.assertEqual(rc, 1)
            files = list((Path(tmp) / "diagnostics").glob("*.txt"))
            self.assertEqual(len(files), 1)
            self.assertIn(files[0].read_text(), output.getvalue())

    def test_save_failure_is_reporter_error(self):
        with patch.object(health, "collect", return_value=health.Report()), patch.object(health, "save_report", side_effect=OSError("secret")), patch.object(sys, "argv", ["check_health.py"]):
            output = io.StringIO()
            with contextlib.redirect_stderr(output):
                rc = health.main()
            self.assertEqual(rc, 3)
            self.assertNotIn("secret", output.getvalue())

    def test_redirects_are_rejected(self):
        handler = health.NoRedirect()
        self.assertIsNone(handler.redirect_request(None, None, 302, "redirect", {}, "https://example.invalid/"))


if __name__ == "__main__":
    unittest.main()
