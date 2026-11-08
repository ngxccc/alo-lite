#!/usr/bin/env bash
# Package AloLite as a Debian package (.deb)
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

VERSION="${1:-$(cargo metadata --locked --no-deps --format-version 1 \
  | jq -r '.packages[] | select(.name == "alolite-cli") | .version')}"

echo "=== Packaging .deb for AloLite v${VERSION} ==="
if [ ! -f target/release/alolite ] || [ ! -f target/release/alolite-desktop ]; then
  (cd apps/desktop && bun install --frozen-lockfile && bun run build)
  cargo build --release --locked --bin alolite --bin alolite-desktop
  strip target/release/alolite target/release/alolite-desktop || true
fi

mkdir -p dist pkg_deb/DEBIAN pkg_deb/usr/bin pkg_deb/usr/share/applications pkg_deb/usr/share/icons/hicolor/256x256/apps

cp target/release/alolite pkg_deb/usr/bin/alolite
cp target/release/alolite-desktop pkg_deb/usr/bin/alolite-desktop
cp packaging/linux/alolite.desktop pkg_deb/usr/share/applications/alolite.desktop
cp crates/alolite-core/assets/app_icon.png pkg_deb/usr/share/icons/hicolor/256x256/apps/alolite.png

cat > pkg_deb/DEBIAN/control << EOF
Package: alolite
Version: ${VERSION}
Section: net
Priority: optional
Architecture: amd64
Depends: libc6 (>= 2.31), libgtk-3-0, libwebkit2gtk-4.1-0, libsoup-3.0-0
Recommends: zenity, xclip, wl-clipboard, fonts-noto-core
Maintainer: ngxc <ngoc.dev@gmail.com>
Description: Lightweight native desktop messaging client written in Rust + Tauri
EOF

if command -v dpkg-deb >/dev/null 2>&1; then
  dpkg-deb --build --root-owner-group pkg_deb "dist/alolite_${VERSION}_amd64.deb"
  echo "Created: dist/alolite_${VERSION}_amd64.deb"
else
  echo "dpkg-deb not found on this system."
fi
rm -rf pkg_deb
