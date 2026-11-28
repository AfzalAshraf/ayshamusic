# Aysha Music — Windows Rust-native candidate

This is an **early candidate**, not a production-ready or full Android clone. The UI and backend are Rust, with Windows system APIs for audio and file dialogs. The Windows ZIP contains the Rust app and a separate pinned `yt-dlp.exe` (including its Python runtime). No separate Python, Node, browser or external Aysha server is required. Keep both executables in the same folder. Dependencies such as egui, rodio/CPAL and system APIs are still used. There is no auto-updater.

## Local music

Choose **Open M4A file** for a single compatible file, or **Choose music folder** to browse M4A files directly in a folder such as one copied from the phone. The folder scan is non-recursive and bounded to 500 songs and 5,000 directory entries. The chosen folder path is remembered in a local SQLite settings database and rescanned on startup. Use **Refresh folder** to rescan or **Forget folder** to remove the saved setting without deleting music. The player reads the file; it never imports, modifies, or deletes it. Files must be compatible M4A/AAC and at most 64 MiB. The previous Windows candidate's local file playback was confirmed by the user; this folder browser and saved-setting behavior require real-device testing.

## Online playback is unresolved

Search uses the existing early Rust implementation; online song preparation now uses the user-approved separate yt-dlp process. Version 0.4.0 produced format-unavailable with its M4A-only filter; this build also accepts MP4-labeled audio-only AAC, then validates an MP4 container before playback. This may not solve the error. Downloads buffer at most 64 MiB before playback, use data and may take time. Config, plugins, cookies, JavaScript runtimes, remote components, proxy routing, retries and client rotation are not enabled. Networks requiring a proxy may fail. A provider sign-in/bot challenge stops online playback for this app session; no bypass is attempted. No live provider probes are run by CI. A provider may deny access or change its interfaces at any time. Local playback is independent of the online provider.

## Error reports

Use **View error report**, then **Copy error report**, to share recent recognized errors. Reports are held in memory, never sent automatically or written to disk, and omit raw provider messages, track names, search text, file paths, URLs, and credentials. Unknown details are omitted for privacy. Review the report before sharing. Reports cannot capture failures before the UI starts.

## Limitations and development

Windows data lives separately under `%LOCALAPPDATA%\AyshaMusicRustDesktop`; Android data is not shared. Telegram account setup, automatic continuation, preferences, full library features, UI parity, packaging/installer, signing and other Android features remain incomplete. The `.exe` is an unsigned CI candidate and SmartScreen may warn about it. Automated tests do not prove physical speaker output or provider availability. See `docs/WINDOWS_RESEARCH.md` for the linked-project review and remaining release gates.

Build on Windows with Rust and MSVC: `cargo build --release --manifest-path windows-app/Cargo.toml`.
