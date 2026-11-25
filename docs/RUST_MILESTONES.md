# Aysha Android — incremental Rust milestones

Aim: move suitable first-party app logic and preferences into Rust in small, tested steps while preserving the app the user already likes. This is a plan, not a promise that 99% of an Android APK can be Rust. GitHub language totals include tests and tools and are not an app-runtime breakdown. No fixed percentage has been measured yet.

## Completed on main

1. **Palette preference:** Rust validates, stores and restores Crimson, Charcoal and Aurora. Existing WebView preference migrates on first use; UI still renders it. `src/color_theme.rs`.
2. Earlier app functionality already uses Rust for the local server, Telegram, YouTube resolution, playback modes/queue decisions and more. Those were not all part of a single conversion milestone.

## Current candidate

3. **Shared-link validation:** Android receives the text intent; a narrow JNI call sends bounded text to Rust, which extracts only one validated YouTube video ID without fetching it. Kotlin delivers only that ID to the verified local page and retains the explicit playback confirmation. Automated offline parser tests and release build are separate from device JNI and physical share acceptance.

4. **Comfort quote preferences:** Rust validates and atomically stores saved quote IDs, category choices and opt-in personalization. Android migrates the existing WebView preference, maintains a local fallback mirror and retries dirty changes on next launch. The actual quote UI and selection remain JavaScript; physical-device migration is not yet tested.

## Candidates — choose and validate each independently

5. Inventory remaining browser-stored preferences and saved content (including song ratings). Decide data ownership, schema, bounded storage, first-use migration, failure feedback, concurrent edits and rollback before moving them. Preserve existing choices; do not move unrelated browser/server functionality without tests.
5. Audit remaining input validation and app settings; move self-contained policy and state transitions into Rust where duplication can be removed. Preserve playback priority and Telegram account semantics.
6. Review artwork processing separately for quality, binary size and licensing before replacing Android bitmap APIs. No change just to increase a language metric.
7. Review UI logic selectively. HTML/CSS/JS remain the current interface; any framework or UI rewrite needs separate approval and accessibility/interaction tests.
8. Reduce the Android bridge only where feasible; Android lifecycle, intents, launcher aliases, notification/MediaSession integration and approved Media3 output may legitimately remain Kotlin/Java.

For every milestone record the source revision, offline tests, Android build, artifact, physical/device gaps, installation/signing constraints and recovery strategy. Never delete sessions or application data to force installation. The user asked for no automatic updater; do not reintroduce one.
