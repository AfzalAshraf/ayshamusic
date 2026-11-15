"""Offline provenance/completeness checks; imported skills are never executed."""
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
ECC = ROOT / "vendor/ecc"


class EccReferenceTests(unittest.TestCase):
    def test_all_pinned_skill_files_match_manifest(self):
        manifest = json.loads((ECC / "manifest.json").read_text())
        self.assertEqual(manifest["revision"], "d2ef2680594f9a17526a6e1a2e18b3d242c5d17b")
        self.assertEqual(manifest["mode"], "inert_reference_no_hooks_or_install")
        files = manifest["files"]
        self.assertEqual(len(files), 620)
        self.assertEqual(manifest["skill_count"], 314)
        self.assertEqual(len(list((ECC / "skills").glob("*/SKILL.md"))), 314)
        for entry in files:
            path = ECC / entry["path"]
            self.assertTrue(path.resolve().is_relative_to(ECC.resolve()))
            self.assertFalse(path.is_symlink())
            self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), entry["sha256"], entry["path"])

    def test_reference_license_index_and_local_failure_skill_exist(self):
        self.assertIn("MIT License", (ECC / "LICENSE").read_text())
        index = (ECC / "INDEX.md").read_text()
        for name in ["search-first", "rust-testing", "verification-loop", "security-review"]:
            self.assertIn(f"(skills/{name}/SKILL.md)", index)
        self.assertTrue((ROOT / "skills/source-playback-debugging/SKILL.md").exists())


if __name__ == "__main__":
    unittest.main()
