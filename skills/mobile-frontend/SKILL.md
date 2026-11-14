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
