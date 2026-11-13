#!/usr/bin/env bash
# Explicitly installs the confirmed afine host's Rust service, without cloud components.
set -euo pipefail
transport=legacy
case "${1:-}" in
  --help)
    printf 'Usage: bash scripts/install_local.sh [--rust-stream-trial]\n'
    printf 'Default: legacy transport. Trial: short yt-dlp resolver + Rust HTTP audio.\n'
    printf 'Both rebuild/restart the service; neither certifies all-Rust or below50MB.\n'
    exit 0 ;;
  --rust-stream-trial) transport=rust-resolved ;;
  "") ;;
  *) printf 'Unknown installer option; no changes made. Use --help.\n' >&2; exit 2 ;;
esac
[[ $# -le 1 ]] || { printf 'Too many installer options; no changes made.\n' >&2; exit 2; }
cd "$(dirname "${BASH_SOURCE[0]}")/.."
if [[ "$(id -un)" != afine || "$HOME" != /home/afine || "$(id -u)" != 1000 ]]; then
  printf 'This installer targets the confirmed afine UID 1000 host. Do not run as root.\n' >&2
  exit 1
fi
for tool in cargo sudo systemctl loginctl pgrep curl mktemp sed grep; do
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
# Render only the known mode marker, never arbitrary environment values.
marker='Environment="MUSIC_STREAM_TRANSPORT=legacy"'
[[ $(grep -cFx "$marker" deploy/aysha-music.service) -eq 1 ]] || {
  printf 'Service transport marker missing/ambiguous; no service changes made.\n' >&2; exit 1;
}
unit_candidate=$(mktemp)
trap 'rm -f "$unit_candidate"' EXIT
sed "s/^Environment=\"MUSIC_STREAM_TRANSPORT=legacy\"$/Environment=\"MUSIC_STREAM_TRANSPORT=$transport\"/" \
  deploy/aysha-music.service > "$unit_candidate"
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
trap 'rm -f "$temporary" "$unit_candidate"' EXIT
install -m 755 target/release/music-server "$temporary"
sudo systemctl stop aysha-music
if [[ -f "$dest" ]]; then cp -p "$dest" "$dest.backup-$stamp"; fi
mv -f "$temporary" "$dest"
sudo install -m 644 "$unit_candidate" "$unit"
sudo systemctl daemon-reload
# Keep the user's PipeWire manager available after SSH logout and across boot.
sudo loginctl enable-linger afine
sudo systemctl enable --now aysha-music
systemctl is-active --quiet aysha-music
status=$(curl --fail --silent --show-error --max-time 5 --retry 5 --retry-connrefused \
  http://127.0.0.1:5050/api/status)
printf '%s' "$status" | /usr/bin/node -e '
  const status = JSON.parse(require("node:fs").readFileSync(0, "utf8"));
  if (status.stream_transport !== process.argv[1]) process.exit(1);
' "$transport"
printf '\nInstalled and running independently of SSH. Stream transport: %s\n' "$transport"
if [[ "$transport" == rust-resolved ]]; then
  printf 'OPT-IN TRIAL: yt-dlp/optional JS still resolves URLs; Rust transfers audio.\n'
  printf 'Not verified: audible host playback, provider compatibility or total below50MB.\n'
  printf 'HTTP health is not sound acceptance. Restore default transport with this installer without the trial flag.\n'
fi
printf 'Bluetooth connection is a separate check.\n'
printf 'Logs: sudo journalctl -u aysha-music -n 80 --no-pager\n'
printf 'Keep the Linux server powered on. Closing SSH does not stop this service.\n'
