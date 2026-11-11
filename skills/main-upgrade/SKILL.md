---
name: aysha-main-upgrade
description: Main entry point for every Aysha Music Player upgrade: preserve the user-approved Android-first baseline, route one department at a time, verify improvements and document honest readiness.
---

# Aysha main upgrade skill

## 1. Mission

Improve the player from the version the user loves, not from a blank canvas. Preserve its identity, usability and working local playback while making measurable improvements in reliability, accessibility, security and performance.

This is the project's main repository procedure. It is not a claim of a newly installed Arena capability, persistent hidden memory or authority to modify the user's host. Follow the current user request and applicable higher-priority instructions. A request to "upgrade" invokes this procedure; it does not authorize every possible change, deployment or scheduled task.

## 2. Approved foundation

Read docs/UPGRADE_BASELINE.md first. The user-approved visual/interaction foundation is source commit 4e19638, with verification recorded at 307db41. Compare against this baseline and the current working tree; do not reset to it or discard later improvements.

Preserve unless the user explicitly asks otherwise:

- Aysha's original dark/rose visual identity, warm artwork, readable typography and locally embedded SVGs.
- Android Chromium as the primary target, portrait-first layout, safe areas, bottom navigation, compact mini-player and expanded Now Playing.
- Browser Back/Forward, modal Back/Escape behavior, search, visible error/loading states, keyboard/zoom/reduced-motion support and touch targets.
- One audio element reused across views; opening a screen is not permission to start a duplicate stream.
- Rust music-server on the local Linux host, port 5050 and systemd. Python/cloud deployment is not the local production replacement.
- Explicitly requested dedication, with an actual red heart emoji:

  **Made with ❤️ for Aysha — with love, from Afzal**

Do not rebrand as YouTube Music, use another product's logo, replace this design with a new template, add fake premium features or restore a desktop-only layout under the label "upgrade".

## 3. Start-of-upgrade inspection

1. Read AGENTS.md, this skill, the approved baseline and the latest entries in docs/LOCAL_QA_LEDGER.md.
2. Inspect git status/diff and relevant source/tests on the current working branch. Preserve unrelated work. Do not assume repository state from an older conversation or install an old binary after a failed build.
3. Restate the requested outcome briefly. Distinguish reported symptoms, inspected source facts, measured results and assumptions.
4. Identify the smallest high-value change and write explicit acceptance checks. Prefer correctness and user benefit over extra features or dependencies.
5. Ask only consequential unanswered questions. Do not repeatedly ask about established facts, but do not silently invent domain names, audience, timezones, permissions, budget scope, synchronization or persistence semantics.
6. Choose one active department for this response. If architecture changes are needed, obtain design approval before the next department implements them. An existing failure routes to QA before adding new features.

## 4. Department routing and specialist skills

| Department | Responsibility | Relevant procedure |
| --- | --- | --- |
| Product & Architecture | Flows, acceptance checks, state boundaries, trade-offs and scope approval | This skill; docs/UPGRADE_BASELINE.md; docs/REMOTE_MULTI_DEVICE_PLAN.md |
| UI/UX Design & Frontend | Android layout, interaction, states and accessibility | skills/mobile-frontend/SKILL.md |
| Iconography (UI specialist) | Semantic, consistent and accessible local SVGs | skills/icons/SKILL.md |
| Backend & Database | API contracts, sessions, owned processes, validation and agreed persistence | src/playback.rs and src/media.rs; approved architecture |
| QA, Security & Debugging | Reproduce failures, regression fixes, security and resource evidence | tests/ and docs/LOCAL_QA_LEDGER.md |
| Networking & Self-Hosting | Authenticated remote access, DNS/tunnel, services and approved maintenance | skills/networking-self-hosting/SKILL.md |

Iconography is a specialist within UI/UX, not an excuse to switch multiple active departments in one response. Read the relevant skill before changing its area. A scoped branding correction explicitly requested by the user does not authorize unrelated visual redesign.

## 5. Improvement order

Use this priority order unless the user directs otherwise:

1. Fix a reproduced playback, security or navigation failure.
2. Preserve the approved mobile experience and correct misleading/nonworking controls.
3. Improve process/session ownership, bounded work and recovery.
4. Add the specifically requested feature with the smallest maintainable implementation.
5. Measure and optimize resources; then polish nonessential details.

Do not treat a large rewrite, new framework, new database, dependency upgrade or background daemon as automatically better. Research current official documentation when behavior, compatibility or policy may have changed. Record the reason and compatibility evidence for any upgrade; do not update all services indiscriminately.

## 6. Change contract

Before implementing, identify:

- **Keep:** baseline behaviors and visuals this work must preserve.
- **Change:** the exact behavior or files being improved.
- **Prove:** automated and manual acceptance checks, including regressions.
- **Recover:** how to revert source/deployment without losing user data or disrupting unrelated services.
- **Ask:** remaining consequential decisions, if any.

Keep this proportional: a footer correction needs a short contract, not a new architecture. A multi-user session change needs a proper design and authorization boundary. Record evidence in the ledger rather than generating duplicate status documents for every tiny edit.

## 7. Implementation standards

