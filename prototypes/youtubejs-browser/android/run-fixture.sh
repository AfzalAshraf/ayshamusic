#!/usr/bin/env bash
# CI-only diagnostic/build/test; never contacts a music provider.
set -euo pipefail
cd "$(dirname "$0")/../../../android-app"
log="${RUNNER_TEMP:-/tmp}/aysha-webview-gradle.log"
if ./gradlew --no-daemon -PayshaEmulatorTests=true \
  -Pandroid.testInstrumentationRunnerArguments.class=com.afzal.ayshamusic.BrowserExtractorFixtureTest \
  connectedAndroidAudioDebugAndroidTest > "$log" 2>&1; then
  cat "$log"
else
  cat "$log"
  FIXTURE_LOG="$log" python3 - <<'PY'
import os
text=open(os.environ['FIXTURE_LOG']).read()[-12000:]
print('::error title=Offline WebView build/test::'+text.replace('%','%25').replace('\n','%0A').replace('\r','%0D'))
PY
  exit 1
fi
