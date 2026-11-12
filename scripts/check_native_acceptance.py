#!/usr/bin/env python3
"""Offline native-acquisition gate over committed host evidence; never runs a probe.

This is an evidence review, not a live environment/whole-app readiness checker.
A build or a low-RSS rejection must never count as native source-access acceptance.
"""
import json
from pathlib import Path

EVIDENCE = Path(__file__).resolve().parents[1] / "docs/research/NATIVE_ACQUISITION_HOST.json"
WATCH_ACTIONS = {
    "aysha-direct-v4.2-config-diagnostic": "one_watch_only_config_diagnostic_no_player_or_media",
    "aysha-direct-v4.3-syntax-diagnostic": "one_watch_only_syntax_subtype_diagnostic_no_player_or_media",
    "aysha-direct-v4.4-metadata-guard": "one_guarded_metadata_watch_check_no_player_or_media",
}


def assess(evidence):
    native = evidence.get("latest_native", {})
    prefix_pass = (native.get("status") == "pass" and native.get("bytes") == 65536
                   and native.get("external_tools") is False and not native.get("code"))
    approved = evidence.get("approved_experiment", {})
    awaiting_trial = (isinstance(approved, dict) and approved.get("approval_recorded") is True
                      and approved.get("candidate") in ("aysha-direct-v4-watch-bootstrap", "aysha-direct-v4.1-watch-literals")
                      and approved.get("status") == "awaiting_single_host_trial")
    diagnostic = evidence.get("pending_diagnostic", {})
    watch_action = (WATCH_ACTIONS.get(diagnostic.get("candidate")) if isinstance(diagnostic, dict)
                    and diagnostic.get("status") == "awaiting_one_watch_only_request" else None)
    return {
        "status": "pass" if prefix_pass else "blocked",
        "code": "historical_native_prefix_pass" if prefix_pass else "native_acquisition_not_demonstrated",
        "network_executed": False,
        "native_executed": False,
        "native_prefix_gate_passed": prefix_pass,
        "whole_stack_ready": False,
        "memory_budget_accepted": False,
        "stop_identical_trials": evidence.get("stop_identical_native_trials") is True,
        "next": ("separate_decoder_controls_and_whole_budget_acceptance" if prefix_pass else
                 watch_action if watch_action else
                 ("one_parser_corrected_v4_1_trial_after_offline_verification" if approved.get("candidate") == "aysha-direct-v4.1-watch-literals" else
                  "one_approved_v4_host_trial_after_offline_verification") if awaiting_trial else
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
