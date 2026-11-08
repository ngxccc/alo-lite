#!/usr/bin/env bash
# Build and package AloLite release bundle for Linux (.tar.gz).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

exec ./packaging/linux/package-tarball.sh "$@"
