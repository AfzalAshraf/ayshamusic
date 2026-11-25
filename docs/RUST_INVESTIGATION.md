# Rust migration investigation: Android playback and preferences

## Observed in repository

- `android-app/scripts/build_apk.sh` already cross-compiles the Rust library with `cargo-ndk`, packages ARM64, and builds the Media3 Android flavor. `RustBridge.kt` and the Android module in `src/lib.rs` already provide JNI. UniFFI is not installed; no dependency was added for a small existing bridge.
- Media3 (`androidx.media3:media3-exoplayer`) handles Android playback; Rust owns the embedded backend and playback/queue modes. The current APK is not a Room/local-folder-scanning app. `rusqlite` already backs parts of Telegram session/storage; `mp4ameta` writes supported metadata.
- Album art currently crosses the JNI boundary as a capped byte array. `AlbumArtwork.kt` rejects over 1 MiB input and over 16 million source pixels, downsamples before creating a 320×320 JPEG, and caps output at 200,000 bytes. Bounds do not prove low peak memory and JNI copies may add overhead; no physical allocation trace has been taken.
- The user interface remains HTML/JavaScript in a WebView. Android-specific lifecycle, MediaSession, notifications, launcher icons and Media3 still require the platform bridge. GitHub language statistics include repository tools/tests and are not an APK-runtime measurement.

## This milestone

Rust owns bounded, versioned comfort-quote preferences; Android first imports existing WebView choices, retaining a local fallback mirror. Writes are serialized; edits during initial migration win; dirty records remain available after failed Rust writes and can be retried on next load. Browser/Linux behavior remains localStorage-based. An explicit reset clears the choices in both stores through the same queued update. Theme and playback-mode persistence already had Rust implementations.

## Not established / follow-up

No phone GC, native heap, frame timing, battery, audio dropout or memory profiling was performed. Do not attribute stutter to GC or claim migration reduces RAM without real device data. Collect comparable device traces during the same playback and artwork workload, with clear scope, permissions and no private account data. JNI share parsing is unit-tested in Rust and packaged for ARM64, but the offline x86 emulator excludes the Rust JNI library; on-device share/bridge behavior remains unverified. Album art replacement, UniFFI adoption and any change from the approved Media3 output require separate evidence and review; none was performed here. No provider access was attempted.
