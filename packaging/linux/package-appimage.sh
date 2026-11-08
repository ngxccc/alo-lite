#!/usr/bin/env bash
# Package Alolite as a standalone AppImage
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

VERSION="${1:-$(cargo metadata --locked --no-deps --format-version 1 \
  | jq -r '.packages[] | select(.name == "alolite") | .version')}"

echo "=== Packaging .AppImage for Alolite v${VERSION} ==="
if [ ! -f target/release/alolite ]; then
  echo "Building release binary..."
  cargo build --release --locked --bin alolite
  strip target/release/alolite || true
fi

mkdir -p dist AppDir/usr/bin AppDir/usr/share/applications AppDir/usr/share/icons/hicolor/256x256/apps

cp target/release/alolite AppDir/usr/bin/alolite
cp packaging/linux/alolite.desktop AppDir/alolite.desktop
cp packaging/linux/alolite.desktop AppDir/usr/share/applications/alolite.desktop
cp src/assets/app_icon.png AppDir/alolite.png
cp src/assets/app_icon.png AppDir/.DirIcon
cp src/assets/app_icon.png AppDir/usr/share/icons/hicolor/256x256/apps/alolite.png

cat > AppDir/AppRun << 'EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "${0}")")"
export PATH="${HERE}/usr/bin:${PATH}"
exec "${HERE}/usr/bin/alolite" "$@"
EOF
chmod +x AppDir/AppRun

if ! command -v appimagetool >/dev/null 2>&1 && [ ! -f ./appimagetool ]; then
  curl -fsSL -o appimagetool "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage"
  chmod +x appimagetool
fi

TOOL="./appimagetool"
if command -v appimagetool >/dev/null 2>&1; then
  TOOL="appimagetool"
fi

ARCH=x86_64 "$TOOL" --appimage-extract-and-run AppDir "dist/alolite-${VERSION}-x86_64.AppImage"
chmod +x "dist/alolite-${VERSION}-x86_64.AppImage"
rm -rf AppDir
echo "✓ Created: dist/alolite-${VERSION}-x86_64.AppImage"
