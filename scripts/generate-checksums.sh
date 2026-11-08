#!/usr/bin/env bash
# Generate SHA256SUMS.txt for release assets
set -euo pipefail

TARGET_DIR="${1:-dist}"
cd "$TARGET_DIR"

echo "=== Generating SHA256SUMS.txt ==="
sha256sum * > SHA256SUMS.txt 2>/dev/null || true
cat SHA256SUMS.txt
