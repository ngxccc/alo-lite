#!/usr/bin/env bash
# scripts/detect-and-tag-release.sh - Detect release commits merged into main and create git tags
set -euo pipefail

REF_TYPE="${1:-${GITHUB_REF_TYPE:-}}"
REF_NAME="${2:-${GITHUB_REF_NAME:-}}"
EVENT_NAME="${3:-${GITHUB_EVENT_NAME:-}}"
MANUAL_TAG="${4:-}"

OUTPUT_FILE="${GITHUB_OUTPUT:-/dev/null}"

if [[ "$REF_TYPE" == "tag" ]]; then
  echo "Direct tag push detected: $REF_NAME"
  echo "should_release=true" >> "$OUTPUT_FILE"
  echo "tag_name=$REF_NAME" >> "$OUTPUT_FILE"
  exit 0
fi

if [[ "$EVENT_NAME" == "workflow_dispatch" && -n "$MANUAL_TAG" ]]; then
  echo "Manual workflow dispatch release: $MANUAL_TAG"
  echo "should_release=true" >> "$OUTPUT_FILE"
  echo "tag_name=$MANUAL_TAG" >> "$OUTPUT_FILE"
  exit 0
fi

COMMIT_MSG="$(git log -1 --pretty=%B)"
echo "Evaluating recent commit: $COMMIT_MSG"

if [[ "$COMMIT_MSG" =~ chore\(release\):[[:space:]]*v?([0-9]+\.[0-9]+\.[0-9]+) ]]; then
  TAG="v${BASH_REMATCH[1]}"
  echo "Matched release commit for tag: $TAG"
  git fetch --tags origin 2>/dev/null || true
  if git rev-parse "$TAG" >/dev/null 2>&1; then
    echo "Tag $TAG already exists, skipping release."
    echo "should_release=false" >> "$OUTPUT_FILE"
  else
    git config user.name "github-actions[bot]"
    git config user.email "github-actions[bot]@users.noreply.github.com"
    git tag -a "$TAG" -m "AloLite Release $TAG"
    git push origin "$TAG"
    echo "Created and pushed tag $TAG"
    echo "should_release=true" >> "$OUTPUT_FILE"
    echo "tag_name=$TAG" >> "$OUTPUT_FILE"
  fi
else
  echo "Not a release commit, skipping release workflow."
  echo "should_release=false" >> "$OUTPUT_FILE"
fi
