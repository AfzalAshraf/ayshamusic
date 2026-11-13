#!/usr/bin/env python3
"""Offline native-acquisition gate over committed host evidence; never runs a probe.

This is an evidence review, not a live environment/whole-app readiness checker.
A build or a low-RSS rejection must never count as native source-access acceptance.
"""
import json
from pathlib import Path

EVIDENCE = Path(__file__).resolve().parents[1] / "docs/research/NATIVE_ACQUISITION_HOST.json"
WATCH_ACTIONS = {
    "aysha-direct-v5.2-coverage-only": "one_approved_watch_only_coverage_inventory_no_player_or_media",
    "aysha-direct-v4.2-config-diagnostic": "one_watch_only_config_diagnostic_no_player_or_media",
    "aysha-direct-v4.3-syntax-diagnostic": "one_watch_only_syntax_subtype_diagnostic_no_player_or_media",
    "aysha-direct-v4.4-metadata-guard": "one_guarded_metadata_watch_check_no_player_or_media",
}

NATIVE_ACTIONS = {
    "aysha-direct-v4-watch-bootstrap": "one_approved_v4_host_trial_after_offline_verification",
    "aysha-direct-v4.1-watch-literals": "one_parser_corrected_v4_1_trial_after_offline_verification",
    "aysha-direct-v5-data-literals": "one_bounded_v5_native_trial_after_offline_verification",
    "aysha-direct-v5.1-static-wrapper": "one_bounded_v5_1_wrapper_trial_with_value_free_observations",
}


def observed_service_budget(evidence):
    """Recompute a lower-bound failure; never accept an old service as a new native stack."""
    budget = evidence.get("requested_budget_check", {})
    reports = budget.get("host_observations", []) if isinstance(budget, dict) else []
    if not isinstance(reports, list):
        reports = []
    valid = [r for r in reports if isinstance(r, dict)
             and r.get("schema") == 1 and r.get("candidate") in {"aysha-service-playback-budget-v1", "aysha-service-playback-budget-v2-30mb"}
             and r.get("measurement_scope") == "aysha_music_service_cgroup_and_descendants_only"
             and type(r.get("sampled_max_current_bytes")) is int and r["sampled_max_current_bytes"] >= 0
             and type(r.get("samples_completed")) is int and 1 <= r["samples_completed"] <= 30]
    if not valid:
        return {"status": "unverified", "code": "no_valid_service_budget_observation",
                "whole_stack_accepted": False}
    current = max(r["sampled_max_current_bytes"] for r in valid)
    decimal = current >= 30_000_000
    binary = current >= 30 * 1048576
    return {
        "status": "fail" if decimal else "unverified",
        "code": ("current_service_alone_at_or_above_both_30_unit_limits" if binary else
                 "current_service_at_or_above_decimal_30_MB_limit" if decimal else
                 "service_only_under_upper_limit_not_whole_stack_acceptance"),
        "observations_recorded": len(valid), "max_sampled_current_bytes": current,
        "current_target": "strictly_below_30_MB_total",
        "target_upper_bytes": {"30_MB": 30_000_000, "30_MiB": 30 * 1048576},
        "at_or_above_30_MB": decimal, "at_or_above_30_MiB": binary,
        "max_sampled_current_MB": current / 1_000_000,
        "max_sampled_current_MiB": current / 1048576,
        "exceeds_15_MB": current > 15_000_000, "exceeds_15_MiB": current > 15 * 1048576,
        "historical_15_fields": "Previous target comparison only; observations are not rewritten.",
        "whole_stack_accepted": False,
        "scope": "Existing service cgroup lower bound, not future-native feasibility or candidate memory. RSS buckets not summed; historical peak not attributed to these windows.",
    }


def assess(evidence):
    native = evidence.get("latest_native", {})
    prefix_pass = (native.get("status") == "pass" and native.get("bytes") == 65536
                   and native.get("external_tools") is False and not native.get("code")
                   and native.get("watch_parser_only") is not True and native.get("stage") != "watch"
                   and native.get("player_requested") is not False and native.get("media_requested") is not False)
    approved = evidence.get("approved_experiment", {})
    native_action = (NATIVE_ACTIONS.get(approved.get("candidate")) if isinstance(approved, dict)
                     and approved.get("approval_recorded") is True
                     and approved.get("status") == "awaiting_single_host_trial"
                     and approved.get("host_result_recorded") is not True else None)
    diagnostic = evidence.get("pending_diagnostic", {})
    watch_action = (WATCH_ACTIONS.get(diagnostic.get("candidate")) if isinstance(diagnostic, dict)
                    and diagnostic.get("status") == "awaiting_one_watch_only_request" else None)
    review = evidence.get("review_required", {})
    if isinstance(diagnostic, dict) and diagnostic.get("candidate") == "aysha-direct-v5.2-coverage-only":
        prepared = evidence.get("prepared_diagnostic", {})
        authorized = (isinstance(prepared, dict) and isinstance(review, dict)
                      and prepared.get("candidate") == diagnostic.get("candidate")
                      and prepared.get("approval_recorded") is True
                      and prepared.get("status") == "awaiting_one_approved_watch_only_request"
                      and prepared.get("host_result_recorded") is not True
                      and review.get("status") == "approved_watch_only_diagnostic")
        if not authorized:
            watch_action = None
    offline_review = (("offline_reproduce_ignored_watch_source_before_native_replacement"
                       if review.get("next") == "offline_reproduce_ignored_watch_source_before_native_replacement"
                       else "offline_source_coverage_review_no_live_retry")
                      if isinstance(review, dict) and review.get("status") == "offline_only" else None)
    return {
        "status": "pass" if prefix_pass else "blocked",
        "code": "historical_native_prefix_pass" if prefix_pass else "native_acquisition_not_demonstrated",
        "network_executed": False,
        "native_executed": False,
        "native_prefix_gate_passed": prefix_pass,
        "whole_stack_ready": False,
        "memory_budget_accepted": False,
        "native_replacement_allowed": False,
        "deployment_action": "keep_working_service_do_not_install_prototype",
        "current_service_budget": observed_service_budget(evidence),
        "stop_identical_trials": evidence.get("stop_identical_native_trials") is True,
        "next": ("separate_decoder_controls_and_whole_budget_acceptance" if prefix_pass else
                 offline_review if offline_review else
                 watch_action if watch_action else
                 native_action if native_action else
                 "review_and_approve_next_probe_scope_before_any_new_native_trial"),
        "limits": "Committed historical evidence only. No media validation occurs here. A prefix pass alone cannot certify decoding, audible output, resource budgets or production readiness.",
    }


def main():
    try:
        evidence = json.loads(EVIDENCE.read_text())
        if evidence.get("schema") != 1 or not isinstance(evidence.get("latest_native"), dict):
            raise ValueError("invalid schema")
        report = assess(evidence)
    except (OSError, ValueError, TypeError, AttributeError):
        report = {"status": "blocked", "code": "evidence_missing_or_invalid",
                  "network_executed": False, "native_executed": False,
                  "whole_stack_ready": False, "memory_budget_accepted": False}
    print(json.dumps(report, indent=2))
    return 0 if report["status"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
