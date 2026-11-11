#!/usr/bin/env bash
# CI-only media generation/decoding and ALSA-null playback; not hardware acceptance.
set -euo pipefail
binary=${1:?Usage: bash tests/rust_audio_smoke.sh PATH_TO_PROTOTYPE}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
ffmpeg -hide_banner -loglevel error -f lavfi -i 'sine=frequency=440:duration=1' \
  -ar 44100 -ac 2 "$work/tone.wav"
ffmpeg -hide_banner -loglevel error -i "$work/tone.wav" -c:a aac "$work/tone.m4a"
ffmpeg -hide_banner -loglevel error -i "$work/tone.wav" -c:a libmp3lame "$work/tone.mp3"
for file in "$work/tone.wav" "$work/tone.m4a" "$work/tone.mp3"; do
  "$binary" --file "$file" --decode-only --seconds 1 | tee "$work/decode.log"
  grep -q 'DECODE_OK samples=' "$work/decode.log"
done
printf 'not audio' > "$work/corrupt.m4a"
if "$binary" --file "$work/corrupt.m4a" --decode-only; then
  echo 'ERROR: corrupt input incorrectly succeeded' >&2
  exit 1
fi
if "$binary" --file "$work/missing.wav" --decode-only; then
  echo 'ERROR: missing input incorrectly succeeded' >&2
  exit 1
fi
truncate -s 67108865 "$work/oversized.wav"
if "$binary" --file "$work/oversized.wav" --decode-only; then
  echo 'ERROR: oversized input incorrectly succeeded' >&2
  exit 1
fi
# A null output exercises the actual playback branch without a physical speaker.
# It neither demonstrates audible sound nor realistic audio-device memory use.
cat > "$work/asound.conf" <<'ALSA'
pcm.!default {
  type null
  hint.description "CI null output"
}
ctl.!default {
  type hw
  card 0
}
ALSA
ALSA_CONFIG_PATH="$work/asound.conf" timeout 10 "$binary" \
  --file "$work/tone.wav" --seconds 2 | tee "$work/play.log"
grep -q 'PLAYING default Linux output' "$work/play.log"
grep -q 'MEMORY playback-finished' "$work/play.log"
echo 'PASS: WAV/M4A/MP3 headless decoding, invalid input rejection and ALSA-null playback'
