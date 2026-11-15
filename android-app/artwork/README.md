# Launcher artwork

`aysha-launcher.png` is the user's supplied image-9.png, cropped to its black/silver
rounded-square icon (source rectangle 402,85–993,676), with only the surrounding
cream background/cast shadow removed by a rounded alpha mask. No AI recreation.
The logo, metallic A and AYSHA text are unchanged.

Regenerate density/adaptive resources with Pillow installed:

```
python3 android-app/tools/generate_launcher_icons.py
```

Pillow is a development tool, not shipped in the APK. Legacy round icons and the
adaptive foreground inset the complete artwork so Android launcher masks cannot
cut off AYSHA. The player Play button, embedded SVG sprite, dedication and UI
colors are not modified.

Build 11 keeps the existing package IDs ending rustaudio10/androidaudio10 and the
same ports. The display names/version distinguish it from code10. CI preview APKs
use ephemeral signing keys; an installed APK signed by another run may not accept
an in-place update. Never uninstall an app with valuable local Telegram/session
state merely to work around that without considering that app-private data loss.
