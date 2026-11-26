# Startup, timer and Windows candidate

## Android startup

The app already sets a dark Android window background, but WebView's default surface can be white before the local document is rendered. Set its color to `#030303` **before** adding it to the view hierarchy. On Android 12+ set the system splash background explicitly to the same color; older Android versions inherit the dark window theme. This is a plausible fix for the reported flash, not a claim that every device or launcher was physically verified.

## Timer

Media3 sends its actual position on player events and once per second; the WebView separately polls the Rust state once per second. Those phases and message scheduling can make the displayed timer lag by up to roughly two reporting intervals in normal circumstances, and more during delays. The labels display whole seconds. The player is the source of truth; no deliberate audio delay, speed adjustment or artificial timer prediction was introduced. A few milliseconds is not on its own evidence of a playback defect. Distinguishing screen delay from audible sync requires phone observation and a timed fixture or device trace; not yet measured.

## Windows

The user selected a separate Rust-native Windows interface and independent local data. `windows-app/` is an early candidate, not a full clone. The repository's Linux `src/main.rs` uses Unix and Linux tools, so it is not simply portable unchanged. The candidate embeds the existing Rust phone backend in-process without exposing its loopback API or accessing the phone's Telegram session. Windows audio and UI require system libraries, and package dependencies still matter; Rust source does not mean the entire binary is Rust. See `windows-app/README.md` for implemented and missing features. Preserve the Android Media3 path unchanged. CI builds do not prove Windows audio or provider playback.
