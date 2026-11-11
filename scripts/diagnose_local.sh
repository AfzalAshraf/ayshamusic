#!/usr/bin/env bash
# Read-only local Linux diagnostics. No pairing, connection or service changes.
set -u
run() {
  printf '\n--- %s ---\n' "$*"
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'Missing command: %s\n' "$1"
    return
  fi
  timeout 15 "$@" 2>&1 || printf '(command failed or timed out)\n'
}
run id
run systemctl show aysha-music -p User -p ExecStart -p WorkingDirectory -p MainPID
run systemctl is-active bluetooth
run bluetoothctl show
run bluetoothctl info 04:CB:88:92:43:60
run pactl info
run pactl list sinks short
run pactl list cards short
run ps -C music-server,yt-dlp,mpv -o pid,ppid,rss,comm
script="${MUSIC_BT_CONNECT_SCRIPT:-/home/afine/connect_jbl.sh}"
printf '\n--- Connection script: %s ---\n' "$script"
if [[ -r "$script" ]]; then
  bash -n "$script" && printf 'Script exists and shell syntax is valid (contents not executed).\n'
else
  printf 'Script is missing or unreadable by this user.\n'
fi
printf '\nRun as the same user as music-server. This shell test cannot prove service-context audio access.\n'
printf 'Review output for private details before sharing.\n'
