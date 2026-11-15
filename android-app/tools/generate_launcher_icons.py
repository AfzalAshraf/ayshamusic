"""Generate the launcher icons (red disc + music note) with the Python stdlib only.

Run from anywhere: python3 android-app/tools/generate_launcher_icons.py
"""
import math
import os
import struct
import zlib


def _chunk(kind, data):
    return (
        struct.pack(">I", len(data))
        + kind
        + data
        + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
    )


def write_png(path, size, round_icon):
    # A full red disc with transparent corners in BOTH legacy variants. The old
    # regular icon was a small square on an opaque black background.
    red = (232, 45, 70)
    cx = cy = (size - 1) / 2.0
    radius = size * 0.47
    rows = []
    for y in range(size):
        row = bytearray([0])  # filter type 0
        for x in range(size):
            coverage = max(0.0, min(1.0, radius - math.hypot(x - cx, y - cy) + 0.5))
            nx, ny = (x + 0.5) / size, (y + 0.5) / size
            note = (
                ((nx - 0.39) ** 2 + (ny - 0.66) ** 2 <= 0.075 ** 2)
                or ((nx - 0.61) ** 2 + (ny - 0.60) ** 2 <= 0.075 ** 2)
                or (0.435 <= nx <= 0.465 and 0.34 <= ny <= 0.66)
                or (0.655 <= nx <= 0.685 and 0.28 <= ny <= 0.60)
                or (0.435 <= nx <= 0.685 and 0.34 - (nx - 0.435) * 0.24 <= ny <= 0.39 - (nx - 0.435) * 0.24)
            )
            rgb = (255, 255, 255) if note and coverage else red
            row += bytes((*rgb, round(255 * coverage)))
        rows.append(bytes(row))

    raw = b"".join(rows)
    png = b"\x89PNG\r\n\x1a\n"
    png += _chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0))
    png += _chunk(b"IDAT", zlib.compress(raw, 9))
    png += _chunk(b"IEND", b"")

    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as handle:
        handle.write(png)


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    res = os.path.join(os.path.dirname(here), "app", "src", "main", "res")
    targets = [
        ("mipmap-mdpi", 48),
        ("mipmap-hdpi", 72),
        ("mipmap-xhdpi", 96),
        ("mipmap-xxhdpi", 144),
        ("mipmap-xxxhdpi", 192),
    ]
    for folder, size in targets:
        write_png(os.path.join(res, folder, "ic_launcher.png"), size, round_icon=False)
        write_png(os.path.join(res, folder, "ic_launcher_round.png"), size, round_icon=True)
    print("launcher icons written")


if __name__ == "__main__":
    main()
