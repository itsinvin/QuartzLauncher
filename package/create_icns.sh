#!/bin/sh

set -e
cd "$(dirname "$0")/.."
python3 scripts/generate_icons.py
cd package
if command -v iconutil >/dev/null 2>&1; then
    iconutil -c icns mac.iconset
    rm -rf mac.iconset
    echo "Generated package/mac.icns"
else
    echo "iconutil not found; mac.iconset left for manual conversion on macOS"
fi
