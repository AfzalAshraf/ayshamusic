#!/usr/bin/env bash
# Explicitly installs the confirmed afine host's Rust service, without cloud components.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
if [[ "$(id -un)" != afine || "$HOME" != /home/afine || "$(id -u)" != 1000 ]]; then
  printf 'This installer targets the confirmed afine UID 1000 host. Do not run as root.\n' >&2
  exit 1
fi
for tool in cargo sudo systemctl loginctl pgrep curl mktemp; do
  command -v "$tool" >/dev/null || { printf 'Missing tool: %s\n' "$tool" >&2; exit 1; }
done
[[ -x /home/afine/.local/bin/yt-dlp && -x /usr/bin/node ]] || {
  printf 'Expected yt-dlp or Node executable missing; no packages were changed.\n' >&2; exit 1;
}
/usr/bin/node -e 'if (Number(process.versions.node.split(".")[0]) < 22) process.exit(1)'
# Unreviewed drop-ins could override ExecStart, environment or permissions.
dropins=$(systemctl show aysha-music -p DropInPaths --value)
[[ -z "$dropins" ]] || { printf 'Review existing service drop-ins first: %s\n' "$dropins" >&2; exit 1; }
service_pid=$(systemctl show aysha-music -p MainPID --value)
while read -r pid; do
  if [[ -n "$pid" && "$pid" != "$service_pid" ]]; then
    printf 'Foreground music-server PID %s is running. Stop it with Ctrl-C, then retry.\n' "$pid" >&2
    exit 1
  fi
done < <(pgrep -u 1000 -x music-server || true)

# Never stop or replace the working service after a failed build.
cargo test
cargo build --release
sudo -v
unit=/etc/systemd/system/aysha-music.service
stamp=$(date +%Y%m%d-%H%M%S)
if [[ -f "$unit" ]]; then
  sudo cp -a "$unit" "$unit.backup-$stamp"
  printf 'Saved previous service unit: %s.backup-%s\n' "$unit" "$stamp"
fi
dest=/home/afine/music-server/target/release/music-server
mkdir -p "$(dirname "$dest")"
temporary=$(mktemp "$(dirname "$dest")/.music-server.XXXXXX")
trap 'rm -f "$temporary"' EXIT
install -m 755 target/release/music-server "$temporary"
sudo systemctl stop aysha-music
if [[ -f "$dest" ]]; then cp -p "$dest" "$dest.backup-$stamp"; fi
mv -f "$temporary" "$dest"
sudo install -m 644 deploy/aysha-music.service "$unit"
sudo systemctl daemon-reload
# Keep the user's PipeWire manager available after SSH logout and across boot.
sudo loginctl enable-linger afine
sudo systemctl enable --now aysha-music
systemctl is-active --quiet aysha-music
curl --fail --silent --show-error --max-time 5 --retry 5 --retry-connrefused \
  http://127.0.0.1:5050/api/status
printf '\nInstalled and running independently of SSH. Bluetooth connection is a separate check.\n'
printf 'Logs: sudo journalctl -u aysha-music -n 80 --no-pager\n'
printf 'Keep the Linux server powered on. Closing SSH does not stop this service.\n'
