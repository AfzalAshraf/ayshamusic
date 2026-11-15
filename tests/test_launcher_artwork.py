"""Check supplied launcher PNG resources without build-time Pillow dependency."""
from pathlib import Path
import struct
import unittest
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
RES = ROOT / "android-app/app/src/main/res"
ANDROID = "{http://schemas.android.com/apk/res/android}"


def png_header(path):
    raw = path.read_bytes()
    assert raw[:8] == b"\x89PNG\r\n\x1a\n"
    assert raw[12:16] == b"IHDR"
    return struct.unpack(">IIBBBBB", raw[16:29])


class LauncherArtworkTests(unittest.TestCase):
    def test_user_master_is_rgba_and_cropped_square(self):
        width, height, depth, color, *_ = png_header(ROOT / "android-app/artwork/aysha-launcher.png")
        self.assertEqual((width, height), (591, 591))
        self.assertEqual((depth, color), (8, 6))

    def test_all_legacy_densities_are_real_pngs_with_alpha(self):
        for density, size in [("mdpi", 48), ("hdpi", 72), ("xhdpi", 96), ("xxhdpi", 144), ("xxxhdpi", 192)]:
            for name in ["ic_launcher", "ic_launcher_round"]:
                with self.subTest(density=density, name=name):
                    header = png_header(RES / f"mipmap-{density}/{name}.png")
                    self.assertEqual(header[:4], (size, size, 8, 6))

    def test_adaptive_launchers_reference_supplied_raster_and_dark_background(self):
        self.assertEqual(png_header(RES / "drawable-nodpi/ic_launcher_artwork.png")[:4], (432, 432, 8, 6))
        for name in ["ic_launcher", "ic_launcher_round"]:
            icon = ET.parse(RES / f"mipmap-anydpi-v26/{name}.xml").getroot()
            self.assertEqual(icon.tag, "adaptive-icon")
            self.assertEqual(icon.find("foreground").attrib[ANDROID + "drawable"], "@drawable/ic_launcher_artwork")
            self.assertEqual(icon.find("background").attrib[ANDROID + "drawable"], "@color/launcher_background")
        colors = {c.attrib["name"]: c.text for c in ET.parse(RES / "values/colors.xml").getroot()}
        self.assertEqual(colors["launcher_background"], "#12161B")
        self.assertFalse((RES / "drawable/ic_launcher_foreground.xml").exists())


if __name__ == "__main__":
    unittest.main()
