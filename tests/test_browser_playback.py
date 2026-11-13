import contextlib
import io
import json
import unittest
from unittest import mock

from scripts import check_browser_playback as diagnostic


class BrowserPlaybackTriageTests(unittest.TestCase):
    def run_report(self, journal="ordinary service message", journal_rc=0, served=None, active=True, mode=None, stream_active=None, expected_mode=None):
        expected = (diagnostic.health.ROOT / 'src/index.html').read_bytes()
        def command(argv, timeout):
            if argv[0] == 'systemctl':
                return (0, 'ActiveState=active\nSubState=running' if active else 'ActiveState=failed')
            self.assertEqual(argv[0], 'journalctl')
            self.assertIn('100', argv)
            self.assertIn('5 minutes ago', argv)
            return journal_rc, journal
        def http(path):
            self.assertIn(path, ['/', '/api/status'])
            return (expected if served is None else served) if path == '/' else json.dumps({
                "is_playing": True, "current_track": {"title": "PRIVATE"},
                "stream_transport": mode, "rust_stream_active": stream_active}).encode()
        with mock.patch.object(diagnostic.health, 'command', side_effect=command), \
                mock.patch.object(diagnostic.health, 'http_get', side_effect=http):
            return diagnostic.collect(expected_mode)

    def test_routes_are_local_readonly_no_media_or_controls_and_status_not_a_pass(self):
        report = self.run_report()
        self.assertFalse(report['starts_audio'])
        self.assertFalse(report['upstream_requested'])
        self.assertFalse(report['service_changed'])
        self.assertFalse(report['audibility_verified'])
        self.assertFalse(report['memory_budget_accepted'])
        self.assertNotIn('PRIVATE', json.dumps(report))
        self.assertEqual(report['code'], 'need_phone_error_and_listening_result_not_native_acceptance')

    def test_stale_embedded_page_and_stopped_service_have_actionable_failure(self):
        for kwargs in ({'served': b'old html'}, {'active': False}):
            report = self.run_report(**kwargs)
            self.assertEqual(report['code'], 'fix_reported_service_or_deployment_layer_first')

    def test_error_categories_never_return_raw_urls_tokens_or_log_text(self):
        report = self.run_report(journal='ERROR: PRIVATE https://example.invalid/TOKEN HTTP Error 403 Forbidden\nyt-dlp returned no usable audio header: PRIVATE')
        hints = report['checks'][-1]['signature_counts']
        self.assertEqual(hints['UPSTREAM_HTTP_403'], 1)
        self.assertEqual(hints['STREAM_NO_AUDIO_HEADER'], 1)
        self.assertEqual(report['code'], 'review_reported_stream_error_hints_no_automatic_retry')
        for value in ['PRIVATE', 'TOKEN', 'example.invalid']:
            self.assertNotIn(value, json.dumps(report))

    def test_permission_warning_empty_or_missing_journal_is_not_a_clean_log_pass(self):
        for text, rc in [('', 0), ('Permission denied PRIVATE', 0), ('not seeing messages', 0), ('PRIVATE', 1), ('', 127)]:
            report = self.run_report(journal=text, journal_rc=rc)
            self.assertEqual(report['checks'][-1]['status'], 'unverified')
            self.assertEqual(report['checks'][-1]['signature_counts'], {})

    def test_json_arrays_or_unavailable_status_do_not_crash_or_expose_details(self):
        for result in [b'[]', b'PRIVATE', OSError('PRIVATE')]:
            with mock.patch.object(diagnostic.health, 'command', return_value=(127, 'PRIVATE')), \
                    mock.patch.object(diagnostic.health, 'http_get', side_effect=[b'old', result]):
                report = diagnostic.collect()
            self.assertNotIn('PRIVATE', json.dumps(report))
            self.assertEqual(report['checks'][2]['status'], 'fail')

    def test_transport_expectation_distinguishes_trial_legacy_invalid_and_unknown_backend(self):
        for mode, expected in [('rust-resolved', 'observed'), ('legacy', 'fail'), ('invalid', 'fail'), (None, 'unverified')]:
            report = self.run_report(mode=mode, stream_active=False, expected_mode='rust-resolved')
            row = next(row for row in report['checks'] if row['id'] == 'STREAM_TRANSPORT')
            self.assertEqual(row['status'], expected)
            self.assertIs(row['rust_stream_active'], False)
            self.assertFalse(report['audibility_verified'])
            self.assertFalse(report['memory_budget_accepted'])

    def test_untrusted_mode_or_active_values_never_escape_into_report(self):
        for mode in ['PRIVATE', [], {}, 1, True]:
            report = self.run_report(mode=mode, stream_active='PRIVATE')
            row = next(row for row in report['checks'] if row['id'] == 'STREAM_TRANSPORT')
            self.assertEqual(row['configured'], 'unverified')
            self.assertIsNone(row['rust_stream_active'])
            self.assertNotIn('PRIVATE', json.dumps(report))

    def test_fixed_rust_failure_categories_are_reported_without_raw_resolver_output(self):
        report = self.run_report(journal='Rust-resolved stream: resolver_origin_rejected PRIVATE https://secret.invalid/TOKEN\n'
                                 'Rust-resolved stream: metadata_exit_failed\n'
                                 'Rust-resolved stream: native_stream_response_rejected\n'
                                 'Rust-resolved stream: native_startup_timeout', mode='rust-resolved')
        counts = report['checks'][-1]['signature_counts']
        for code in ['RUST_RESOLVER_RECORD_REJECTED', 'RUST_RESOLVER_FAILED', 'RUST_CDN_OR_HEADER_FAILED', 'RUST_STARTUP_TIMEOUT']:
            self.assertEqual(counts[code], 1)
        for secret in ['PRIVATE', 'TOKEN', 'secret.invalid']:
            self.assertNotIn(secret, json.dumps(report))

    def test_invalid_transport_cli_never_runs_probes(self):
        with mock.patch.object(diagnostic, 'collect') as collect, contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit) as error:
                diagnostic.main(['--expect-transport', 'PRIVATE'])
        self.assertEqual(error.exception.code, 2)
        collect.assert_not_called()

    def test_cli_never_claims_full_success(self):
        for state, expected in [('fail', 1), ('unverified', 2), ('pass', 2)]:
            output = io.StringIO()
            with mock.patch.object(diagnostic, 'collect', return_value={'checks': [{'status': state}]}), contextlib.redirect_stdout(output):
                self.assertEqual(diagnostic.main([]), expected)
            self.assertEqual(json.loads(output.getvalue())['checks'][0]['status'], state)


if __name__ == '__main__':
    unittest.main()
