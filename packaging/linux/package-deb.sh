#!/usr/bin/env bash
# Package Alolite as a Debian package (.deb)
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

VERSION="${1:-$(cargo metadata --locked --no-deps --format-version 1 \
  | jq -r '.packages[] | select(.name == "alolite") | .version')}"

echo "=== Packaging .deb for Alolite v${VERSION} ==="
if [ ! -f target/release/alolite ]; then
  echo "Building release binary..."
  cargo build --release --locked --bin alolite
  strip target/release/alolite || true
fi

mkdir -p dist pkg_deb/DEBIAN pkg_deb/usr/bin pkg_deb/usr/share/applications pkg_deb/usr/share/icons/hicolor/256x256/apps

cp target/release/alolite pkg_deb/usr/bin/alolite
cp packaging/linux/alolite.desktop pkg_deb/usr/share/applications/alolite.desktop
cp src/assets/app_icon.png pkg_deb/usr/share/icons/hicolor/256x256/apps/alolite.png

cat > pkg_deb/DEBIAN/control << EOF
Package: alolite
Version: ${VERSION}
Section: net
Priority: optional
Architecture: amd64
Depends: libc6 (>= 2.31), libx11-6, libxcb1, libxkbcommon0, libgl1
Recommends: zenity, xclip, wl-clipboard, fonts-noto-core
Maintainer: ngxc <ngoc.dev@gmail.com>
Description: Lightweight native desktop messaging client written in Rust (zero Node.js, zero Electron)
EOF

if command -v dpkg-deb >/dev/null 2>&1; then
  dpkg-deb --build --root-owner-group pkg_deb "dist/alolite_${VERSION}_amd64.deb"
  echo "✓ Created: dist/alolite_${VERSION}_amd64.deb"
else
  echo "⚠️ dpkg-deb not found on this system. On Ubuntu/Debian, install with 'apt install dpkg'."
fi
rm -rf pkg_deb
