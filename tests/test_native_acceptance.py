import contextlib
import io
import json
import unittest
from unittest import mock
from scripts import check_native_acceptance as gate


class NativeAcceptanceTests(unittest.TestCase):
    def test_host_v5_1_failure_is_blocked_despite_build_and_low_rss(self):
        evidence = json.loads(gate.EVIDENCE.read_text())
        result = gate.assess(evidence)
        self.assertEqual(evidence['latest_native']['debug_build_seconds'], 3.16)
        self.assertEqual(evidence['latest_native']['peak_rss_kib'], 10632)
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
        self.assertEqual(result['next'], 'offline_reproduce_ignored_watch_source_before_native_replacement')

    def test_watch_diagnostic_pass_is_not_native_media_acceptance(self):
        result = gate.assess({'latest_native': {'status':'diagnostic_pass', 'watch_parser_only':True,
                            'player_requested':False,'media_requested':False,'external_tools':False}})
        self.assertEqual(result['status'], 'blocked')
        self.assertFalse(result['native_prefix_gate_passed'])
        self.assertFalse(result['whole_stack_ready'])

    def test_host_syntax_diagnostic_is_recorded_separately_without_media_claim(self):
        evidence = json.loads(gate.EVIDENCE.read_text())
        diagnostic = evidence['latest_diagnostic']
        self.assertEqual(diagnostic['code'], 'watch_visitor_missing')
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

    def test_v5_1_consumed_approval_and_fixture_success_do_not_replace_live_acquisition_evidence(self):
        evidence = json.loads(gate.EVIDENCE.read_text())
        result = gate.assess(evidence)
        self.assertEqual(evidence['approved_experiment']['candidate'], 'aysha-direct-v5.1-static-wrapper')
        self.assertEqual(result['next'], 'offline_reproduce_ignored_watch_source_before_native_replacement')
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

    def test_consumed_trial_cannot_be_recommended_even_with_stale_awaiting_status(self):
        evidence = json.loads(gate.EVIDENCE.read_text())
        evidence.pop('review_required')
        evidence['approved_experiment']['status'] = 'awaiting_single_host_trial'
        self.assertTrue(evidence['approved_experiment']['host_result_recorded'])
        self.assertEqual(gate.assess(evidence)['next'],
                         'review_and_approve_next_probe_scope_before_any_new_native_trial')

    def test_offline_review_takes_precedence_over_stale_native_or_watch_trial(self):
        evidence = json.loads(gate.EVIDENCE.read_text())
        evidence['approved_experiment'].update(status='awaiting_single_host_trial',host_result_recorded=False)
        evidence['pending_diagnostic'].update(status='awaiting_one_watch_only_request',
                                             candidate='aysha-direct-v4.4-metadata-guard')
        evidence['review_required']['status'] = 'offline_only'
        evidence['review_required']['next'] = 'offline_source_coverage_review_no_live_retry'
        self.assertEqual(gate.assess(evidence)['next'], 'offline_source_coverage_review_no_live_retry')

    def test_latest_observation_only_proves_absence_in_recognized_sources(self):
        evidence = json.loads(gate.EVIDENCE.read_text())
        native = evidence['latest_native']
        self.assertFalse(native['player_requested'])
        self.assertFalse(native['media_requested'])
        observed = native['watch_observation']
        self.assertEqual(observed['config_objects'], 1)
        self.assertEqual(observed['initial_players'], 1)
        self.assertEqual(observed['wrapped_config_blocks'], 0)
        self.assertEqual(observed['ignored_script_tails'], 35)
        self.assertFalse(any(observed[key] for key in (
            'config_visitor_present', 'context_visitor_present', 'player_visitor_present')))
        self.assertEqual(gate.assess(evidence)['status'], 'blocked')

    def test_confirmed_offline_guard_is_historical_and_new_request_is_only_watch_diagnostic(self):
        evidence = json.loads(gate.EVIDENCE.read_text())
        review = evidence['latest_offline_host_review']
        self.assertFalse(review['network_executed'])
        self.assertFalse(review['native_executed'])
        self.assertEqual(review['source_revision'], 'efde115')
        self.assertTrue(evidence['prepared_diagnostic']['approval_recorded'])
        result = gate.assess(evidence)
        self.assertEqual(result['status'], 'blocked')
        self.assertEqual(result['next'], 'offline_reproduce_ignored_watch_source_before_native_replacement')

    def test_host_coverage_fixture_pass_is_not_live_acquisition_even_after_separate_user_approval(self):
        evidence = json.loads(gate.EVIDENCE.read_text())
        diagnostic = evidence['prepared_diagnostic']
        host = diagnostic['host_fixture_verification']
        self.assertEqual(host['source_revision'], 'f207206')
        self.assertEqual(host['tests_passed'], 7)
        self.assertEqual(host['tests_filtered_out'], 55)
        self.assertFalse(host['provider_requests'])
        self.assertFalse(host['normal_cli_binary_build_verified'])
        self.assertTrue(diagnostic['approval_recorded'])
        result = gate.assess(evidence)
        self.assertEqual(result['next'], 'offline_reproduce_ignored_watch_source_before_native_replacement')
        self.assertFalse(result['native_prefix_gate_passed'])
        self.assertFalse(result['memory_budget_accepted'])

    def test_coverage_watch_action_requires_separate_live_approval_and_unconsumed_request(self):
        for change in ({'approval_recorded': False}, {'host_result_recorded': True},
                       {'status': 'completed_failed_stop_live_retries'}):
            evidence = json.loads(gate.EVIDENCE.read_text())
            evidence['prepared_diagnostic'].update(change)
            result = gate.assess(evidence)
            self.assertNotEqual(result['next'], 'one_approved_watch_only_coverage_inventory_no_player_or_media')
            self.assertFalse(result['native_prefix_gate_passed'])
            self.assertFalse(result['whole_stack_ready'])

    def test_completed_coverage_inventory_does_not_authorize_another_request_or_install(self):
        evidence = json.loads(gate.EVIDENCE.read_text())
        self.assertTrue(evidence['prepared_diagnostic']['host_result_recorded'])
        observed = evidence['latest_diagnostic']['watch_observation']['coverage']
        self.assertEqual(observed['ignored_tails']['with_raw_visitor_name'], 2)
        self.assertEqual(observed['parsed_config_objects']['with_raw_visitor_name'], 0)
        self.assertEqual(observed['skipped_attribute_bodies']['with_raw_visitor_name'], 0)
        result = gate.assess(evidence)
        self.assertFalse(result['native_replacement_allowed'])
        self.assertEqual(result['deployment_action'], 'keep_working_service_do_not_install_prototype')
        self.assertEqual(result['next'], 'offline_reproduce_ignored_watch_source_before_native_replacement')

    def test_working_service_budget_is_a_recomputed_failure_not_native_feasibility(self):
        evidence = json.loads(gate.EVIDENCE.read_text())
        reports = evidence['requested_budget_check']['host_observations']
        self.assertEqual(len(reports), 2)
        self.assertEqual(reports[1]['playing_flag_start'], True)
        self.assertEqual(reports[1]['playing_flag_end'], True)
        self.assertEqual(reports[1]['historical_peak_bytes'], 218480640)
        result = gate.assess(evidence)
        measured = result['current_service_budget']
        self.assertEqual(measured['max_sampled_current_bytes'], 101171200)
        self.assertEqual(measured['observations_recorded'], 2)
        self.assertTrue(measured['exceeds_15_MB'])
        self.assertTrue(measured['exceeds_15_MiB'])
        self.assertEqual(measured['status'], 'fail')
        self.assertFalse(result['memory_budget_accepted'])

    def test_low_probe_rss_or_claimed_pass_cannot_erase_existing_service_failure(self):
        evidence = json.loads(gate.EVIDENCE.read_text())
        for report in evidence['requested_budget_check']['host_observations']:
            report.update(status='pass', whole_stack_accepted=True)
        evidence['latest_diagnostic'].update(status='diagnostic_pass', peak_rss_kib=1)
        result = gate.assess(evidence)
        self.assertEqual(result['current_service_budget']['status'], 'fail')
        self.assertFalse(result['native_replacement_allowed'])

    def test_invalid_or_low_service_budget_observation_never_becomes_acceptance(self):
        for reports in (None, {}, [], [{'sampled_max_current_bytes':1}],
                        [{'schema':1,'candidate':'aysha-service-playback-budget-v1',
                          'measurement_scope':'aysha_music_service_cgroup_and_descendants_only',
                          'samples_completed':20,'sampled_max_current_bytes':False}]):
            measured = gate.observed_service_budget({'requested_budget_check': {'host_observations':reports}})
            self.assertEqual(measured['status'], 'unverified')
        evidence = json.loads(gate.EVIDENCE.read_text())
        for report in evidence['requested_budget_check']['host_observations']:
            report['sampled_max_current_bytes'] = 1
        measured = gate.observed_service_budget(evidence)
        self.assertEqual(measured['status'], 'unverified')
        self.assertFalse(measured['whole_stack_accepted'])

    def test_current_50mb_target_is_strict_and_keeps_historical_evidence_intact(self):
        evidence = json.loads(gate.EVIDENCE.read_text())
        before = json.dumps(evidence, sort_keys=True)
        report = gate.observed_service_budget(evidence)
        self.assertEqual(report['current_target'], 'strictly_below_50_MB_total')
        self.assertTrue(report['at_or_above_50_MB'])
        self.assertTrue(report['at_or_above_50_MiB'])
        self.assertEqual(json.dumps(evidence, sort_keys=True), before)
        for amount, decimal, binary in [(49_999_999, False, False), (50_000_000, True, False),
                                        (50 * 1048576 - 1, True, False), (50 * 1048576, True, True)]:
            for observation in evidence['requested_budget_check']['host_observations']:
                observation['sampled_max_current_bytes'] = amount
            report = gate.observed_service_budget(evidence)
            self.assertEqual(report['at_or_above_50_MB'], decimal)
            self.assertEqual(report['at_or_above_50_MiB'], binary)
            self.assertEqual(report['status'], 'fail' if decimal else 'unverified')
            self.assertFalse(report['whole_stack_accepted'])

    def test_old_and_new_budget_schemas_are_reassessed_at_current_target(self):
        for candidate in ['aysha-service-playback-budget-v1', 'aysha-service-playback-budget-v2-30mb',
                          'aysha-service-playback-budget-v3-50mb', 'unknown']:
            report = gate.observed_service_budget({'requested_budget_check': {'host_observations': [{
                'schema': 1, 'candidate': candidate,
                'measurement_scope': 'aysha_music_service_cgroup_and_descendants_only',
                'samples_completed': 20, 'sampled_max_current_bytes': 50_000_000,
            }]}})
            self.assertEqual(report['status'], 'unverified' if candidate == 'unknown' else 'fail')
            self.assertFalse(report['whole_stack_accepted'])

    def test_missing_or_invalid_evidence_fails_closed_without_exception_text(self):
        for error in [OSError('private-path'), ValueError('private-data')]:
            output = io.StringIO()
            with mock.patch.object(gate.Path, 'read_text', side_effect=error), contextlib.redirect_stdout(output):
                self.assertEqual(gate.main(), 1)
            self.assertEqual(json.loads(output.getvalue())['code'], 'evidence_missing_or_invalid')
            self.assertNotIn('private', output.getvalue())


if __name__ == '__main__':
    unittest.main()
