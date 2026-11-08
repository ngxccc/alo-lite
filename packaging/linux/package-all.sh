#!/usr/bin/env bash
# Build release binaries (CLI + Tauri Desktop) and generate Linux packages
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

VERSION="${1:-$(cargo metadata --locked --no-deps --format-version 1 \
  | jq -r '.packages[] | select(.name == "alolite-cli") | .version')}"

echo "=== [1/5] Building frontend and release binaries for AloLite v${VERSION} ==="
(cd apps/desktop && bun install --frozen-lockfile && bun run build)
cargo build --release --locked --bin alolite --bin alolite-desktop
strip target/release/alolite target/release/alolite-desktop || true

echo "=== [2/5] Packaging .deb ==="
./packaging/linux/package-deb.sh "$VERSION"

echo "=== [3/5] Packaging .rpm ==="
if command -v rpmbuild >/dev/null 2>&1; then
  ./packaging/linux/package-rpm.sh "$VERSION"
else
  echo "rpmbuild not found, skipping .rpm"
fi

echo "=== [4/5] Packaging .AppImage ==="
./packaging/linux/package-appimage.sh "$VERSION"

echo "=== [5/5] Packaging .tar.gz ==="
./packaging/linux/package-tarball.sh "$VERSION"

echo "=== All Linux packages created in dist/ ==="
ls -lh dist/
