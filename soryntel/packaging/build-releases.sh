#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="0.1.0"
DIST_DIR="${ROOT_DIR}/dist"
STAGE_DIR="/tmp/soryntel_pkg_stage"

rm -rf "${STAGE_DIR}"
mkdir -p "${STAGE_DIR}" "${DIST_DIR}"

echo "==> [1/5] Compilando Release para Linux (x86_64-unknown-linux-gnu)..."
cd "${ROOT_DIR}"
cargo build --release -p soryntel-desktop

echo "==> [2/5] Compilando Release para Windows (x86_64-pc-windows-gnu)..."
cargo build --release --target x86_64-pc-windows-gnu -p soryntel-desktop

# Strip Windows exe cleanly into staging copy
WIN_STAGE="${STAGE_DIR}/Soryntel-Chemistry-${VERSION}-win64"
mkdir -p "${WIN_STAGE}/Databases"
cp "${ROOT_DIR}/target/x86_64-pc-windows-gnu/release/soryntel-desktop.exe" "${WIN_STAGE}/soryntel-desktop.exe"
if command -v x86_64-w64-mingw32-strip >/dev/null 2>&1; then
    x86_64-w64-mingw32-strip "${WIN_STAGE}/soryntel-desktop.exe" || true
fi
cp "${ROOT_DIR}/packaging/soryntel-icon.ico" "${WIN_STAGE}/soryntel-icon.ico"

echo "==> [3/5] Empaquetando Instalador Windows (.exe NSIS) y ZIP portable..."
cp "${WIN_STAGE}/soryntel-desktop.exe" "${ROOT_DIR}/target/x86_64-pc-windows-gnu/release/soryntel-desktop.exe"
makensis -V2 "${ROOT_DIR}/packaging/windows/soryntel.nsi"
(cd "${STAGE_DIR}" && zip -qr "${DIST_DIR}/Soryntel-Chemistry-${VERSION}-win64-portable.zip" "Soryntel-Chemistry-${VERSION}-win64")

echo "==> [4/5] Empaquetando Instalador Universal Linux (.tar.gz + .run auto-extraíble)..."
LIN_STAGE="${STAGE_DIR}/Soryntel-Chemistry-${VERSION}-linux-x86_64"
mkdir -p "${LIN_STAGE}"
cp "${ROOT_DIR}/target/release/soryntel-desktop" "${LIN_STAGE}/soryntel-desktop"
strip "${LIN_STAGE}/soryntel-desktop" || true
cp "${ROOT_DIR}/packaging/linux/install.sh" "${LIN_STAGE}/install.sh"
cp "${ROOT_DIR}/packaging/linux/uninstall.sh" "${LIN_STAGE}/uninstall.sh"
cp "${ROOT_DIR}/packaging/linux/soryntel-chemistry.desktop" "${LIN_STAGE}/soryntel-chemistry.desktop"
cp "${ROOT_DIR}/packaging/soryntel-icon.png" "${LIN_STAGE}/soryntel-icon.png"
cp "${ROOT_DIR}/packaging/soryntel-icon.svg" "${LIN_STAGE}/soryntel-icon.svg"
chmod 755 "${LIN_STAGE}/soryntel-desktop" "${LIN_STAGE}/install.sh" "${LIN_STAGE}/uninstall.sh"

tar -C "${STAGE_DIR}" -czf "${DIST_DIR}/Soryntel-Chemistry-${VERSION}-linux-x86_64.tar.gz" "Soryntel-Chemistry-${VERSION}-linux-x86_64"

RUN_FILE="${DIST_DIR}/Soryntel-Chemistry-${VERSION}-linux-x86_64.run"
cat > "${RUN_FILE}" <<'RUNNER_HEADER'
#!/usr/bin/env bash
set -euo pipefail
TMP_EXTRACT="$(mktemp -d)"
trap 'rm -rf "${TMP_EXTRACT}"' EXIT
ARCHIVE_LINE=$(awk '/^__ARCHIVE_BELOW__$/ {print NR + 1; exit 0;}' "$0")
tail -n +"${ARCHIVE_LINE}" "$0" | tar -xz -C "${TMP_EXTRACT}"
bash "${TMP_EXTRACT}/Soryntel-Chemistry-0.1.0-linux-x86_64/install.sh"
exit 0
__ARCHIVE_BELOW__
RUNNER_HEADER
cat "${DIST_DIR}/Soryntel-Chemistry-${VERSION}-linux-x86_64.tar.gz" >> "${RUN_FILE}"
chmod 755 "${RUN_FILE}"

echo "==> [5/5] Empaquetando Paquete Nativo Arch / CachyOS (.pkg.tar.zst)..."
ARCH_BUILD_DIR="${STAGE_DIR}/archpkg"
mkdir -p "${ARCH_BUILD_DIR}"
cp "${ROOT_DIR}/packaging/linux/arch/PKGBUILD" "${ARCH_BUILD_DIR}/PKGBUILD"
SRCDEST="${ROOT_DIR}" PKGDEST="${DIST_DIR}" BUILDDIR="${STAGE_DIR}/arch_work" \
    bash -c "cd '${ARCH_BUILD_DIR}' && startdir='${ROOT_DIR}/packaging/linux/arch' makepkg -f --noconfirm"

rm -rf "${STAGE_DIR}"
echo "==> Empaquetado finalizado en ${DIST_DIR}:"
ls -lh "${DIST_DIR}"
