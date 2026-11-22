---
name: android-music-frontend
description: Build and verify a responsive Android Chromium music interface with native browser history, accessible local icons and truthful playback states.
---

# UI/UX Design & Frontend: Android music app skill

Scope: the local Rust-served src/index.html, not the separate legacy src/www asset copy. Do not change backend/session ownership or infrastructure in a visual pass. Seek approval for ambiguous behavior. Use original Aysha branding, inspired music-app interaction patterns and project-owned assets; do not represent it as an official YouTube product.

## Design rules

- Make portrait widths 320, 360, 390 and 412px usable without horizontal viewport scrolling. Also test tablet, phone landscape and desktop.
- Use dynamic viewport units and safe-area insets. Keep bottom navigation and mini-player from covering the last scrollable controls. Do not rely on an unsupported browser orientation lock.
- Keep navigation labels visible. Use native buttons and labelled inputs with at least 44px primary touch targets, visible focus and zoom support.
- Put a readable track title/artist and truthful output destination in both mini and full player. Reuse the same audio element and handlers; never start a second audio stream just to open the player.
- Use local SVG symbols from src/icons.svg via the icon skill. No remote icon fonts or branding copies.
- Use restrained color, spacing, typography, album art and state hierarchy. Never add fake downloads, subscriptions, shuffle or seek buttons without working behavior.
- Display loading, empty and failure states. Make retries possible. Guard against stale search responses overriding a newer request.

## Navigation rules

- Tab changes create same-document history entries. Back/Forward restore the visible tab and its scroll position.
- Opening full-screen Now Playing creates an overlay entry. Android/browser Back closes the overlay first; it must not immediately exit the player site.
- The app's back button follows history and has a safe Home fallback for direct links.
- Escape/cancel closes the modal and maintains consistent history. Native dialog traps focus and restores it on close.
- Search results must not be replaced by a genre request triggered by tab initialization. Preserve loaded pages when returning via history.
- No service-worker/offline/PWA claims unless the Rust server actually serves the manifest/worker and their behavior is tested. Never cache audio or /api/* in a worker.

## Verification

Run the existing frontend and Python regressions, then tests/mobile.browser.cjs using the test-only browser dependencies documented in docs/MOBILE_UI.md. Inspect generated screenshots as well as dimensions. The browser suite uses explicit API fixtures; it does not prove live audio, Bluetooth, Access authentication, session isolation or hardware back behavior on a particular phone.

Acceptance requires real Android Chromium portrait/rotation, Back, screen lock/background, network changes, touch targets and error-state checks. UI readiness may reach 4/5 on automated desktop Chromium emulation, not 5/5 until actual target-device evidence and remaining UI acceptance checks pass.

## Sync and seeking evidence rules (build16)

- Reference mockup counters/songs are never production defaults. Render real bounded queue metadata and explicit missing-data states. Keep Manage separate from counters; allowlist local theme choices without storing account/listening secrets or writing on restoration.
- A stream-reader callback measures bytes consumed by an uploader, not server acknowledgement. Label accordingly; do not call100%read “Saved”. Separate downloading/checking/uploading/confirming and explain app quota waits.
- X must cancel persistent queue work and active future where possible, with a serialized final-send boundary. Disabling X during commit is more truthful than claiming to recall a cloud message. Never delete existing messages to simulate cancellation.
- Native sliders require a real bridge seek operation, current-song identity, finite duration/bounds and stale-event revision rejection. Preserve paused state and don't restart a source just to seek/open NowPlaying. Drag/keyboard preview must survive status updates. Unknown durations disabled. Test actualExoPlayer seek separately from browser request fixtures.
- Build16 adds tests/sync-seek.browser.cjs and actual emulator forward/back paused/resume fixture. Offline green is not real account upload/cancellation or physical-device acceptance. One desktop butterfly timing assertion transiently failed then passed unchanged locally/finalCI; don't hide that history or weaken unrelated assertions to ship a feature.
