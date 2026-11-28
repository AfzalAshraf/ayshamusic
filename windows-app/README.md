# Aysha Music — Windows Rust-native candidate

This is an **early candidate**, not a production-ready or full Android clone. The UI and backend are Rust, with Windows system APIs for audio and file dialogs. The Windows ZIP contains the Rust app, a pinned `yt-dlp.exe` (including its Python runtime), and a pinned Deno JavaScript runtime. No separate Python, Node, Deno, browser or external Aysha server installation is required. Keep all three executables in the same folder. Dependencies such as egui, rodio/CPAL and system APIs are still used. There is no auto-updater.

## Local music

Choose **Open M4A file** for a single compatible file, or **Choose music folder** to browse M4A files directly in a folder such as one copied from the phone. The folder scan is non-recursive and bounded to 500 songs and 5,000 directory entries. The chosen folder path is remembered in a local SQLite settings database and rescanned on startup. Use **Refresh folder** to rescan or **Forget folder** to remove the saved setting without deleting music. The player reads the file; it never imports, modifies, or deletes it. Files must be compatible M4A/AAC and at most 64 MiB. The previous Windows candidate's local file playback was confirmed by the user; this folder browser and saved-setting behavior require real-device testing.

## Online playback is unresolved

Search uses the existing early Rust implementation; online song preparation now uses the user-approved separate yt-dlp process. Version 0.4.0 produced format-unavailable with its M4A-only filter. This candidate accepts direct WebM/Opus or M4A/MP4 AAC audio and decodes it without re-encoding (Opus uses bundled libopus). No compatible format or a provider restriction may still prevent playback. Downloads buffer at most 64 MiB before playback, use data and may take time; fast progressive startup is not yet implemented. yt-dlp uses the pinned local Deno runtime with its bundled JavaScript component; it does not fetch remote components. User config, plugins, cookies, proxy routing, retries and client rotation are not enabled. The runtime executes provider JavaScript, which carries security and compatibility risks; upstream's JIT-less setting is requested. There is no automatic update. Networks requiring a proxy may fail. A provider sign-in/bot challenge stops online playback for this app session; no bypass is attempted. No live provider probes are run by CI. A provider may deny access or change its interfaces at any time. Local playback is independent of the online provider.

## Error reports

Use **View error report**, then **Copy error report**, to share recent recognized errors. Reports are held in memory, never sent automatically or written to disk, and omit raw provider messages, track names, search text, file paths, URLs, and credentials. Unknown details are omitted for privacy. Review the report before sharing. Reports cannot capture failures before the UI starts.

## Limitations and development

Windows data lives separately under `%LOCALAPPDATA%\AyshaMusicRustDesktop`; Android data is not shared. Telegram account setup, automatic continuation, preferences, full library features, UI parity, packaging/installer, signing and other Android features remain incomplete. The app executable is an unsigned CI candidate; Windows application-control policy has blocked an earlier build. A new build does not resolve signing, and either bundled executable may also be blocked. Automated tests do not prove physical speaker output or provider availability. See `docs/WINDOWS_RESEARCH.md` for the linked-project review and remaining release gates.

Build on Windows with Rust and MSVC: `cargo build --release --manifest-path windows-app/Cargo.toml`.

## Audio dependencies

Windows WebM/Opus decoding uses Symphonia (MPL-2.0), symphonia-adapter-libopus (MIT OR Apache-2.0), and statically built libopus (BSD-style license). These components are included for decoding; they do not transcode or alter the downloaded audio.

The diagnostic report includes fixed categories for selected yt-dlp warnings when available; raw warnings are not retained in the report. Some yt-dlp causes are logged only at debug level and are not captured, so an unqualified format error does not identify the cause. This diagnostic change does not itself fix provider availability.
