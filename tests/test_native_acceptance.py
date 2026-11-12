import contextlib
import io
import json
import unittest
from unittest import mock
from scripts import check_native_acceptance as gate


class NativeAcceptanceTests(unittest.TestCase):
    def test_host_v5_failure_is_blocked_despite_build_and_low_rss(self):
        evidence = json.loads(gate.EVIDENCE.read_text())
        result = gate.assess(evidence)
        self.assertEqual(evidence['latest_native']['debug_build_seconds'], 1.64)
        self.assertEqual(evidence['latest_native']['peak_rss_kib'], 10884)
        self.assertEqual(result['status'], 'blocked')
        self.assertFalse(result['native_prefix_gate_passed'])
        self.assertFalse(result['memory_budget_accepted'])
        self.assertFalse(result['whole_stack_ready'])
        self.assertTrue(result['stop_identical_trials'])

    def test_baseline_pass_and_production_ready_claim_do_not_override_native_failure(self):
        evidence = {'latest_native': {'status':'fail', 'production_ready':True},
                    'baseline_trace': {'status':'pass', 'bytes':65536}}
        self.assertEqual(gate.assess(evidence)['status'], 'blocked')

    def test_success_requires_full_prefix_and_explicit_no_helpers(self):
        for native in [{'status':'pass'}, {'status':'pass', 'bytes':65536},
                       {'status':'pass', 'bytes':65536, 'external_tools':True},
                       {'status':'pass', 'bytes':65536, 'external_tools':False, 'code':'player_bot_check'},
                       {'status':'pass', 'bytes':8, 'external_tools':False}]:
            self.assertEqual(gate.assess({'latest_native':native})['status'], 'blocked')

    def test_historical_prefix_success_never_certifies_memory_or_production(self):
        result = gate.assess({'latest_native': {'status':'pass', 'bytes':65536, 'external_tools':False}})
        self.assertTrue(result['native_prefix_gate_passed'])
        self.assertFalse(result['whole_stack_ready'])
        self.assertFalse(result['memory_budget_accepted'])
        self.assertFalse(result['network_executed'])
        self.assertFalse(result['native_executed'])

    def test_approval_changes_next_action_but_never_acceptance(self):
        evidence = {'latest_native': {'status':'fail','code':'player_bot_check'},
                    'approved_experiment': {'candidate':'aysha-direct-v4-watch-bootstrap',
                                            'approval_recorded':True,'status':'awaiting_single_host_trial'}}
        result = gate.assess(evidence)
        self.assertEqual(result['status'], 'blocked')
        self.assertEqual(result['next'], 'one_approved_v4_host_trial_after_offline_verification')
        self.assertFalse(result['whole_stack_ready'])
        self.assertFalse(result['memory_budget_accepted'])

    def test_v5_missing_visitor_never_counts_as_player_or_media_attempt_success(self):
        evidence = json.loads(gate.EVIDENCE.read_text())
        self.assertEqual(evidence['latest_native']['stage'], 'watch')
        self.assertEqual(evidence['latest_native']['code'], 'watch_visitor_missing')
        result = gate.assess(evidence)
        self.assertFalse(result['native_prefix_gate_passed'])
        self.assertEqual(result['next'], 'one_bounded_v5_1_wrapper_trial_with_value_free_observations')

    def test_watch_diagnostic_pass_is_not_native_media_acceptance(self):
        result = gate.assess({'latest_native': {'status':'diagnostic_pass', 'watch_parser_only':True,
                            'player_requested':False,'media_requested':False,'external_tools':False}})
        self.assertEqual(result['status'], 'blocked')
        self.assertFalse(result['native_prefix_gate_passed'])
        self.assertFalse(result['whole_stack_ready'])

    def test_host_syntax_diagnostic_is_recorded_separately_without_media_claim(self):
        evidence = json.loads(gate.EVIDENCE.read_text())
        diagnostic = evidence['latest_diagnostic']
        self.assertEqual(diagnostic['code'], 'watch_config_js_single_quote')
        self.assertEqual(diagnostic['build_profile'], 'debug')
        self.assertFalse(diagnostic['player_requested'])
        self.assertFalse(diagnostic['media_requested'])
        self.assertEqual(gate.assess(evidence)['status'], 'blocked')

    def test_guarded_metadata_watch_pass_still_cannot_count_as_media(self):
        report = {'status':'diagnostic_pass','metadata_nonfinite_ignored':True,'external_tools':False,
                  'watch_parser_only':True,'player_requested':False,'media_requested':False}
        result = gate.assess({'latest_native':report})
        self.assertFalse(result['native_prefix_gate_passed'])
        self.assertFalse(result['memory_budget_accepted'])
        self.assertFalse(result['whole_stack_ready'])

    def test_v5_1_approval_and_fixture_success_do_not_replace_live_acquisition_evidence(self):
        evidence = json.loads(gate.EVIDENCE.read_text())
        result = gate.assess(evidence)
        self.assertEqual(evidence['approved_experiment']['candidate'], 'aysha-direct-v5.1-static-wrapper')
        self.assertEqual(result['next'], 'one_bounded_v5_1_wrapper_trial_with_value_free_observations')
        self.assertEqual(result['status'], 'blocked')
        self.assertFalse(result['memory_budget_accepted'])
        self.assertFalse(result['whole_stack_ready'])

    def test_observations_and_watch_only_success_never_override_missing_media(self):
        for field in [{'watch_parser_only':True}, {'stage':'watch'}, {'player_requested':False},
                      {'media_requested':False}]:
            report = {'status':'pass','bytes':65536,'external_tools':False,
                      'watch_observation': {'config_objects':1,'wrapped_config_blocks':1,
                                            'initial_players':1,'config_visitor_present':True}, **field}
            result = gate.assess({'latest_native':report})
            self.assertFalse(result['native_prefix_gate_passed'])
            self.assertFalse(result['whole_stack_ready'])
            self.assertFalse(result['memory_budget_accepted'])

    def test_missing_or_invalid_evidence_fails_closed_without_exception_text(self):
        for error in [OSError('private-path'), ValueError('private-data')]:
            output = io.StringIO()
            with mock.patch.object(gate.Path, 'read_text', side_effect=error), contextlib.redirect_stdout(output):
                self.assertEqual(gate.main(), 1)
            self.assertEqual(json.loads(output.getvalue())['code'], 'evidence_missing_or_invalid')
            self.assertNotIn('private', output.getvalue())


if __name__ == '__main__':
    unittest.main()
