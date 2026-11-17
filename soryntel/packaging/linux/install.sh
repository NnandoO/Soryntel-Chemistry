#!/usr/bin/env bash
set -euo pipefail

APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

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

echo "Instalando Soryntel Chemistry en ${BIN_DIR}..."
mkdir -p "${BIN_DIR}" "${APP_SHARE}" "${ICON_PNG_DIR}" "${ICON_SVG_DIR}" "${LIB_DIR}/Databases"

install -m 755 "${APP_DIR}/soryntel-desktop" "${LIB_DIR}/soryntel-desktop"
install -m 644 "${APP_DIR}/soryntel-icon.png" "${ICON_PNG_DIR}/soryntel-chemistry.png"
install -m 644 "${APP_DIR}/soryntel-icon.svg" "${ICON_SVG_DIR}/soryntel-chemistry.svg"

cat > "${BIN_DIR}/soryntel-desktop" <<EOF
#!/usr/bin/env bash
exec "${LIB_DIR}/soryntel-desktop" "\$@"
EOF
chmod 755 "${BIN_DIR}/soryntel-desktop"

sed "s|^Exec=.*|Exec=${BIN_DIR}/soryntel-desktop|" \
    "${APP_DIR}/soryntel-chemistry.desktop" > "${APP_SHARE}/soryntel-chemistry.desktop"
chmod 644 "${APP_SHARE}/soryntel-chemistry.desktop"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "${APP_SHARE}" >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$(dirname "$(dirname "${ICON_PNG_DIR}")")" >/dev/null 2>&1 || true
fi

echo "✔ Soryntel Chemistry instalado correctamente."
echo "  Ejecutable: ${BIN_DIR}/soryntel-desktop"
echo "  Lanzador:   ${APP_SHARE}/soryntel-chemistry.desktop"
