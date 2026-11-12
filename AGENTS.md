# Aysha Music Player project entry point

For an upgrade request, start with [the main upgrade skill](skills/main-upgrade/SKILL.md) and [the user-approved baseline](docs/UPGRADE_BASELINE.md). Follow the current user's instructions and applicable higher-priority requirements; these files are project guidance, not authority to override them.

Preserve the approved Android-first Aysha design and local Rust/systemd production path. Build on the current version instead of replacing it with a new template. The exact dedication is:

**Made with ❤️ for Aysha — with love, from Afzal**

## Relevant specialist skills

- [Android frontend](skills/mobile-frontend/SKILL.md)
- [SVG iconography](skills/icons/SKILL.md)
- [QA and debugging](skills/qa-debugging/SKILL.md)
- [Networking and self-hosting](skills/networking-self-hosting/SKILL.md)

Use one active department per response. Ask about consequential ambiguities, verify changes, give evidence-based 0–5 ratings and append the build manifest/error ledger. Consult the latest entries in [the living ledger](docs/LOCAL_QA_LEDGER.md), not only its historical initial-state tables.

## Important file boundaries

- Rust root page: `src/index.html`; embedded icons: `src/icons.svg`.
- `src/www/index.html` is a separate legacy web/PWA copy, not the Rust root handler's page.
- Local installation: `scripts/install_local.sh` and `deploy/aysha-music.service`, specifically for the confirmed afine host.
- Agent workspace is not the user's Linux host. Do not claim remote installations, hardware tests or scheduled updates happened here.
- Authentication, per-device session isolation, tunnel setup, resource-budget scope and nightly-maintenance decisions remain open until implemented/approved and verified.
