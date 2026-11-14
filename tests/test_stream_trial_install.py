"""Offline installer option/template tests; never run sudo/systemctl/install/build."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
INSTALLER = ROOT / 'scripts/install_local.sh'
UNIT = ROOT / 'deploy/aysha-music.service'


class StreamTrialInstallTests(unittest.TestCase):
    def safe_cli(self, *args):
        # Empty PATH makes accidental preflight/host-command execution fail.
        with tempfile.TemporaryDirectory() as empty:
            return subprocess.run(['/bin/bash', str(INSTALLER), *args], cwd=ROOT,
                                  env=dict(os.environ, PATH=empty), capture_output=True, text=True, timeout=5)

    def test_help_explains_explicit_trial_without_calling_any_host_commands(self):
        result = self.safe_cli('--help')
        self.assertEqual(result.returncode, 0)
        self.assertIn('--rust-stream-trial', result.stdout)
        self.assertIn('neither certifies all-Rust or below50MB', result.stdout)
        self.assertEqual(result.stderr, '')

    def test_invalid_or_extra_flags_fail_before_any_host_command(self):
        for args in [('--unknown',), ('--rust-stream-trial', 'extra'), ('', 'extra')]:
            result = self.safe_cli(*args)
            self.assertEqual(result.returncode, 2)
            self.assertIn('no changes made', result.stderr)

    def test_template_keeps_legacy_default_and_render_only_changes_one_service_setting(self):
        source = UNIT.read_text()
        marker = 'Environment="MUSIC_STREAM_TRANSPORT=legacy"'
        self.assertEqual(source.splitlines().count(marker), 1)
        self.assertLess(source.index(marker), source.index('[Install]'))
        for mode in ['legacy', 'rust-resolved']:
            expression = f's/^Environment="MUSIC_STREAM_TRANSPORT=legacy"$/Environment="MUSIC_STREAM_TRANSPORT={mode}"/'
            result = subprocess.run(['sed', expression, str(UNIT)], capture_output=True, text=True, check=True, timeout=5)
            self.assertEqual(result.stdout, source.replace(marker, f'Environment="MUSIC_STREAM_TRANSPORT={mode}"'))
            self.assertIn('KillMode=control-group', result.stdout)
            self.assertIn('ExecStart=/home/afine/music-server/target/release/music-server', result.stdout)

    def test_shell_syntax_and_build_before_service_stop_are_preserved(self):
        subprocess.run(['/bin/bash', '-n', str(INSTALLER)], check=True, timeout=5)
        source = INSTALLER.read_text()
        self.assertLess(source.index('cargo test'), source.index('sudo systemctl stop'))
        self.assertLess(source.index('cargo build --release'), source.index('sudo systemctl stop'))
        self.assertIn('status.stream_transport !== process.argv[1]', source)
        self.assertIn('sudo cp -a "$unit" "$unit.backup-$stamp"', source)
        self.assertIn('"$dest.backup-$stamp"', source)


if __name__ == '__main__':
    unittest.main()
