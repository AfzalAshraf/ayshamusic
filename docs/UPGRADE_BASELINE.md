# User-approved upgrade baseline

## Approval and purpose

After the Android-first redesign, the user said: "Fantastic and the best i love it" and asked that this become the foundation for future upgrades. This document records that approval. It is not evidence that every department or target-device acceptance test is complete.

- Approved frontend source: `4e19638` (Android portrait UI, history-aware navigation and full player).
- Verification record: `307db41`; Linux CI run 43078897480 passed Rust tests/release build, Python/frontend tests, shell syntax and seven-viewport browser fixture checks.
- Future baseline: current approved version plus subsequent verified improvements. Do not reset later work back to these commits.
- Upgrade entry point: `skills/main-upgrade/SKILL.md`, linked from `AGENTS.md`.

## Identity and dedication

Keep Aysha's Music original branding and the approved dark/rose, mobile-first design. The requested visible dedication, on both the page and expanded-player footer, is exactly:

**Made with ❤️ for Aysha — with love, from Afzal**

The heart is the real red-heart emoji U+2764 U+FE0F, not the text word "love", the unstyled HTML hearts entity, an icon font or a downloaded image. An accessible label may accompany it without changing the visible sentence. Other functional icons remain project-owned local SVGs.

## Preserve

- Portrait-responsive Android Chromium layout, safe areas and bottom navigation labels.
- Mini-player and full-screen Now Playing sharing the same audio element.
- Tab/modal browser Back/Forward, visible in-app back button, search and stale-response protection.
- Clear loading, empty and failure states; native controls, labels, keyboard focus, zoom and reduced-motion behavior.
- Local Rust music-server, port 5050 and systemd; no automatic substitution with Python/cloud hosting.
- Existing source/test boundaries, regression suites and installer backups.

## Known limits: do not mistake approval for completion

- Real Android hardware Back, screen-lock/background playback, TalkBack and network-change tests remain pending unless the ledger records new evidence.
- Devices still share global backend playback state; independent simultaneous listening is not yet implemented.
- The JBL last reported `org.bluez.Error.Failed br-connection-page-timeout`. UI approval does not establish a hardware fix.
- Cloudflare is not deployed. Domain, access policy, provider media suitability and origin protection must be decided before public launch.
- Nightly maintenance time/timezone/scope and idle-update behavior are not approved. Do not auto-update arbitrary branch code or restart active music without agreement.
- The latest target supersedes the historical 100 MB/20 MB/10–15 MB budgets: all application-server responsibilities in Rust strictly below 30 MB total, with no hidden runtime helpers or off-host fallback. Latest supplied service-cgroup samples reached 101,171,200 bytes, failing both 30 MB and 30 MiB ceilings. Listener count, decimal/binary units, complete shared-service accounting and independent/synchronized semantics remain unresolved; see MEMORY_BUDGET_PLAN.md and ALL_RUST_UPGRADE_GATES.md. No native playback or measured multi-device budget acceptance has been demonstrated.
- Queue/history/likes are in-memory; persistent storage semantics remain unapproved.
- Sequential audio proxy does not implement Range/206 seeking; no offline/PWA guarantees are made.

## Starting readiness scores

| Department | Rating | Gate before claiming 5/5 |
| --- | --- | --- |
| Product & Architecture | 2/5 | Approved session/access/resource/persistence and maintenance decisions. |
| UI/UX Design & Frontend | 4/5 | Actual Android/accessibility/audio acceptance and no regressions from this approved design. |
| Backend & Database | 2/5 | Device isolation, owned processes, bounded concurrency and agreed storage behavior. |
| QA, Security & Debugging | 3/5 | Real-device/hardware/security/load and recovery evidence. |
| Networking & Self-Hosting | 1/5 | Approved authenticated tunnel, protected origin, cellular tests and verified operation/maintenance. |

Iconography is a UI specialist, not a separate artificial department. The main skill is a procedure, not an extra application-readiness score. Scores change only with scope/evidence, recorded in the living ledger.

## Small changes and future redesigns

The dedication correction is explicitly authorized and does not reopen the approved layout. Other visual or architectural replacements need their own user approval. "Upgrade" means preserve + improve + verify, not silently redesign, add services, buy a domain or schedule maintenance.
