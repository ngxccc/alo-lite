#!/usr/bin/env bash
# scripts/setup-git-hooks.sh - Configure Git to use tracked .githooks directory
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

git config core.hooksPath .githooks
chmod +x .githooks/*

echo "✅ Git hooks configured to use: .githooks"
