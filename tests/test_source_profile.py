import contextlib
import io
import json
import socket
from pathlib import Path
import unittest
from unittest import mock
from scripts import inspect_source_profile as profile


class SourceProfileTests(unittest.TestCase):
    def test_v2_reference_still_matches_the_unchanged_native_source(self):
        source = (Path(__file__).resolve().parents[1] / "prototypes/rust-direct/src/main.rs").read_text()
        self.assertIn('"aysha-direct-v2-visionos"', source)
        self.assertIn('const VERSION: &str = "' + profile.NATIVE["clientVersion"] + '"', source)
        self.assertIn('const AGENT: &str = "' + profile.NATIVE_AGENT + '"', source)
        for key, value in profile.NATIVE.items():
            if key == "clientVersion":
                continue
            self.assertRegex(source, '"' + key + r'"\s*:\s*"' + value.replace('.', r'\.') + '"')
        # The offline reference describes legacy v2; v3 changes are opt-in.
        self.assertIn('if profile != RequestProfile::V2', source)
        self.assertIn('[flag, id] if flag == "--video-id"', source)
        self.assertIn('=> RequestProfile::V2', source)

    def test_exact_client_fields_and_only_booleans_for_headers(self):
        client = dict(profile.NATIVE, timeZone="UTC", utcOffsetMinutes=0,
                      visitorData="PRIVATE_VISITOR", arbitrary="PRIVATE_SECRET")
        headers = {"User-Agent": profile.NATIVE_AGENT, "X-YouTube-Client-Name": "101",
                   "Origin": "https://www.youtube.com", "X-Goog-Visitor-Id": "PRIVATE_VISITOR"}
        result = profile.summarize({}, {"client": client}, headers, "2026.11.01")
        self.assertTrue(result["user_agent_matches_native_v2"])
        self.assertTrue(result["client_header_is_101"])
        self.assertFalse(result["fields"]["timeZone"]["matches"])
        self.assertFalse(result["fields"]["utcOffsetMinutes"]["matches"])
        self.assertTrue(result["visitor_header_present_with_empty_offline_config"])
        self.assertNotIn("PRIVATE", json.dumps(result))
        self.assertNotIn("https://", json.dumps(result))

    def test_defaults_are_not_relabelled_as_wire_evidence(self):
        result = profile.summarize({"REQUIRE_AUTH": True, "REQUIRE_JS_PLAYER": False},
                                   {"client": {"clientVersion": "2.00"}}, {}, "2026.11.01")
        self.assertFalse(result["fields"]["clientVersion"]["matches"])
        self.assertEqual(result["fields"]["clientVersion"]["installed_default"], "2.00")
        self.assertTrue(result["default_requires_authentication"])
        self.assertFalse(result["network_executed"])
        self.assertIn("not_live_request", result["scope"])

    def test_unbounded_or_unsafe_values_are_not_serialized(self):
        result = profile.summarize({}, {"client": {"clientVersion": "https://secret/",
                                  "userAgent": "private", "utcOffsetMinutes": 999999}}, {}, "x"*10000)
        self.assertIsNone(result["package_version"])
        self.assertIsNone(result["fields"]["clientVersion"]["installed_default"])
        self.assertIsNone(result["fields"]["utcOffsetMinutes"]["installed_default"])
        self.assertNotIn("secret", json.dumps(result))

    def test_errors_are_fixed_and_do_not_print_raw_exceptions(self):
        for exception, code in [(ImportError("private path"), "python_package_unavailable_or_incompatible"),
                                (RuntimeError("secret token"), "offline_profile_api_changed_or_network_blocked")]:
            output = io.StringIO()
            with mock.patch.object(profile, "inspect_installed", side_effect=exception), contextlib.redirect_stdout(output):
                self.assertEqual(profile.main([]), 1)
            result = json.loads(output.getvalue())
            self.assertEqual(result["code"], code)
            self.assertNotIn("private", output.getvalue())
            self.assertNotIn("secret", output.getvalue())

    def test_network_guard_does_not_open_a_connection(self):
        with self.assertRaises(RuntimeError):
            profile.blocked_network("example.invalid", 443)

    def test_script_restores_socket_methods_after_failed_import(self):
        original = socket.socket.connect
        real_import = __import__

        def intercept(name, *args, **kwargs):
            if name == "yt_dlp":
                self.assertIs(socket.socket.connect, profile.blocked_network)
                self.assertIs(socket.create_connection, profile.blocked_network)
                raise ImportError("fixture")
            return real_import(name, *args, **kwargs)

        with mock.patch("builtins.__import__", side_effect=intercept):
            with self.assertRaises(ImportError):
                profile.inspect_installed()
        self.assertIs(socket.socket.connect, original)


if __name__ == "__main__":
    unittest.main()
