#!/usr/bin/env bash
# Rebuilds the app icon from its two sources (D-026):
#   src-tauri/icons/AppIcon.icon     — Icon Composer file (Liquid Glass, macOS 26+) → Assets.car
#   src-tauri/icons/AppIcon-flat.svg — flat twin for macOS 14–15 → icon.icns + PNGs
# Edit AppIcon.icon in Icon Composer (inside Xcode) or the SVGs, then run this. Needs Xcode
# (first run: `xcodebuild -runFirstLaunch`).
set -euo pipefail
cd "$(dirname "$0")/.."
icons="$PWD/src-tauri/icons" # absolute: actool does not resolve relative paths
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

xcrun actool "$icons/AppIcon.icon" --compile "$tmp" --app-icon AppIcon --platform macosx \
  --minimum-deployment-target 14.2 --output-partial-info-plist "$tmp/partial.plist" >/dev/null
cp "$tmp/Assets.car" "$icons/Assets.car"

swift scripts/svg2png.swift "$icons/AppIcon-flat.svg" "$tmp/icon-1024.png"
pnpm tauri icon "$tmp/icon-1024.png" --output "$tmp/set" >/dev/null
for f in 32x32.png 128x128.png 128x128@2x.png icon.icns icon.png; do cp "$tmp/set/$f" "$icons/$f"; done
echo "icon rebuilt: $icons/{Assets.car,icon.icns,*.png}"
