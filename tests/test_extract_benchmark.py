"""Offline benchmark-harness tests, not live YouTube extraction acceptance."""
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("benchmark", Path(__file__).resolve().parents[1] / "scripts/benchmark_extract.py")
benchmark = importlib.util.module_from_spec(spec)
spec.loader.exec_module(benchmark)


class BenchmarkTests(unittest.TestCase):
    def test_native_metrics_are_allowlisted_and_startup_inclusive(self):
        payload = {"status": "pass", "bytes": 65536, "total_ms": 1, "resolve_ms": 1,
                   "first_byte_ms": 1, "url": "signed-secret-url", "peak_rss_kib": 12000}
        row = benchmark.execute([sys.executable, "-c", "print(" + repr(json.dumps(payload)) + ")"])
        self.assertEqual(row["status"], "pass")
        self.assertEqual(row["native_total_ms"], 1)
        self.assertGreater(row["total_ms"], 1)
        self.assertNotIn("signed-secret", str(row))

    def test_failure_and_untrusted_error_text_are_not_echoed(self):
        payload = json.dumps({"status": "fail", "code": "https://signed-secret-url", "peak_rss_kib": 26000})
        row = benchmark.execute([sys.executable, "-c", "print(" + repr(payload) + "); raise SystemExit(1)"])
        self.assertEqual(row["status"], "fail")
        self.assertEqual(row["code"], "native_failure")
        self.assertEqual(row["peak_rss_kib"], 26000)
        self.assertNotIn("signed-secret", str(row))

    def test_format_counts_are_allowlisted_on_failure(self):
        payload = {"status":"fail", "code":"all_aac_urls_missing", "format_counts":{
            "aac_candidates":2, "empty_urls":2, "url":"signed-secret",
            "formats_total":-1, "invalid_urls":True, "usable_urls":"signed-secret"}}
        row = benchmark.execute([sys.executable, "-c", "print(" + repr(json.dumps(payload)) + "); raise SystemExit(1)"])
        self.assertEqual(row["format_counts"], {"aac_candidates":2, "empty_urls":2})
        self.assertNotIn("signed-secret", str(row))

    def test_binary_probe_checks_bytes_and_container(self):
        row = benchmark.execute([sys.executable, "-c", "import sys; sys.stdout.buffer.write(b'\\0\\0\\0\\x20ftyp'+b'x'*(65536-8))"], binary_media=True)
        self.assertEqual(row["status"], "pass")
        self.assertEqual(row["bytes"], 65536)
        self.assertIsNone(row["peak_rss_kib"])
        row = benchmark.execute([sys.executable, "-c", "print('not audio')"], binary_media=True)
        self.assertEqual(row["status"], "fail")
        self.assertLess(row["bytes"], 65536)
        self.assertFalse(row["mp4_header"])

    def test_missing_binary_and_timeout_return_failure(self):
        self.assertEqual(benchmark.execute(["/not-an-executable"])["status"], "fail")
        row = benchmark.execute([sys.executable, "-c", "import time; time.sleep(30)"], deadline=0.05)
        self.assertEqual(row["code"], "deadline_exceeded")

    def test_group_rss_counts_only_owned_group_including_children(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for pid, group, rss in [(10, 10, 50), (11, 10, 75), (12, 20, 900)]:
                path = root / str(pid)
                path.mkdir()
                (path / "stat").write_text(f"{pid} (test with ) spaces) S 1 {group} 0")
                (path / "status").write_text(f"VmRSS: {rss} kB\n")
            self.assertEqual(benchmark.group_rss_kib(10, root), 125)
            self.assertIsNone(benchmark.group_rss_kib(99, root))

    def test_non_object_json_fails_cleanly(self):
        self.assertEqual(benchmark.execute([sys.executable, "-c", "print('[]')"])["status"], "fail")

    def test_summary_reports_failed_attempts_not_just_fast_successes(self):
        result = benchmark.summarize([{"tool": "rust", "status": "pass", "total_ms": 20},
                                      {"tool": "rust", "status": "fail"},
                                      {"tool": "yt-dlp", "status": "pass", "total_ms": 10}])
        self.assertEqual(result["rust"], {"successes": 1, "attempts": 2, "median_probe_ms": 20})
        self.assertEqual(benchmark.summarize([])["rust"]["median_probe_ms"], None)


if __name__ == "__main__":
    unittest.main()
