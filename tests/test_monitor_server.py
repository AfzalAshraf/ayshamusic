import contextlib
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock

from scripts import monitor_server as monitor


def state(pid='123'):
    return {'ControlGroup': '/system.slice/aysha-music.service', 'MainPID': pid, 'NRestarts': '0'}


def status():
    return {'available': True, 'transport': 'rust-resolved', 'playing_flag': True,
            'rust_stream_active': True, 'status_request_ms': 5.0}


def sample(current, peak=67_592_192, swap=0):
    return {'memory.current': current, 'memory.peak': peak, 'memory.swap.current': swap,
            'anon': current - 1000, 'file': 500, 'kernel': 500}


class ServerMonitorTests(unittest.TestCase):
    def test_runtime_status_reads_only_fixed_status_and_never_track_or_unknown_mode(self):
        for mode in ['rust-resolved', 'legacy', 'PRIVATE', [], True]:
            with mock.patch.object(monitor.health, 'http_get', return_value=json.dumps({
                    'is_playing': True, 'stream_transport': mode, 'rust_stream_active': False,
                    'current_track': {'title': 'PRIVATE', 'url': 'https://secret.invalid/TOKEN'}}).encode()) as get:
                result = monitor.runtime_status()
            get.assert_called_once_with('/api/status')
            self.assertNotIn('PRIVATE', json.dumps(result))
            self.assertNotIn('TOKEN', json.dumps(result))
            self.assertIn(result['transport'], ['legacy', 'rust-resolved', 'unverified'])
        for body in [b'[]', b'PRIVATE', b'{"is_playing":1}']:
            with mock.patch.object(monitor.health, 'http_get', return_value=body):
                self.assertFalse(monitor.runtime_status()['available'])

    def test_loaded_image_hash_and_owned_listener_identify_server_without_exposing_pid_or_paths(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            process = root / '123'
            (process / 'net').mkdir(parents=True)
            (process / 'fd').mkdir()
            (process / 'comm').write_text('music-server\n')
            fields = ['S'] + ['0'] * 18 + ['345']
            (process / 'stat').write_text('123 (music-server) ' + ' '.join(fields))
            installed = root / 'installed'
            installed.write_bytes(b'fixture executable')
            (process / 'exe').symlink_to(installed)
            (process / 'net/tcp').write_text('header\n0: 00000000:13BA 00000000:0000 0A 0 0 0 1000 0 456\n')
            (process / 'fd/3').symlink_to('socket:[456]')
            result, token = monitor.running_image('123', root, installed)
            self.assertEqual(token, '345')
            self.assertEqual(result['main_process'], 'music-server')
            self.assertTrue(result['loaded_matches_installed'])
            self.assertTrue(result['main_process_listens_on_5050'])
            self.assertTrue(result['image_identity_stable_during_read'])
            self.assertNotIn(str(root), json.dumps(result))
            (process / 'exe').unlink()
            (process / 'exe').write_bytes(b'old loaded image')
            result, _ = monitor.running_image('123', root, installed)
            self.assertFalse(result['loaded_matches_installed'])

    def test_missing_permission_identity_or_invalid_pid_is_unverified_not_a_match(self):
        with tempfile.TemporaryDirectory() as tmp:
            for pid in ['../private', '0', 'PRIVATE', '123']:
                result, _ = monitor.running_image(pid, Path(tmp), Path(tmp) / 'missing')
                self.assertIsNone(result['loaded_matches_installed'])
                self.assertIsNone(result['main_process_listens_on_5050'])
                self.assertNotIn('PRIVATE', json.dumps(result))

    def test_host_memory_is_all_workloads_estimate_and_uses_kib_units(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'meminfo').write_text('MemTotal: 10000 kB\nMemAvailable: 4000 kB\nSwapTotal: 200 kB\nSwapFree: 100 kB\nPRIVATE: 5 kB\n')
            result = monitor.host_memory(root)
            self.assertEqual(result['nonavailable_estimate_bytes'], 6000 * 1024)
            self.assertEqual(result['swap_used_bytes'], 100 * 1024)
            self.assertNotIn('PRIVATE', json.dumps(result))
            (root / 'meminfo').write_text('MemTotal: 1 kB\nMemAvailable: 2 kB\n')
            self.assertIsNone(monitor.host_memory(root))

    def run_window(self, readings, after=None):
        image = {'main_process': 'music-server', 'main_process_listens_on_5050': True,
                 'image_identity_stable_during_read': True, 'loaded_matches_installed': True}
        progress = []
        with tempfile.TemporaryDirectory() as tmp, \
                mock.patch.object(monitor.budget, 'service_state', side_effect=[state(), after or state()]), \
                mock.patch.object(monitor, 'running_image', return_value=(image, None)), \
                mock.patch.object(monitor, 'runtime_status', side_effect=lambda: status()), \
                mock.patch.object(monitor, 'host_memory', return_value={'total_bytes': 1_000_000_000, 'nonavailable_estimate_bytes': 900_000_000}), \
                mock.patch.object(monitor.health, 'cgroup_snapshot', side_effect=readings), \
                mock.patch.object(monitor.time, 'sleep'):
            report = monitor.collect(len(readings), 1, root=Path(tmp), notify=progress.append)
        self.assertEqual(len(progress), len(readings))
        return report

    def test_timeline_and_peak_sample_correlate_helper_burst_without_summing_rss_with_cgroup(self):
        report = self.run_window([
            (sample(6_451_200), {'music-server': (1, 3_000_000)}, False),
            (sample(67_000_000), {'music-server': (1, 4_000_000), 'yt-dlp': (1, 60_000_000), 'PRIVATE': (1, 1000)}, False),
            (sample(7_000_000), {'music-server': (1, 4_000_000)}, False),
        ])
        self.assertEqual(report['status'], 'fail')
        self.assertEqual(report['maximum_current_sample']['index'], 2)
        self.assertEqual(report['sampled_max_current_bytes'], 67_000_000)
        self.assertEqual(report['ram_output']['service_current_MB'], 7)
        self.assertEqual(report['ram_output']['window_max_service_MB'], 67)
        self.assertEqual(report['ram_output']['host_nonavailable_estimate_MB'], 900)
        self.assertIsNone(report['ram_output']['whole_application_total_MB'])
        self.assertEqual(report['process_rss_bucket_maxima']['yt-dlp']['max_count'], 1)
        self.assertNotIn('PRIVATE', json.dumps(report))
        self.assertFalse(report['whole_stack_accepted'])
        self.assertFalse(report['source_revision_certified'])
        for flag in ['starts_playback', 'upstream_requested', 'builds_run']:
            self.assertFalse(report[flag])

    def test_low_current_with_historical_high_peak_is_not_accepted_as_full_playback_budget(self):
        report = self.run_window([(sample(6_451_200), {'music-server': (1, 3_000_000)}, False)])
        self.assertEqual(report['status'], 'incomplete')
        self.assertEqual(report['historical_peak_bytes'], 67_592_192)
        self.assertFalse(report['whole_stack_accepted'])
        self.assertIn('historical_peak', report['code'])

    def test_restart_and_failed_memory_read_stay_incomplete_without_leaking_exception(self):
        report = self.run_window([OSError('PRIVATE')], after=state('456'))
        self.assertFalse(report['service_identity_stable'])
        self.assertEqual(report['samples_completed'], 0)
        self.assertIsNone(report['ram_output']['service_current_MB'])
        self.assertNotIn('PRIVATE', json.dumps(report))
        self.assertEqual(report['runtime_identity_status'], 'unverified')

    def test_complete_listener_scan_can_report_different_main_process_without_paths(self):
        with tempfile.TemporaryDirectory() as tmp:
            process = Path(tmp) / '123'
            (process / 'net').mkdir(parents=True)
            (process / 'fd').mkdir()
            (process / 'net/tcp').write_text('header\n')
            self.assertIs(monitor.listens_on_5050(process), False)
            (process / 'comm').write_text('PRIVATE-PROCESS')
            result, _ = monitor.running_image('123', Path(tmp), Path(tmp) / 'missing')
            self.assertEqual(result['main_process'], 'unexpected_process')
            self.assertNotIn('PRIVATE', json.dumps(result))

    def test_sampling_deadline_stops_before_remaining_requests(self):
        image = {'main_process': 'music-server', 'main_process_listens_on_5050': True,
                 'image_identity_stable_during_read': True}
        with tempfile.TemporaryDirectory() as tmp, \
                mock.patch.object(monitor.budget, 'service_state', return_value=state()), \
                mock.patch.object(monitor, 'running_image', return_value=(image, None)), \
                mock.patch.object(monitor, 'runtime_status', return_value=status()) as api, \
                mock.patch.object(monitor, 'host_memory', return_value=None), \
                mock.patch.object(monitor.health, 'cgroup_snapshot', return_value=(sample(6_451_200), {}, True)), \
                mock.patch.object(monitor.time, 'monotonic', side_effect=[0, 0, 0, 0, 181, 181]):
            report = monitor.collect(2, 1, root=Path(tmp))
        self.assertEqual(report['samples_collected'], 1)
        self.assertFalse(report['window_complete'])
        self.assertEqual(api.call_count, 1)

    def test_progress_reports_no_paths_or_track_metadata(self):
        row = {'elapsed_seconds': 2, 'memory': sample(6_451_200), 'process_rss_buckets': {'yt-dlp': {'count': 1}}, 'runtime': status()}
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            monitor.progress(row)
        self.assertIn('service=6.45 MB', output.getvalue())
        self.assertIn('yt-dlp=1', output.getvalue())

    def test_save_is_private_and_summary_excludes_full_timeline_but_file_keeps_it(self):
        with tempfile.TemporaryDirectory() as tmp, mock.patch.object(monitor.health, 'ROOT', Path(tmp)):
            report = {'timeline': [{'index': 1}], 'status': 'incomplete', 'runtime_identity_status': 'observed'}
            name = monitor.save(report)
            path = Path(tmp) / name
            self.assertEqual(path.stat().st_mode & 0o777, 0o600)
            self.assertEqual(json.loads(path.read_text()), report)
        with mock.patch.object(monitor, 'collect', return_value=report), \
                mock.patch.object(monitor, 'save', return_value='diagnostics/fixture.json'), \
                contextlib.redirect_stdout(io.StringIO()) as output:
            self.assertEqual(monitor.main(['--samples', '1', '--interval', '1']), 2)
        self.assertNotIn('"timeline"', output.getvalue())
        self.assertIn('saved_report', output.getvalue())

    def test_cli_bounds_no_probes_on_invalid_values_and_failure_no_private_exception(self):
        for args in [['--samples', '0'], ['--samples', '61'], ['--interval', '0'], ['--interval', '60']]:
            with mock.patch.object(monitor, 'collect') as collect, contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit):
                    monitor.main(args)
            collect.assert_not_called()
        with mock.patch.object(monitor, 'collect', side_effect=OSError('PRIVATE')), \
                contextlib.redirect_stdout(io.StringIO()) as output:
            self.assertEqual(monitor.main(['--samples', '1']), 3)
        self.assertNotIn('PRIVATE', output.getvalue())


if __name__ == '__main__':
    unittest.main()
