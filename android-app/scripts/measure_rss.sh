#!/usr/bin/env bash
# Measure on-device memory for the installed app against the RSS budget.
#
# Usage:
#   android-app/scripts/measure_rss.sh [package] [--watch SECONDS]
#
# Prereqs: adb with exactly one device/emulator connected, app installed.
# Env: MAX_TOTAL_MB  hard budget in MB (default 35 for cold idle, see notes)
#
# Notes:
#   - "TOTAL" from dumpsys meminfo is proportional (PSS), which is what the
#     OS reports per process; RSS sums shared pages multiple times.
#   - The WebView runs in the app's process here, so TOTAL covers
#     Kotlin + WebView + the Rust server started via JNI.
#   - Measure three states: cold idle, after opening the player page, and
#     while playing, then compare against the 25-35 MB target.
set -euo pipefail

PKG="${1:-com.afzal.ayshamusic}"
WATCH=0
if [[ "${2:-}" == "--watch" ]]; then WATCH="${3:-30}"; fi
MAX_MB="${MAX_TOTAL_MB:-35}"

adb get-state >/dev/null 2>&1 || { echo "error: no adb device" >&2; exit 2; }

sample() {
  local label="$1"
  local pid
  pid="$(adb shell pidof "$PKG" 2>/dev/null | tr -d '\r' | awk '{ print $1 }')"
  if [[ -z "$pid" ]]; then
    echo "$label: app not running"
    return
  fi
  local total_kb
  total_kb="$(adb shell dumpsys meminfo "$PKG" | tr -d '\r' \
    | awk '/^ *TOTAL/ { print $2; exit }')"
  if [[ -z "$total_kb" ]]; then
    echo "$label: could not read TOTAL from dumpsys"
    return
  fi
  awk -v kb="$total_kb" -v l="$label" -v pid="$pid" \
    'BEGIN { printf "%s (pid %s): %.1f MB\n", l, pid, kb/1024 }'
}

if [[ "$WATCH" != "0" ]]; then
  END=$((SECONDS + WATCH))
  while [[ $SECONDS -lt $END ]]; do
    sample "t+${SECONDS}s"
    sleep 5
  done
  exit 0
fi

sample "current"
TOTAL_KB="$(adb shell dumpsys meminfo "$PKG" | tr -d '\r' | awk '/^ *TOTAL/ { print $2; exit }')"
if [[ -n "${TOTAL_KB:-}" ]]; then
  awk -v kb="$TOTAL_KB" -v max="$MAX_MB" 'BEGIN {
    mb = kb/1024
    if (mb > max) { printf "FAIL: %.1f MB exceeds the %s MB budget\n", mb, max; exit 1 }
    printf "PASS: %.1f MB within the %s MB budget\n", mb, max
  }'
fi
