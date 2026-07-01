#!/usr/bin/env python3
"""Generate launcher icons from the Minecraft Block of Quartz texture.

Source: https://static.wikia.nocookie.net/minecraft_gamepedia/images/7/7d/Block_of_Quartz_JE1_BE1.png
Stored at assets/images/quartz_block.png
"""

from __future__ import annotations

import os
import sys

try:
    from PIL import Image
except ImportError:
    print("Pillow is required: pip install pillow", file=sys.stderr)
    sys.exit(1)

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SOURCE = os.path.join(ROOT, "assets", "images", "quartz_block.png")
ICON_SIZES = [16, 32, 48, 256]
MAC_ICONSET = [
    ("icon_16x16.png", 16),
    ("icon_16x16@2x.png", 32),
    ("icon_32x32.png", 32),
    ("icon_32x32@2x.png", 64),
    ("icon_128x128.png", 128),
    ("icon_128x128@2x.png", 256),
    ("icon_256x256.png", 256),
    ("icon_256x256@2x.png", 512),
    ("icon_512x512.png", 512),
    ("icon_512x512@2x.png", 1024),
]


def main() -> None:
    if not os.path.isfile(SOURCE):
        print(f"Missing source image: {SOURCE}", file=sys.stderr)
        sys.exit(1)

    src = Image.open(SOURCE).convert("RGBA")
    icons_dir = os.path.join(ROOT, "package", "windows_icons")
    os.makedirs(icons_dir, exist_ok=True)

    for size in ICON_SIZES:
        out = src.resize((size, size), Image.NEAREST)
        out.save(os.path.join(icons_dir, f"icon_{size}x{size}.png"))

    ico_path = os.path.join(ROOT, "package", "windows.ico")
    src.save(ico_path, format="ICO", sizes=[(size, size) for size in ICON_SIZES])

    mac_iconset = os.path.join(ROOT, "package", "mac.iconset")
    os.makedirs(mac_iconset, exist_ok=True)
    for filename, size in MAC_ICONSET:
        out = src.resize((size, size), Image.NEAREST)
        out.save(os.path.join(mac_iconset, filename))

    print(f"Generated {ico_path}, {len(ICON_SIZES)} Windows PNGs, and macOS iconset from {SOURCE}")


if __name__ == "__main__":
    main()
