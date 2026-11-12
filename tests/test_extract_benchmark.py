"""Offline benchmark-harness tests, not live YouTube extraction acceptance."""
import contextlib
import io
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

spec = importlib.util.spec_from_file_location("benchmark", Path(__file__).resolve().parents[1] / "scripts/benchmark_extract.py")
benchmark = importlib.util.module_from_spec(spec)
spec.loader.exec_module(benchmark)


class BenchmarkTests(unittest.TestCase):
    def run_baseline(self, result):
        with tempfile.TemporaryDirectory() as directory, mock.patch.object(benchmark, "ROOT", Path(directory)), \
                mock.patch.object(benchmark, "BINARY", Path(directory) / "missing-native"), \
                mock.patch.object(benchmark, "execute", return_value=result) as execute, \
                contextlib.redirect_stdout(io.StringIO()):
            code = benchmark.main(["--baseline-only", "--video-id", "Jm7Satp7Zqk", "--runs", "1"])
            reports = list((Path(directory) / "diagnostics").glob("*.json"))
            self.assertEqual(len(reports), 1)
            report = json.loads(reports[0].read_text())
            self.assertEqual(reports[0].stat().st_mode & 0o777, 0o600)
            return code, report, execute.call_args_list

    def test_baseline_only_needs_no_native_binary_and_never_runs_it(self):
        code, report, calls = self.run_baseline({"status":"pass", "bytes":65536, "total_ms":100})
        self.assertEqual(code, 0)
        self.assertEqual(report["mode"], "baseline_only")
        self.assertFalse(report["native_executed"])
        self.assertEqual(report["summary"]["rust"]["attempts"], 0)
        self.assertEqual(len(calls), 1)
        argv = calls[0].args[0]
        self.assertEqual(argv[0], "yt-dlp")
        self.assertIn("--ignore-config", argv)
        self.assertNotIn("--cookies", argv)
        self.assertNotIn("--cookies-from-browser", argv)
        self.assertEqual(argv[-2:], ["--", "https://www.youtube.com/watch?v=Jm7Satp7Zqk"])
        self.assertTrue(calls[0].kwargs["binary_media"])

    def test_baseline_failure_remains_failure_without_native_fallback(self):
        code, report, calls = self.run_baseline({"status":"fail", "code":"short_or_non_mp4_probe"})
        self.assertEqual(code, 1)
        self.assertEqual(report["results"][0]["status"], "fail")
        self.assertEqual(len(calls), 1)
        self.assertFalse(report["native_executed"])

    def test_baseline_still_rejects_unsafe_ids_and_unbounded_run_counts(self):
        for args in [
            ["--video-id", "https://evil.test"],
            ["--video-id", "Jm7Satp7Zqk", "--runs", "6"],
        ]:
            with contextlib.redirect_stderr(io.StringIO()), mock.patch.object(benchmark, "execute") as execute:
                with self.assertRaises(SystemExit) as error:
                    benchmark.main(["--baseline-only", *args])
                self.assertEqual(error.exception.code, 2)
                execute.assert_not_called()

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
        payload = {"status":"fail", "candidate":"rusty_ytdl-0.7.4", "code":"all_aac_urls_missing", "format_counts":{
            "aac_candidates":2, "empty_urls":2, "url":"signed-secret",
            "formats_total":-1, "invalid_urls":True, "usable_urls":"signed-secret"}}
        row = benchmark.execute([sys.executable, "-c", "print(" + repr(json.dumps(payload)) + "); raise SystemExit(1)"])
        self.assertEqual(row["format_counts"], {"aac_candidates":2, "empty_urls":2})
        self.assertEqual(row["candidate"], "rusty_ytdl-0.7.4")
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
