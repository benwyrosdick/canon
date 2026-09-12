#!/bin/bash
# Build a self-contained Finder application and a drag-to-install disk image.
set -euo pipefail

if [[ "$(uname -s)" != Darwin ]]; then
    printf '%s\n' 'This packaging command requires macOS and Xcode Command Line Tools.' >&2
    exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROFILE=release
case "${1:-}" in
    '') ;;
    --debug) PROFILE=debug ;;
    *) printf 'Usage: bash scripts/build-macos.sh [--debug]\n' >&2; exit 2 ;;
esac
for tool in cargo swift iconutil codesign hdiutil plutil; do
    command -v "$tool" >/dev/null || { printf 'Missing tool: %s\n' "$tool" >&2; exit 1; }
done

cd "$ROOT"
DIST="$ROOT/dist/macos"
mkdir -p "$DIST"
WORK="$(mktemp -d "$DIST/.package.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT
APP="$WORK/3D Canon.app"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

# Explicit target-dir makes the artifact path independent of the caller's Cargo config.
CARGO_ARGS=(build --locked --bin canon-3d --features game --target-dir "$ROOT/target")
if [[ "$PROFILE" == release ]]; then CARGO_ARGS+=(--release); fi
cargo "${CARGO_ARGS[@]}"
cp "$ROOT/target/$PROFILE/canon-3d" "$APP/Contents/MacOS/3D Canon"
chmod 755 "$APP/Contents/MacOS/3D Canon"

swift "$ROOT/scripts/macos-icon.swift" "$WORK/Canon.iconset"
iconutil --convert icns "$WORK/Canon.iconset" --output "$APP/Contents/Resources/Canon.icns"
cp "$WORK/Canon.iconset/icon_512x512@2x.png" "$DIST/3D Canon.png"

# Read the version from Cargo instead of maintaining a separate bundle version.
VERSION="$(cargo pkgid --manifest-path "$ROOT/Cargo.toml")"
VERSION="${VERSION##*#}"
VERSION="${VERSION##*@}"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key><string>en</string>
    <key>CFBundleExecutable</key><string>3D Canon</string>
    <key>CFBundleIdentifier</key><string>com.canon3d.game</string>
    <key>CFBundleName</key><string>3D Canon</string>
    <key>CFBundleDisplayName</key><string>3D Canon</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$VERSION</string>
    <key>CFBundleVersion</key><string>$VERSION</string>
    <key>CFBundleIconFile</key><string>Canon.icns</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>NSPrincipalClass</key><string>NSApplication</string>
</dict>
</plist>
PLIST
plutil -lint "$APP/Contents/Info.plist"

# Local builds get an ad-hoc signature. A Developer ID can be supplied for distribution.
IDENTITY="${MACOS_SIGN_IDENTITY:--}"
if [[ "$IDENTITY" == '-' ]]; then
    codesign --force --sign - "$APP"
else
    codesign --force --sign "$IDENTITY" --options runtime --timestamp "$APP"
fi
codesign --verify --deep --strict "$APP"

# ditto preserves the bundle layout, executable permissions, and signature.
rm -rf "$DIST/3D Canon.app"
ditto "$APP" "$DIST/3D Canon.app"
mkdir -p "$WORK/disk"
ditto "$APP" "$WORK/disk/3D Canon.app"
ln -s /Applications "$WORK/disk/Applications"
hdiutil create -volname '3D Canon' -srcfolder "$WORK/disk" -format UDZO \
    -ov "$DIST/3D Canon.dmg"

printf '\nBuilt for %s (%s):\n  %s\n  %s\n' "$(uname -m)" "$PROFILE" \
    "$DIST/3D Canon.app" "$DIST/3D Canon.dmg"
printf '%s\n' 'Open the DMG, then drag 3D Canon into Applications.'
