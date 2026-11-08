#!/usr/bin/env bash
# Build and package Alolite standalone bundle for Linux (.tar.gz).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

VER="$(cargo metadata --locked --no-deps --format-version 1 \
  | python3 -c 'import json,sys; d=json.load(sys.stdin); print(next(p["version"] for p in d["packages"] if p["name"]=="alolite"))')"

echo "=== Building Alolite Linux release v${VER} ==="
mkdir -p dist

cargo build --release --locked --bin alolite
strip target/release/alolite || true

STAGE="alolite-${VER}-linux-x86_64"
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
echo "=== Successfully built dist/${STAGE}.tar.gz ==="
