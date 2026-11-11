"""Static checks for the actual embedded SVG inventory; no external packages."""
from pathlib import Path
import re
import unittest
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]


class IconTests(unittest.TestCase):
    def test_sprite_symbols_are_unique_and_safe(self):
        sprite = ET.parse(ROOT / "src/icons.svg").getroot()
        symbols = sprite.findall("{http://www.w3.org/2000/svg}symbol")
        names = [node.attrib["id"] for node in symbols]
        self.assertEqual(len(names), len(set(names)))
        self.assertEqual(len(names), 16)
        for symbol in symbols:
            self.assertEqual(symbol.attrib.get("viewBox"), "0 0 24 24")
        for node in sprite.iter():
            self.assertNotIn(node.tag.rsplit("}", 1)[-1], ("script", "foreignObject", "image"))
            self.assertFalse(any(key.lower().startswith("on") for key in node.attrib))

    def test_all_static_and_dynamic_icon_names_exist(self):
        names = {node.attrib["id"] for node in ET.parse(ROOT / "src/icons.svg").getroot()}
        page = (ROOT / "src/index.html").read_text()
        used = set(re.findall(r'/icons\.svg#([\w-]+)', page))
        used.update(re.findall(r"icon\('([\w-]+)'\)", page))
        used.update({"play", "pause", "queue", "check", "volume-off", "volume-low", "volume-high"})
        self.assertTrue(used <= names, used - names)

    def test_sprite_is_embedded_in_the_rust_server(self):
        source = (ROOT / "src/main.rs").read_text()
        self.assertIn('include_str!("icons.svg")', source)
        self.assertIn('.route("/icons.svg", get(icons_handler))', source)
        self.assertIn('"image/svg+xml"', source)


if __name__ == "__main__":
    unittest.main()
