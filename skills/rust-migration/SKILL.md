---
name: aysha-rust-migration
description: Plan and execute small, independently tested Aysha Android Rust migration milestones while preserving working playback, user data, and mandatory Android integration.
---

# Aysha Rust migration

Use this skill when working on Aysha app features or upgrades. The user's standing direction is to migrate suitable app logic to Rust incrementally. Apply higher-priority and current user instructions first; this document is project guidance, not blanket permission to change unrelated features, probe a music provider, publish, or deploy.

## For each chat and milestone

1. Read `AGENTS.md`, the main upgrade skill, current user request, `docs/RUST_MILESTONES.md`, current code/tests and the latest QA ledger. Check the working tree and protect unrelated work. A merged branch cannot be used for edits: follow the current session's branch restrictions.
2. Identify one small, valuable Rust migration relevant to the task, or explicitly explain why migration would be unsafe or unrelated. Never silently expand scope, add runtime helpers, or exchange working features for a larger Rust percentage. Ask before consequential changes.
3. Record Keep / Change / Prove / Recover: preserve session data and preferences, app package ID and signing constraints, current appearance, all approved icons, Rust Telegram/backend and Media3-only Android playback. Preserve existing public-provider restrictions and sealed challenge gates; no provider probing without fresh permission.
4. Move validation, state transitions, preference policy, serialization or algorithms into well-bounded Rust modules. Keep Android platform APIs such as launcher aliases, activity lifecycle, notifications, MediaSession and Media3 in a minimal Kotlin/Java bridge when necessary. Keep the existing embedded UI unless a UI rewrite is separately approved. No Python/Node/external app or hosted runtime.
5. Add offline Rust tests for valid, invalid and boundary cases and integration tests for the actual bridge and persistence migration where feasible. Verify regressions, binary size and Android build. Distinguish host tests, emulator tests, full JNI/physical-device evidence and not-run checks. Do not claim successful installation, session preservation or speaker playback without evidence.
6. Version the Android build appropriately, document the exact change, artifact, tests, remaining risks and next milestone. Do not delete or clear data to resolve signing mismatches. Do not add auto-updates without a new explicit request.
7. Measure only clearly defined metrics (e.g. first-party app logic modules or audited source lines). GitHub language percentages count repository tests, scripts and non-app files, and cannot establish the APK's Rust percentage. Do not remove tests or alter language classification to inflate a number. A 99% outcome is an aspiration, not a guarantee or a reason to remove necessary platform code.

## Design principles

- Prefer pure functions, explicit ownership, bounded inputs, typed models, atomic file updates and stable failure semantics.
- Migrate one path at a time with parity fixtures; do not duplicate inconsistent business rules across Kotlin, JavaScript and Rust.
- Keep secrets, Telegram sessions, existing files and user's choices intact. Never ask for private keys, credentials or auth codes in chat.
- Retain a small, audited JNI/API boundary. Validate both sides, do not expose arbitrary code/URLs, and test realistic lifecycle, restart, ordering and error cases.
- Each milestone must leave the app usable and independently revertible without data loss.

## Windows exception selected by user

On Windows, the user explicitly selected a packaged yt-dlp process for online acquisition in preference to the single Rust-only candidate. This is not a change to Android's standalone Media3-only runtime requirement. Keep the GUI and audio engine in Rust, bound input/CPU/storage, never log secrets, avoid auto-updates and do not bypass provider sign-in/bot challenges. Explain that the packaged yt-dlp includes Python and that a build or an offline format fixture does not prove live provider playback. Keep changes small, preserve local-file playback and use the manually maintained architecture map in `docs/WINDOWS_ARCHITECTURE.md`; this is not a Graphify-generated graph.

The Windows user also approved packaging a pinned Deno JavaScript runtime alongside yt-dlp for its web client. This is a Windows-only exception, not permission to add runtimes to Android, run provider probes, rotate identities, bypass bot checks, or automatically update dependencies. The tool executes provider JavaScript: retain sandbox restrictions, review upstream changes and clearly disclose the associated risk and lack of live verification.
