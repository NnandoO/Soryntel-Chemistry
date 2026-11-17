#!/usr/bin/env bash
# Builds the release artifacts the website links to.
#
#   packaging/build-releases.sh [linux|windows|arch|all]   (default: all)
#
# The version comes from [workspace.package] in Cargo.toml; nothing else
# hard-codes it. Output goes to dist/ with these exact names:
#   Soryntel-Chemistry-<v>-linux-x86_64.tar.gz
#   Soryntel-Chemistry-<v>-linux-x86_64.run
#   Soryntel-Chemistry-<v>-win64-setup.exe
#   Soryntel-Chemistry-<v>-win64-portable.zip
#   soryntel-chemistry-<v>-1-x86_64.pkg.tar.zst
#
# The `arch` stage packages an already built target/release/soryntel-desktop,
# so it can run in an Arch container after the `linux` stage.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(awk '/^\[workspace.package\]/{f=1;next} /^\[/{f=0} f && /^version *=/{gsub(/[" ]/,"",$0); split($0,a,"="); print a[2]; exit}' "${ROOT_DIR}/Cargo.toml")"
[[ -n "${VERSION}" ]] || { echo "No se pudo leer la versión de Cargo.toml" >&2; exit 1; }
DIST_DIR="${DIST_DIR:-${ROOT_DIR}/dist}"
STAGE_DIR="$(mktemp -d)"
trap 'rm -rf "${STAGE_DIR}"' EXIT
TARGETS="${1:-all}"

mkdir -p "${DIST_DIR}"
cd "${ROOT_DIR}"
echo "==> Soryntel Chemistry ${VERSION} (${TARGETS})"

want() { [[ "${TARGETS}" == all || "${TARGETS}" == "$1" ]]; }

build_linux() {
    echo "==> Linux: compilando (x86_64-unknown-linux-gnu)..."
    cargo build --release --locked -p soryntel-desktop

    local name="Soryntel-Chemistry-${VERSION}-linux-x86_64"
    local stage="${STAGE_DIR}/${name}"
    mkdir -p "${stage}"
    cp target/release/soryntel-desktop "${stage}/soryntel-desktop"
    strip "${stage}/soryntel-desktop" || true
    cp packaging/linux/install.sh packaging/linux/uninstall.sh packaging/linux/soryntel-chemistry.desktop "${stage}/"
    cp packaging/soryntel-icon.png packaging/soryntel-icon.svg "${stage}/"
    chmod 755 "${stage}/soryntel-desktop" "${stage}/install.sh" "${stage}/uninstall.sh"

    echo "==> Linux: empaquetando .tar.gz y .run..."
    tar -C "${STAGE_DIR}" -czf "${DIST_DIR}/${name}.tar.gz" "${name}"

    local run="${DIST_DIR}/${name}.run"
    cat > "${run}" <<'RUNNER_HEADER'
#!/usr/bin/env bash
set -euo pipefail
TMP_EXTRACT="$(mktemp -d)"
trap 'rm -rf "${TMP_EXTRACT}"' EXIT
ARCHIVE_LINE=$(awk '/^__ARCHIVE_BELOW__$/ {print NR + 1; exit 0;}' "$0")
tail -n +"${ARCHIVE_LINE}" "$0" | tar -xz -C "${TMP_EXTRACT}"
bash "${TMP_EXTRACT}"/Soryntel-Chemistry-*-linux-x86_64/install.sh
exit 0
__ARCHIVE_BELOW__
RUNNER_HEADER
    cat "${DIST_DIR}/${name}.tar.gz" >> "${run}"
    chmod 755 "${run}"
}

build_windows() {
    echo "==> Windows: compilando (x86_64-pc-windows-gnu)..."
    cargo build --release --locked --target x86_64-pc-windows-gnu -p soryntel-desktop
    local exe="target/x86_64-pc-windows-gnu/release/soryntel-desktop.exe"
    if command -v x86_64-w64-mingw32-strip >/dev/null 2>&1; then
        x86_64-w64-mingw32-strip "${exe}" || true
    fi

    echo "==> Windows: instalador NSIS..."
    makensis -V2 -DAPP_VERSION="${VERSION}" packaging/windows/soryntel.nsi

    echo "==> Windows: ZIP portable..."
    local name="Soryntel-Chemistry-${VERSION}-win64"
    mkdir -p "${STAGE_DIR}/${name}/Databases"
    cp "${exe}" "${STAGE_DIR}/${name}/soryntel-desktop.exe"
    cp packaging/soryntel-icon.ico "${STAGE_DIR}/${name}/"
    (cd "${STAGE_DIR}" && zip -qr "${DIST_DIR}/${name}-portable.zip" "${name}")
}

build_arch() {
    echo "==> Arch Linux: paquete nativo (.pkg.tar.zst)..."
    [[ -x target/release/soryntel-desktop ]] || { echo "Falta target/release/soryntel-desktop: ejecute antes la etapa linux" >&2; exit 1; }
    local build="${STAGE_DIR}/archpkg"
    mkdir -p "${build}"
    sed "s/^pkgver=.*/pkgver=${VERSION}/" packaging/linux/arch/PKGBUILD > "${build}/PKGBUILD"
    SRCDEST="${ROOT_DIR}" PKGDEST="${DIST_DIR}" BUILDDIR="${STAGE_DIR}/arch_work" PKGEXT=".pkg.tar.zst" \
        bash -c "cd '${build}' && makepkg -f --noconfirm --nodeps"
}

want linux && build_linux
want windows && build_windows
want arch && build_arch

echo "==> Artefactos en ${DIST_DIR}:"
ls -lh "${DIST_DIR}"
