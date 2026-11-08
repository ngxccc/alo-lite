#!/usr/bin/env bash
set -euo pipefail

if [ "$(id -u)" -ne 0 ] && [ -f "/usr/local/bin/alolite" ] && command -v sudo >/dev/null 2>&1; then
    exec sudo bash "$0" "$@"
fi

rm -f /usr/local/bin/alolite \
      /usr/local/share/applications/alolite.desktop \
      /usr/local/share/icons/hicolor/256x256/apps/alolite.png \
      "$HOME/.local/bin/alolite" \
      "$HOME/.local/share/applications/alolite.desktop" \
      "$HOME/.local/share/icons/hicolor/256x256/apps/alolite.png"

echo "Đã gỡ cài đặt Alolite."
