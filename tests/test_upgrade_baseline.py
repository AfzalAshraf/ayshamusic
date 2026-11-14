"""Guard the explicit dedication and discoverable upgrade procedure."""
import html
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
DEDICATION = "Made with ❤️ for Aysha — with love, from Afzal"


class UpgradeBaselineTests(unittest.TestCase):
    def test_both_served_footers_have_the_exact_dedication(self):
        page = (ROOT / "src/index.html").read_text()
        main = re.search(r'<div class="footer">(.*?)</div>', page, re.S)
        expanded = re.search(r'<div class="now-footer"><span>(.*?)</span><button', page, re.S)
        for match in (main, expanded):
            self.assertIsNotNone(match)
            text = html.unescape(re.sub(r"<[^>]+>", "", match.group(1)))
            self.assertEqual(text, DEDICATION)
            self.assertIn("\u2764\ufe0f", text)

    def test_requested_home_heading_and_profile_name(self):
        page = (ROOT / "src/index.html").read_text()
        self.assertIn('Your music.<br><span>Your kind of moment. For you from my heart</span>', page)
        self.assertIn('<div class="profile-mark" aria-label="Aysha\'s Music">AYSHA</div>', page)

    def test_upgrade_skill_is_discoverable_and_links_existing_procedures(self):
        entry = (ROOT / "AGENTS.md").read_text()
        skill = (ROOT / "skills/main-upgrade/SKILL.md").read_text()
        self.assertIn("skills/main-upgrade/SKILL.md", entry)
        self.assertTrue(skill.startswith("---\nname: aysha-main-upgrade\ndescription:"))
        for path in ("docs/UPGRADE_BASELINE.md", "skills/mobile-frontend/SKILL.md", "skills/icons/SKILL.md", "skills/networking-self-hosting/SKILL.md"):
            self.assertIn(path, skill)
            self.assertTrue((ROOT / path).is_file(), path)
        self.assertIn(DEDICATION, skill)


if __name__ == "__main__":
    unittest.main()