- Keep state transitions pure/testable where possible; separate HTTP handlers, streaming/process lifetime, device sessions and physical-speaker ownership.
- Validate API inputs and expose honest failure states. Never silently swallow a broken request as empty success.
- Bound queues, sessions, expensive searches and concurrent downloader work. Cancel/reap only the processes owned by the request/session; do not expand global process-kill patterns.
- Do not let one device's control action mutate another device's playback after independent sessions are introduced. Until that is implemented, explicitly report the global-state limitation.
- Preserve same-origin relative browser URLs. Do not introduce browser-facing localhost URLs for remote listeners.
- Do not add placeholders, fake controls or claims of offline/background/synchronized behavior that the implementation and tests do not support.
- Avoid unnecessary runtime frameworks, icon fonts, CDN dependencies, build artifacts, credentials or large test datasets in Git.

## 8. Verification ladder

Run checks relevant to the change and state their limits:

```bash
node --test tests/frontend.test.cjs
python3 -m unittest discover -s tests -p 'test_*.py' -v
cargo test
cargo build --release
bash -n scripts/install_local.sh scripts/diagnose_local.sh
git diff --check
```

For layout/history changes, run tests/mobile.browser.cjs as described in docs/MOBILE_UI.md. Maintain checks at 320, 360, 390 and 412px portrait, tablet, phone landscape and desktop. Inspect screenshots; dimensions alone do not establish visual quality.

A fixture test proves fixture behavior, not actual audio/hardware/authentication. Successful compilation is not proof that the running systemd binary was replaced. Audio bytes are not proof of audible playback. A working single device is not proof of isolated multiple listeners. Browser emulation is not proof of Android screen-lock behavior.

If a local tool is missing, use the existing Linux CI where appropriate and report what was actually verified. Do not repeatedly ask the user to compile unverified guesses. Do not declare success when CI failed, and do not fabricate a test count or a deployed service.

## 9. Host, security and maintenance boundaries

- The agent workspace is not /home/afine on the user's machine. Never claim a host installation occurred unless it was actually executed and verified there.
- Keep the confirmed systemd installer, backups and environment requirements. The Linux server must stay powered on; closing SSH is a separate issue.
- Do not expose the current unauthenticated/shared-state API publicly. Remote launch requires agreed authentication, origin protection and device isolation.
- Never request or store credentials in chat/Git. Tunnel tokens stay in protected local storage and must be rotated if exposed.
- Cloudflare domain/access/media-plan suitability and nightly update time/timezone/scope are unresolved until explicitly answered. Do not guess them or enable unattended updates to arbitrary branch heads.
- Proposed nightly updates should be idle-window aware, tested, health-checked and recoverable. Approval is required for a specific schedule and scope; do not restart active music silently.
- The user replaced the earlier 20 MB target with a maximum 100 MB target for simultaneous devices. Clarify listener count and whether it covers the entire service process tree/tunnel, then measure idle and loaded peaks. Follow docs/MEMORY_BUDGET_PLAN.md for the pending proposal. Do not claim that using Rust alone bounds subprocess memory, or enforce a limit that simply kills audio and call it optimization.

## 10. Honest 0–5 department ratings

Use the current evidence, not a target number or the user's praise alone:

- **0:** not assessed or absent.
- **1:** scoped/documented, not implemented or integrated.
- **2:** partial implementation with important known gaps.
- **3:** core behavior implemented with meaningful automated verification; important target-system gates remain.
- **4:** broad automated/integration evidence and polish, but named manual/security/target-device gates remain.
- **5:** all agreed department acceptance gates passed with evidence on the relevant target. No unresolved critical gap in its approved scope.

Rate against the requested scope. State why a score changed, and which gate unlocks the next score. A score may decrease when scope expands or a regression is found. Unknown is not the same as passed. A documentation-only skill can be complete as a document without making the application 5/5.

Current starting ratings and pending gates are in docs/UPGRADE_BASELINE.md; append changes with evidence to the ledger.

## 11. Required turn-end record

Every response ends with both logs, scaled to the actual work:

### 📦 BUILD MANIFEST
- Active Department Skill.
- Exact files created/modified and their purpose.
- Dependencies/APIs/environment/service changes (or None).
- Current App State: what is verified, what is reported and what remains unverified.

### 🚨 ERROR & RISK LEDGER
- Active Bugs / Build Errors: exact error and location where known; None only when truly clean within scope.
- Root Cause Analysis: demonstrated cause versus hypothesis.
- Fix Applied / Next Action and the owner/approval gate.
- Edge Cases to Watch: specific risks, not generic reassurance.

Update docs/LOCAL_QA_LEDGER.md after meaningful work. Preserve old entries as history; use the latest verified entry for current state. Do not silently rewrite historical failures as if they never occurred.

## 12. Upgrade completion rule

An upgrade is complete only when the requested outcome works, relevant regression checks pass, the approved foundation is preserved, operational steps/rollback are documented, and remaining limitations are explicit. Present the actual deliverable and exact host commands when needed. Wait for approval before the next department; do not keep making unrelated changes in the name of reaching 5/5.

User shorthand example:

> Upgrade Aysha's Music Player using the main upgrade skill. Keep the approved mobile design, dedication and working playback as the foundation. Improve the requested area, test it and show honest ratings.
