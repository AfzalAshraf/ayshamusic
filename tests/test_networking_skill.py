"""Documentation-contract checks, not network/deployment acceptance tests."""
from pathlib import Path
import re
import unittest
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parents[1]
SKILL = ROOT / "skills/networking-self-hosting/SKILL.md"


class NetworkingSkillContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.text = SKILL.read_text()

    def test_skill_metadata_and_no_deployment_authority(self):
        self.assertTrue(self.text.startswith("---\nname: networking-self-hosting\n"))
        self.assertIn("description:", self.text.split("---", 2)[1])
        self.assertIn("Never deploy merely because this skill exists", self.text)
        self.assertIn("Architecture approval", self.text)

    def test_project_budget_and_single_preview_contract(self):
        self.assertIn("100,000,000 bytes (100 MB)", self.text)
        self.assertNotIn("20 MB", self.text)
        self.assertIn("do not recreate the duplicate 5051 preview", self.text)
        self.assertIn("Preview servers bind `0.0.0.0`", self.text)
        self.assertIn("production LAN-binding changes require approval", self.text)

    def test_auth_origin_and_secret_boundaries_remain_explicit(self):
        for requirement in (
            "connector validation only protects requests traversing that connector",
            "CORS is not authentication or CSRF protection",
            "Session IDs are not authentication",
            "--token-file",
            "No passwords, tokens, cookies, signed media URLs or private keys",
            "malformed/unknown/unverifiable tokens fail closed",
        ):
            with self.subTest(requirement=requirement):
                self.assertIn(requirement.lower(), self.text.lower())

    def test_local_references_resolve(self):
        links = re.findall(r"\]\(([^)]+)\)", self.text)
        local = [link for link in links if not urlparse(link).scheme and not link.startswith("#")]
        self.assertGreaterEqual(len(local), 5)
        for link in local:
            with self.subTest(link=link):
                self.assertTrue((SKILL.parent / link.split("#", 1)[0]).resolve().is_file())

    def test_research_register_uses_primary_official_sources(self):
        register = self.text.split("## Official research register", 1)[1]
        urls = re.findall(r"https://[^\s)]+", register)
        hosts = {urlparse(url).hostname for url in urls}
        self.assertTrue({"developers.cloudflare.com", "www.freedesktop.org", "github.com",
                         "developer.mozilla.org", "cheatsheetseries.owasp.org", "www.cloudflare.com"} <= hosts)
        self.assertIn("github.com/systemd/systemd/", register)
        self.assertIn("Recheck version-specific flags", register)

    def test_acceptance_evidence_and_rollback_are_not_readiness_claims(self):
        for requirement in (
            "PASS / FAIL / NOT RUN",
            "1/5 — current",
            "Writing a stronger skill does not raise application readiness",
            "Controlled rollout and rollback",
            "MemoryMax",
            "Neither alone proves continuous playback",
            "disable/revert only the newly exposed route",
        ):
            with self.subTest(requirement=requirement):
                self.assertIn(requirement, self.text)


if __name__ == "__main__":
    unittest.main()
