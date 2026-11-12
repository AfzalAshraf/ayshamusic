import contextlib
import io
import json
import unittest
from unittest import mock
from scripts import check_native_acceptance as gate


class NativeAcceptanceTests(unittest.TestCase):
    def test_host_v3_failure_is_blocked_despite_build_and_low_rss(self):
        evidence = json.loads(gate.EVIDENCE.read_text())
        result = gate.assess(evidence)
        self.assertEqual(evidence['latest_native']['tests_passed'], 11)
        self.assertEqual(evidence['latest_native']['peak_rss_kib'], 4404)
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

    def test_missing_or_invalid_evidence_fails_closed_without_exception_text(self):
        for error in [OSError('private-path'), ValueError('private-data')]:
            output = io.StringIO()
            with mock.patch.object(gate.Path, 'read_text', side_effect=error), contextlib.redirect_stdout(output):
                self.assertEqual(gate.main(), 1)
            self.assertEqual(json.loads(output.getvalue())['code'], 'evidence_missing_or_invalid')
            self.assertNotIn('private', output.getvalue())


if __name__ == '__main__':
    unittest.main()
