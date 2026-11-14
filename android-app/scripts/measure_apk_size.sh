#!/usr/bin/env bash
# Report APK size against the project budget and break it down.
#
# Usage: measure_apk_size.sh [path/to/app-release.apk]
# Env:   MAX_APK_MB  hard budget (default 15), WARN_APK_MB (default 12)
set -euo pipefail

MAX_MB="${MAX_APK_MB:-15}"
WARN_MB="${WARN_APK_MB:-12}"

APP="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APK="${1:-}"
if [[ -z "$APK" ]]; then
  APK="$(find "$APP/app/build/outputs/apk/release" -maxdepth 1 -name '*.apk' 2>/dev/null | head -n1)"
fi
[[ -n "$APK" && -f "$APK" ]] || { echo "error: APK not found" >&2; exit 2; }

BYTES="$(stat -c %s "$APK")"
MB="$(awk -v b="$BYTES" 'BEGIN { printf "%.2f", b/1048576 }')"

echo "APK: $APK"
echo "Size: ${MB} MB (${BYTES} bytes)"

echo
echo "Largest entries:"
unzip -l "$APK" | awk 'NR>3 && NF>=4 && $1 ~ /^[0-9]+$/ && $4 != "" { print $1, $4 }' \
  | sort -rn | head -n 12 | awk '{ printf "  %8.2f MB  %s\n", $1/1048576, $2 }'

echo
echo "Payload groups:"
unzip -l "$APK" | awk '
  NR>3 && NF>=4 && $1 ~ /^[0-9]+$/ && $4 != "" {
    p=$4; b=$1
    if (p ~ /^lib\//) k="native lib"
    else if (p ~ /^classes.*\.dex$/) k="dex"
    else if (p ~ /^res\//) k="resources"
    else if (p ~ /^assets\//) k="assets"
    else if (p ~ /^META-INF\//) k="meta"
    else if (p ~ /^AndroidManifest.xml$/ || p ~ /^resources.arsc$/) k="manifest/arsc"
    else k="other"
    sum[k]+=b
  }
  END { for (k in sum) printf "  %8.2f MB  %s\n", sum[k]/1048576, k }
' | sort -rn

echo
if awk -v mb="$MB" -v max="$MAX_MB" 'BEGIN { exit !(mb > max) }'; then
  echo "FAIL: ${MB} MB exceeds the ${MAX_MB} MB APK budget"
  exit 1
fi
if awk -v mb="$MB" -v warn="$WARN_MB" 'BEGIN { exit !(mb > warn) }'; then
  echo "WARN: ${MB} MB is above the ${WARN_MB} MB warning line"
  exit 0
fi
echo "PASS: ${MB} MB within the ${MAX_MB} MB budget"
