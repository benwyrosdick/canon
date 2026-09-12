#!/bin/bash
# Build a distro-agnostic Linux tarball. One glibc binary covers Arch, Fedora, Ubuntu, etc.
set -euo pipefail

if [[ "$(uname -s)" != Linux ]]; then
    printf '%s\n' 'This packaging command must run on Linux.' >&2
    exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROFILE=release
case "${1:-}" in
    '') ;;
    --debug) PROFILE=debug ;;
    *) printf 'Usage: bash scripts/build-linux.sh [--debug]\n' >&2; exit 2 ;;
esac

command -v cargo >/dev/null || { printf 'Missing tool: cargo\n' >&2; exit 1; }

cd "$ROOT"
VERSION="$(cargo pkgid --manifest-path "$ROOT/Cargo.toml")"
VERSION="${VERSION##*#}"
VERSION="${VERSION##*@}"
ARCH="$(uname -m)"
NAME="3D-Canon-${VERSION}-linux-${ARCH}"
DIST="$ROOT/dist/linux"
STAGE="$DIST/$NAME"
rm -rf "$STAGE"
mkdir -p "$STAGE"

CARGO_ARGS=(build --locked --bin canon-3d --features game --target-dir "$ROOT/target")
if [[ "$PROFILE" == release ]]; then CARGO_ARGS+=(--release); fi
cargo "${CARGO_ARGS[@]}"

install -m755 "$ROOT/target/$PROFILE/canon-3d" "$STAGE/3d-canon"
install -m644 "$ROOT/packaging/3d-canon.desktop" "$STAGE/3d-canon.desktop"
install -m644 "$ROOT/packaging/icon.svg" "$STAGE/3d-canon.svg"

cat > "$STAGE/README.txt" <<EOF
3D Canon ${VERSION} (${ARCH} Linux)

This is a generic GNU/Linux build (dynamic glibc). It is meant to run on
Arch, Fedora, Debian, Ubuntu, and similar distributions without a per-distro
package. You need a Vulkan-capable GPU and these libraries:

  libasound, libxkbcommon, wayland, libX11, vulkan loader, libudev

Arch:     sudo pacman -S alsa-lib libxkbcommon vulkan-icd-loader wayland libx11
Debian:   sudo apt install libasound2t64 libxkbcommon0 libwayland-client0 libx11-6 libvulkan1 libudev1
Fedora:   sudo dnf install alsa-lib libxkbcommon wayland-devel libX11 vulkan-loader

Run:
  ./3d-canon
  ./3d-canon --local
  ./3d-canon --relay=HOST:3478

Install for the current user (menu entry + icon):
  bash install.sh
EOF

cat > "$STAGE/install.sh" <<'EOF'
#!/bin/bash
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
BIN="${XDG_BIN_HOME:-$HOME/.local/bin}"
APP="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
ICON="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/scalable/apps"
mkdir -p "$BIN" "$APP" "$ICON"
install -m755 "$HERE/3d-canon" "$BIN/3d-canon"
install -m644 "$HERE/3d-canon.svg" "$ICON/3d-canon.svg"
sed "s|^Exec=.*|Exec=$BIN/3d-canon|" "$HERE/3d-canon.desktop" > "$APP/3d-canon.desktop"
chmod 644 "$APP/3d-canon.desktop"
if command -v update-desktop-database >/dev/null; then
    update-desktop-database "$APP" >/dev/null 2>&1 || true
fi
printf 'Installed 3D Canon to %s\nDesktop entry: %s\n' "$BIN/3d-canon" "$APP/3d-canon.desktop"
EOF
chmod 755 "$STAGE/install.sh"

mkdir -p "$DIST"
tar -C "$DIST" -czf "$DIST/${NAME}.tar.gz" "$NAME"
printf '\nBuilt %s (%s):\n  %s\n' "$ARCH" "$PROFILE" "$DIST/${NAME}.tar.gz"
printf '%s\n' 'Extract the archive and run ./3d-canon, or bash install.sh for a menu entry.'
