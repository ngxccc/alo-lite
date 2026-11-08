#!/usr/bin/env bash
# Generates release notes automatically with categorized commits and package metadata
set -euo pipefail

TAG="${1:-$(git describe --tags --exact-match 2>/dev/null || echo "")}"
if [ -z "$TAG" ]; then
  echo "Usage: $0 <tag-name>" >&2
  exit 1
fi

PREV_TAG=$(git describe --tags --abbrev=0 "${TAG}^" 2>/dev/null || echo "")

if [ -n "$PREV_TAG" ]; then
  RANGE="${PREV_TAG}..${TAG}"
  CHANGELOG_LINK="https://github.com/ngxccc/alo-lite/compare/${PREV_TAG}...${TAG}"
else
  RANGE="${TAG}"
  CHANGELOG_LINK="https://github.com/ngxccc/alo-lite/commits/${TAG}"
fi

echo "### AloLite Release (${TAG})"
echo ""
echo "Lightweight, native messaging desktop client for Linux written in Rust (Zero Node.js, Zero Electron)."
echo ""
echo "#### Available Linux Packages:"
echo "- **Debian / Ubuntu / Linux Mint / Pop!_OS:** \`alolite_*_amd64.deb\`"
echo "- **Fedora / RHEL / Rocky / openSUSE:** \`alolite-*-1.x86_64.rpm\`"
echo "- **Universal Single-File Executable:** \`alolite-*-x86_64.AppImage\`"
echo "- **Portable Standalone Archive:** \`alolite-*-linux-x86_64.tar.gz\`"
echo ""
echo "#### Checksums:"
echo "Verify downloaded assets with \`sha256sum -c SHA256SUMS.txt\`."
echo ""
echo "---"
echo ""
echo "## What's Changed"
echo ""

declare -a FEATS=()
declare -a FIXES=()
declare -a REFACTORS=()
declare -a PERFS=()
declare -a DOCS=()
declare -a CHORES=()
declare -a OTHERS=()

while IFS= read -r line; do
  [ -z "$line" ] && continue
  hash=$(echo "$line" | cut -d' ' -f1)
  msg=$(echo "$line" | cut -d' ' -f2-)
  entry="- ${msg} (\`${hash}\`)"

  case "$msg" in
    feat*|Feat*)
      FEATS+=("$entry")
      ;;
    fix*|Fix*)
      FIXES+=("$entry")
      ;;
    refactor*|Refactor*)
      REFACTORS+=("$entry")
      ;;
    perf*|Perf*)
      PERFS+=("$entry")
      ;;
    docs*|Docs*)
      DOCS+=("$entry")
      ;;
    chore*|ci*|build*|test*|style*)
      CHORES+=("$entry")
      ;;
    *)
      OTHERS+=("$entry")
      ;;
  esac
done < <(git log "$RANGE" --no-merges --oneline)

if [ ${#FEATS[@]} -gt 0 ]; then
  echo "### 🚀 Features"
  printf '%s\n' "${FEATS[@]}"
  echo ""
fi
if [ ${#FIXES[@]} -gt 0 ]; then
  echo "### 🐛 Bug Fixes"
  printf '%s\n' "${FIXES[@]}"
  echo ""
fi
if [ ${#REFACTORS[@]} -gt 0 ]; then
  echo "### 🔨 Refactoring & Architecture"
  printf '%s\n' "${REFACTORS[@]}"
  echo ""
fi
if [ ${#PERFS[@]} -gt 0 ]; then
  echo "### ⚡ Performance"
  printf '%s\n' "${PERFS[@]}"
  echo ""
fi
if [ ${#DOCS[@]} -gt 0 ]; then
  echo "### 📝 Documentation"
  printf '%s\n' "${DOCS[@]}"
  echo ""
fi
if [ ${#CHORES[@]} -gt 0 ]; then
  echo "### 🔧 Maintenance & CI"
  printf '%s\n' "${CHORES[@]}"
  echo ""
fi
if [ ${#OTHERS[@]} -gt 0 ]; then
  echo "### Other Changes"
  printf '%s\n' "${OTHERS[@]}"
  echo ""
fi

echo "**Full Changelog**: ${CHANGELOG_LINK}"
