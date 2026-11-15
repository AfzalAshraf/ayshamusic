"""Rasterize the user's supplied black/silver Aysha launcher artwork.

Requires Pillow (build-time only): python3 -m pip install Pillow
Run: python3 android-app/tools/generate_launcher_icons.py
The checked-in master is cropped from image-9.png; no AI redraw or player UI edits.
"""
from pathlib import Path
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
MASTER = ROOT / "artwork/aysha-launcher.png"
RES = ROOT / "app/src/main/res"


def main():
    artwork = Image.open(MASTER).convert("RGBA")
    for density, size in [("mdpi", 48), ("hdpi", 72), ("xhdpi", 96), ("xxhdpi", 144), ("xxxhdpi", 192)]:
        icon = artwork.resize((size, size), Image.Resampling.LANCZOS)
        target = RES / f"mipmap-{density}"
        target.mkdir(parents=True, exist_ok=True)
        icon.save(target / "ic_launcher.png")
        # Legacy launchers that request roundIcon should receive a circular
        # silhouette without cropping any of the supplied artwork or AYSHA text.
        round_icon = Image.new("RGBA", (size, size), (18, 22, 27, 255))
        inner = round(size * 0.73)
        round_icon.alpha_composite(artwork.resize((inner, inner), Image.Resampling.LANCZOS), ((size-inner)//2, (size-inner)//2))
        mask = Image.new("L", (size*4, size*4), 0)
        ImageDraw.Draw(mask).ellipse((0, 0, size*4-1, size*4-1), fill=255)
        round_icon.putalpha(mask.resize((size, size), Image.Resampling.LANCZOS))
        round_icon.save(target / "ic_launcher_round.png")
    # Full 108dp adaptive foreground: artwork's corners sit within the central
    # 66dp safe circle. Masked circle/squircle launchers retain the entire logo.
    foreground = Image.new("RGBA", (432, 432))
    foreground.alpha_composite(artwork.resize((184, 184), Image.Resampling.LANCZOS), (124, 124))
    target = RES / "drawable-nodpi"
    target.mkdir(parents=True, exist_ok=True)
    foreground.save(target / "ic_launcher_artwork.png")
    print("User-supplied launcher artwork generated; player Play unchanged")


if __name__ == "__main__":
    main()
