"""Generate Build20 romantic adaptive/legacy launchers (Pillow is dev-only).

Run: python3 android-app/tools/generate_launcher_icons.py
The original image-9 black/silver master remains archived, not the current icon.
This tool never edits the player Play button or SVG playback controls.
"""
from pathlib import Path
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
MASTER = ROOT / "artwork/romantic-launcher.png"
RES = ROOT / "app/src/main/res"


def main():
    artwork = Image.open(MASTER).convert("RGB")
    if artwork.width != artwork.height:
        raise ValueError("Launcher master must be square")
    artwork.resize((512, 512), Image.Resampling.LANCZOS).save(RES / "drawable-nodpi/ic_launcher_artwork.png", optimize=True)
    # Adaptive artwork has a full-bleed plum field; its heart/A stays in the
    # central safe circle. Legacy launchers need a closer crop of that field.
    side = artwork.width
    inset = side // 8
    legacy = artwork.crop((inset, inset, side-inset, side-inset))
    for density, size in [("mdpi", 48), ("hdpi", 72), ("xhdpi", 96), ("xxhdpi", 144), ("xxxhdpi", 192)]:
        target = RES / f"mipmap-{density}"
        for name, circular in [("ic_launcher", False), ("ic_launcher_round", True)]:
            icon = legacy.resize((size, size), Image.Resampling.LANCZOS).convert("RGBA")
            mask = Image.new("L", (size, size))
            draw = ImageDraw.Draw(mask)
            if circular:
                draw.ellipse((0, 0, size-1, size-1), fill=255)
            else:
                draw.rounded_rectangle((0, 0, size-1, size-1), radius=int(size*.23), fill=255)
            icon.putalpha(mask)
            icon.save(target / f"{name}.png", optimize=True)
    print("Romantic launcher generated; player Play unchanged")


if __name__ == "__main__":
    main()
