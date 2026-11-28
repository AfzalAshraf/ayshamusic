# Aysha Music — Windows Rust-native candidate

This is an **early candidate**, not a production-ready or full Android clone. The UI and backend are Rust, with Windows system APIs for audio and file dialogs. The release artifact contains one application executable; it does not require yt-dlp, Python, Node, a browser or an external server. Dependencies such as egui, rodio/CPAL and system APIs are still used. There is no auto-updater.

## Local music

Choose **Open M4A file** for a single compatible file, or **Choose music folder** to browse M4A files directly in a folder such as one copied from the phone. The folder scan is non-recursive and bounded to 500 songs and 5,000 directory entries. It is not retained after app exit. The player reads the file; it never imports, modifies, or deletes it. Files must be compatible M4A/AAC and at most 64 MiB. The previous Windows candidate's local file playback was confirmed by the user; this new folder browser and build require real-device testing.

## Online playback is unresolved

The existing Rust search and source resolver are early implementations. Online playback previously produced HTTP 403 on the user's Windows device. This build does not fix that provider refusal and does not bypass bot challenges. The Windows path uses only one existing source profile; it doesn't automatically rotate profiles following a refusal. No live provider probes are run by CI. A provider may deny access or change its interfaces at any time. Local playback is independent of the online provider.

## Error reports

Use **View error report**, then **Copy error report**, to share recent recognized errors. Reports are held in memory, never sent automatically or written to disk, and omit raw provider messages, track names, search text, file paths, URLs, and credentials. Unknown details are omitted for privacy. Review the report before sharing. Reports cannot capture failures before the UI starts.

## Limitations and development

Windows data lives separately under `%LOCALAPPDATA%\AyshaMusicRustDesktop`; Android data is not shared. Telegram account setup, automatic continuation, preferences, full library features, UI parity, packaging/installer, signing and other Android features remain incomplete. The `.exe` is an unsigned CI candidate and SmartScreen may warn about it. Automated tests do not prove physical speaker output or provider availability. See `docs/WINDOWS_RESEARCH.md` for the linked-project review and remaining release gates.

Build on Windows with Rust and MSVC: `cargo build --release --manifest-path windows-app/Cargo.toml`.
