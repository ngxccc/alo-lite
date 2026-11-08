#!/usr/bin/env bash
# Package Alolite as a standalone portable tarball (.tar.gz)
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

VERSION="${1:-$(cargo metadata --locked --no-deps --format-version 1 \
  | jq -r '.packages[] | select(.name == "alolite") | .version')}"

echo "=== Packaging .tar.gz for Alolite v${VERSION} ==="
if [ ! -f target/release/alolite ]; then
  echo "Building release binary..."
  cargo build --release --locked --bin alolite
  strip target/release/alolite || true
fi

mkdir -p dist

STAGE="alolite-${VERSION}-linux-x86_64"
STAGE_DIR="$(mktemp -d)"
trap 'rm -rf "$STAGE_DIR"' EXIT

mkdir -p "$STAGE_DIR/$STAGE"
cp target/release/alolite "$STAGE_DIR/$STAGE/alolite"
cp packaging/linux/install.sh "$STAGE_DIR/$STAGE/install.sh"
cp packaging/linux/uninstall.sh "$STAGE_DIR/$STAGE/uninstall.sh"
cp packaging/linux/alolite.desktop "$STAGE_DIR/$STAGE/alolite.desktop"
cp src/assets/app_icon.png "$STAGE_DIR/$STAGE/alolite.png"
cp README.md LICENSE-MIT LICENSE-APACHE "$STAGE_DIR/$STAGE/"
chmod +x "$STAGE_DIR/$STAGE/alolite" "$STAGE_DIR/$STAGE/install.sh" "$STAGE_DIR/$STAGE/uninstall.sh"

tar -C "$STAGE_DIR" -czf "dist/${STAGE}.tar.gz" "$STAGE"
echo "✓ Created: dist/${STAGE}.tar.gz"
