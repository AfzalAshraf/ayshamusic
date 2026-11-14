"""Generate the launcher icons (red disc + music note) with the Python stdlib only.

Run from anywhere: python3 android-app/tools/generate_launcher_icons.py
"""
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
    bg = (3, 3, 3, 255)
    red = (255, 0, 0, 255)
    white = (255, 255, 255, 255)
    cx = cy = (size - 1) / 2.0
    radius = size * 0.30
    half = size * 0.30
    note_r = size * 0.075
    stem_w = max(1.0, size * 0.018)

    rows = []
    for y in range(size):
        row = bytearray([0])  # filter type 0
        for x in range(size):
            dx, dy = x - cx, y - cy
            if round_icon:
                inside = (dx * dx + dy * dy) <= radius * radius
            else:
                inside = abs(dx) <= half and abs(dy) <= half
            color = bg
            if inside:
                color = red
                # two note heads
                if ((x - (cx - size * 0.06)) ** 2 + (y - (cy + size * 0.12)) ** 2) <= note_r * note_r:
                    color = white
                elif ((x - (cx + size * 0.10)) ** 2 + (y - (cy + size * 0.06)) ** 2) <= note_r * note_r:
                    color = white
                # stems
                elif abs(x - (cx - size * 0.06)) <= stem_w and cy - size * 0.10 <= y <= cy + size * 0.12:
                    color = white
                elif abs(x - (cx + size * 0.10)) <= stem_w and cy - size * 0.16 <= y <= cy + size * 0.06:
                    color = white
                # beam
                elif cx - size * 0.06 <= x <= cx + size * 0.10 and abs(y - (cy - size * 0.14)) <= size * 0.02:
                    color = white
            row += bytes(color)
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
