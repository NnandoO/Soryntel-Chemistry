#!/usr/bin/env bash
set -euo pipefail

if [[ "${EUID}" -eq 0 ]]; then
    BIN_DIR="/usr/local/bin"
    APP_SHARE="/usr/local/share/applications"
    ICON_PNG_DIR="/usr/local/share/icons/hicolor/256x256/apps"
    ICON_SVG_DIR="/usr/local/share/icons/hicolor/scalable/apps"
    LIB_DIR="/usr/local/lib/soryntel-chemistry"
else
    BIN_DIR="${HOME}/.local/bin"
    APP_SHARE="${HOME}/.local/share/applications"
    ICON_PNG_DIR="${HOME}/.local/share/icons/hicolor/256x256/apps"
    ICON_SVG_DIR="${HOME}/.local/share/icons/hicolor/scalable/apps"
    LIB_DIR="${HOME}/.local/lib/soryntel-chemistry"
fi

rm -f "${BIN_DIR}/soryntel-desktop" \
      "${APP_SHARE}/soryntel-chemistry.desktop" \
      "${ICON_PNG_DIR}/soryntel-chemistry.png" \
      "${ICON_SVG_DIR}/soryntel-chemistry.svg"
rm -rf "${LIB_DIR}"

echo "✔ Soryntel Chemistry desinstalado."
