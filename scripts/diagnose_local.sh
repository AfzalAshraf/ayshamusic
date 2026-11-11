#!/usr/bin/env bash
# Read-only local Linux diagnostics. No pairing, connection or service changes.
set -u
# Do not let systemctl invoke a pager or leave an unresponsive probe running.
export SYSTEMD_PAGER=cat SYSTEMD_COLORS=0 PAGER=cat
run() {
  printf '\n--- %s ---\n' "$*"
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'Missing command: %s\n' "$1"
    return
  fi
  timeout --kill-after=2s 15s "$@" 2>&1 || printf '(command failed or timed out; continuing)\n'
}
if ! command -v timeout >/dev/null 2>&1; then
  printf 'Missing GNU timeout; cannot run bounded diagnostics safely.\n' >&2
  exit 1
fi
run id
printf '\n--- Downloader and JavaScript runtime paths ---\n'
for tool in yt-dlp node deno qjs; do
  command -v "$tool" || printf '%s is not on this shell PATH\n' "$tool"
done
run yt-dlp --version
run node --version
run deno --version
run uname -m
if [[ -r /etc/os-release ]]; then
  printf '\n--- Linux distribution ---\n'
  grep -E '^(NAME|VERSION_ID|ID)=' /etc/os-release
fi
run systemctl --no-pager show aysha-music -p User -p ExecStart -p WorkingDirectory -p MainPID
run systemctl --no-pager is-active bluetooth
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
