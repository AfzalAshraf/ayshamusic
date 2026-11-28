# Windows native player: research and release gates

Research only, November 2026. None of the projects below was copied or linked, and no provider request was made during this investigation. Read the actual licenses and provenance before any future adoption.

- [OpenSelena/libreget](https://github.com/OpenSelena/libreget): GPL-3.0; Tauri/Rust/Svelte downloader with many external tools. Not a self-contained Rust extractor.
- [Rust yt-dlp crate](https://docs.rs/yt-dlp/latest/yt_dlp/): GPL-3.0-only Rust wrapper around the external yt-dlp command; its documented dependency download behavior conflicts with our no-auto-update/no-Python native direction.
- [Open Video Downloader](https://github.com/jely2002/youtube-dl-gui): AGPL-3.0 Rust/Tauri GUI around yt-dlp; advertises automatic updates. No code used.
- [youtube-tui](https://github.com/Siriusmart/youtube-tui): GPL-3.0 terminal frontend that uses other players/downloaders. No code used.
- [yt-dlv](https://github.com/nhodges/yt-dlv): Unlicense Rust rewrite, early work-in-progress according to its own README. It describes an embedded C QuickJS engine and features for provider challenges; it is not a drop-in replacement and has not been integrated or security-reviewed. We do not use its bot/PO-token or identity-rotation approaches.

## Current milestone and gates

The user chose a single Rust-native Windows executable instead of packaging yt-dlp/Python. The Windows source resolver is the project's existing early Rust code, not a newly proven provider extractor. A user-reported 403 remains unresolved. The prior yt-dlp candidate's `format_unavailable` only indicates its specific M4A/AAC selector did not match, not why. Provider sign-in/bot challenges remain terminal and must not be bypassed. No additional live provider testing is authorized merely by these links.

The current milestone adds an explicitly selected, bounded local M4A folder and retains existing file playback. It is not production-ready. Before such a claim, independently review dependencies and licenses; pin reproducible dependencies, audit native dialogs/file handling, test offline seeking and playback on real Windows hardware, test failures/cancellation and permissions, complete queue/library/preferences/Telegram parity, accessibility and installer/signing, perform privacy and security review, and validate permitted online sources without bypassing restrictions. Avoid silently adding automatic updates.

## Windows route consistency milestone

The metadata client previously honored configured proxies while the media range client explicitly disabled proxies. These paths can reach the provider via different network addresses. The Windows metadata client now disables proxies too, matching the existing media policy; no route fallback or identity rotation is performed. A Windows network requiring a proxy may not work. This is a confirmed code inconsistency, not proof of the user's 403 cause or a verified provider fix. The `ANDROID` label is a provider client profile, not the device OS.
