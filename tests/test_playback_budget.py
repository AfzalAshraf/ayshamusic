import contextlib
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

from scripts import check_playback_budget as budget


def sample(current=8_000_000, peak=9_000_000, swap=0):
    return {"memory.current": current, "memory.peak": peak, "memory.swap.current": swap}


def state():
    return {"ActiveState": "active", "SubState": "running", "ControlGroup": "/system.slice/aysha-music.service",
            "MainPID": "123", "NRestarts": "0"}


class PlaybackBudgetTests(unittest.TestCase):
    def test_current_cgroup_above_both_limits_is_failure_even_without_audibility_proof(self):
        report = budget.evaluate([sample(87_000_000)], [True, True], True, False)
        self.assertEqual(report['status'], 'fail')
        self.assertTrue(report['sampled_current_exceeds_15_MB'])
        self.assertTrue(report['sampled_current_exceeds_15_MiB'])
        self.assertFalse(report['audible_output_verified'])
        self.assertFalse(report['download_or_decode_verified'])
        self.assertFalse(report['whole_stack_accepted'])

    def test_decimal_and_binary_thresholds_are_distinct(self):
        for amount, decimal, binary in [(15_000_000, False, False), (15_000_001, True, False),
                                        (15 * 1048576, True, False), (15 * 1048576 + 1, True, True)]:
            report = budget.evaluate([sample(amount, amount)], [True, True], True, False)
            self.assertEqual(report['sampled_current_exceeds_15_MB'], decimal)
            self.assertEqual(report['sampled_current_exceeds_15_MiB'], binary)
            self.assertFalse(report['whole_stack_accepted'])

    def test_under_limit_never_certifies_whole_stack(self):
        report = budget.evaluate([sample()], [True, True], True, False)
        self.assertEqual(report['status'], 'incomplete')
        self.assertEqual(report['code'], 'sampled_service_under_upper_limit_whole_stack_still_unproved')
        self.assertFalse(report['native_acquisition_verified'])
        self.assertFalse(report['whole_stack_accepted'])

    def test_idle_or_unknown_playback_never_becomes_a_playback_pass(self):
        for flags in ([False, True], [True, None], [None, None]):
            report = budget.evaluate([sample()], flags, True, False)
            self.assertEqual(report['code'], 'playback_not_reported_at_both_endpoints')
            self.assertFalse(report['whole_stack_accepted'])

    def test_historical_peak_is_kept_separate_from_sampled_current(self):
        report = budget.evaluate([sample(8_000_000, 120_000_000)], [True, True], True, False)
        self.assertEqual(report['status'], 'incomplete')
        self.assertFalse(report['sampled_current_exceeds_15_MB'])
        self.assertTrue(report['historical_peak_exceeds_15_MB'])
        self.assertIn('historical_peak', report['code'])

    def test_swap_and_changed_service_prevent_budget_acceptance(self):
        report = budget.evaluate([sample(swap=1)], [True, True], True, False)
        self.assertEqual(report['code'], 'swap_observed_budget_not_accepted')
        report = budget.evaluate([sample()], [True, True], False, False)
        self.assertEqual(report['code'], 'service_changed_during_window')

    def test_no_samples_remains_incomplete(self):
        self.assertEqual(budget.evaluate([], [True, True], True, True)['code'], 'no_readable_memory_samples')

    def test_cgroup_path_traversal_and_root_fail_closed(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for path in ('', '/', '../private', '/../../private'):
                with self.assertRaises(ValueError):
                    budget.group_path(path, root)
            self.assertEqual(budget.group_path('/system.slice/aysha-music.service', root),
                             root / 'system.slice/aysha-music.service')

    def test_status_reads_only_fixed_loopback_status_and_never_returns_track_metadata(self):
        private = b'{"is_playing":true,"current_track":{"title":"PRIVATE"}}'
        with mock.patch.object(budget.health, 'http_get', return_value=private) as get:
            self.assertIs(budget.playback_flag(), True)
            get.assert_called_once_with('/api/status')
        for body in (b'[]', b'{"is_playing":1}', b'PRIVATE'):
            with mock.patch.object(budget.health, 'http_get', return_value=body):
                self.assertIsNone(budget.playback_flag())

    def test_readonly_systemctl_state_is_validated_and_no_raw_error_is_exposed(self):
        text = '\n'.join(f'{k}={v}' for k, v in state().items())
        with mock.patch.object(budget.health, 'command', return_value=(0, text)) as run:
            self.assertEqual(budget.service_state()['MainPID'], '123')
            argv = run.call_args.args[0]
            self.assertEqual(argv[:4], ['systemctl', '--no-pager', 'show', 'aysha-music'])
            self.assertNotIn('restart', argv)
        with mock.patch.object(budget.health, 'command', return_value=(1, 'PRIVATE')):
            with self.assertRaises(ValueError):
                budget.service_state()

    def test_collection_keeps_cgroup_and_rss_separate_and_redacts_unknown_process_names(self):
        with tempfile.TemporaryDirectory() as tmp, \
                mock.patch.object(budget, 'service_state', return_value=state()), \
                mock.patch.object(budget, 'playback_flag', return_value=True), \
                mock.patch.object(budget.health, 'cgroup_snapshot', side_effect=[
                    (sample(), {'music-server': (1, 2_000_000), 'PRIVATE': (1, 1_000_000)}, False),
                    (sample(10_000_000), {'music-server': (1, 3_000_000)}, True)]), \
                mock.patch.object(budget.time, 'sleep') as sleep:
            report = budget.collect(2, Path(tmp))
        self.assertEqual(report['sampled_max_current_bytes'], 10_000_000)
        self.assertEqual(report['process_rss_buckets']['music-server']['max_rss_bytes'], 3_000_000)
        self.assertIn('other', report['process_rss_buckets'])
        self.assertTrue(report['process_inventory_incomplete'])
        self.assertNotIn('PRIVATE', json.dumps(report))
        sleep.assert_called_once_with(2)

    def test_sampling_failure_or_restart_never_becomes_pass(self):
        after = dict(state(), MainPID='456')
        with tempfile.TemporaryDirectory() as tmp, \
                mock.patch.object(budget, 'service_state', side_effect=[state(), after]), \
                mock.patch.object(budget, 'playback_flag', return_value=True), \
                mock.patch.object(budget.health, 'cgroup_snapshot', side_effect=OSError('PRIVATE')):
            report = budget.collect(1, Path(tmp))
        self.assertEqual(report['samples_completed'], 0)
        self.assertFalse(report['service_identity_stable'])
        self.assertNotIn('PRIVATE', json.dumps(report))

    def test_cli_reports_missing_service_without_private_exception_and_returns_two(self):
        output = io.StringIO()
        with mock.patch.object(budget, 'collect', side_effect=ValueError('PRIVATE')), contextlib.redirect_stdout(output):
            self.assertEqual(budget.main(['--samples', '1']), 2)
        self.assertNotIn('PRIVATE', output.getvalue())
        self.assertEqual(json.loads(output.getvalue())['code'], 'service_cgroup_unavailable')

    def test_cli_bounds_and_no_success_exit_for_partial_scope(self):
        for n in ('0', '31'):
            with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                budget.main(['--samples', n])
        for status, expected in [('fail', 1), ('incomplete', 2)]:
            with mock.patch.object(budget, 'collect', return_value={'status': status}), \
                    contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(budget.main(['--samples', '1']), expected)


if __name__ == '__main__':
    unittest.main()
