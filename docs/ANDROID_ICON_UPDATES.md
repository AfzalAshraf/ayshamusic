# Android icons and Rust settings milestone

Settings offers eight native launcher icons. Android's package manager applies the selected launcher alias; some launchers refresh slowly. The transparent embossed original is composited on a neutral launcher background.

## No automatic updates

The in-app updater, install permission and release publishing automation have been removed at the user's request. CI produces explicitly labelled preview artifacts only. There is no background update check, download, or package installer in this app. App installation or updates are manual.

## Rust milestone 1: color theme preference

The phone's embedded Rust server validates and stores the selected Crimson, Charcoal or Aurora palette in the app-private `color-theme.json`, using a bounded read and atomic replacement. On first launch after upgrading, the existing WebView palette is migrated to Rust; subsequent launches restore the Rust preference. JavaScript still renders the theme and maintains a local fallback; Android launcher selection and Media3 remain native platform code. This is a small milestone, not a claim that the whole app is Rust.

## Signing and data preservation

The package ID remains `com.afzal.ayshamusic.androidaudio10`. An in-place upgrade requires the signing key of the installed app and a higher Android version code. That key is currently unknown; a new CI debug key cannot be assumed to match. Android may reject the APK, and removing auto-update does not fix signing incompatibility. Never uninstall or clear data to force installation. Signing secrets must only be managed in secure build configuration, never chat or source files. A successful CI build or emulator test does not establish physical phone installation or Telegram session preservation.
