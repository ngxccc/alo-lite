#!/usr/bin/env bash
# Build release binary and generate all Linux packages (.deb, .rpm, .AppImage, .tar.gz)
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

VERSION="${1:-$(cargo metadata --locked --no-deps --format-version 1 \
  | jq -r '.packages[] | select(.name == "alolite") | .version')}"

echo "=== [1/5] Building release binary for Alolite v${VERSION} ==="
cargo build --release --locked --bin alolite
strip target/release/alolite || true

echo "=== [2/5] Packaging .deb ==="
./packaging/linux/package-deb.sh "$VERSION"

echo "=== [3/5] Packaging .rpm ==="
if command -v rpmbuild >/dev/null 2>&1; then
  ./packaging/linux/package-rpm.sh "$VERSION"
else
  echo "⚠️ rpmbuild not found, skipping .rpm"
fi

echo "=== [4/5] Packaging .AppImage ==="
./packaging/linux/package-appimage.sh "$VERSION"

echo "=== [5/5] Packaging .tar.gz ==="
./packaging/linux/package-tarball.sh "$VERSION"

echo "=== All Linux packages created in dist/ ==="
ls -lh dist/
