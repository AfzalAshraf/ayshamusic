# Launcher artwork

## Current: Build20 romantic launcher

`romantic-launcher.png` is AI-generated rose-gold heart/A artwork inspired by the
user's `IMG_6795.jpeg` reference. It replaces the earlier black/silver launcher at
the user's explicit request. Full-bleed plum field, no tiny wordmark, centered
emblem inside the adaptive safe area; legacy circle/squircle masks provided.
The player Play icon and app theme colors are unchanged by launcher generation.

Regenerate the checked-in density/adaptive resources:

```
python3 android-app/tools/generate_launcher_icons.py
```

Pillow is a development tool only, never bundled in the APK. The master is1024px,
adaptive foreground512px, legacy variants48/72/96/144/192px. Background `#210817`.

## Archived original

`aysha-launcher.png` preserves the original user-supplied image-9 black/silver
master, cropped to source rectangle402,85–993,676 with surrounding background
removed. This is historical source, no longer the packaged Build20 icon.

## Install continuity

The package ID/port remain unchanged. Preview signing continuity has not been
independently inspected. If Android rejects Update, do not uninstall or clear
app data: that can lose the local Telegram session/settings.
