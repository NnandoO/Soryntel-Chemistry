#!/usr/bin/env bash
# Script para compilar CMA Chemistry para Windows
# Requiere: mingw-w64-gcc (x86_64-w64-mingw32-g++)
#
# Instalación en Arch/CachyOS:
#   sudo pacman -S mingw-w64-gcc mingw-w64-winpthreads
#
# Instalación en Ubuntu/Debian:
#   sudo apt install g++-mingw-w64-x86-64

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BUILD_DIR="${SCRIPT_DIR}/build_win"

# Descargar GLFW y FreeType para Windows si no existen
DEPS_DIR="${SCRIPT_DIR}/deps_win"
GLFW_DIR="${DEPS_DIR}/glfw"
FREETYPE_DIR="${DEPS_DIR}/freetype"

mkdir -p "${DEPS_DIR}"

if [ ! -d "${GLFW_DIR}" ]; then
    if [ ! -d "${DEPS_DIR}/glfw_src" ]; then
        echo "Descargando GLFW para Windows..."
        cd "${DEPS_DIR}"
        git clone --depth 1 --branch 3.4 https://github.com/glfw/glfw.git glfw_src
    else
        echo "GLFW source ya existe, compilando..."
    fi
    mkdir -p "${DEPS_DIR}/glfw_src/build_win"
    cd "${DEPS_DIR}/glfw_src/build_win"
    x86_64-w64-mingw32-cmake .. \
        -DCMAKE_INSTALL_PREFIX="${GLFW_DIR}" \
        -DGLFW_BUILD_EXAMPLES=OFF \
        -DGLFW_BUILD_TESTS=OFF \
        -DGLFW_BUILD_DOCS=OFF
    make -j$(nproc) && make install
    cd "${DEPS_DIR}"
fi

# Configurar y compilar
mkdir -p "${BUILD_DIR}"
cd "${BUILD_DIR}"

x86_64-w64-mingw32-cmake "${SCRIPT_DIR}" \
    -DCMAKE_TOOLCHAIN_FILE="${SCRIPT_DIR}/cmake/x86_64-w64-mingw32.cmake" \
    -DGLFW3_INCLUDE_DIR="${GLFW_DIR}/include" \
    -DGLFW3_LIBRARY="${GLFW_DIR}/lib/libglfw3.a" \
    -DCMAKE_BUILD_TYPE=Release

make -j$(nproc)

echo ""
echo "=== Compilación exitosa ==="
echo "Ejecutable: ${BUILD_DIR}/cma_chemistry.exe"
echo ""
echo "Para distribuir, copia también:"
echo "  - ${GLFW_DIR}/bin/libglfw3.dll (si aplica)"
echo "  - Archivos DLL de MinGW (libgcc_s_seh-1.dll, libstdc++-6.dll, libwinpthread-1.dll)"
echo ""
echo "Para empaquetar todo en un instalador:"
echo "  Puedes usar NSIS (https://nsis.sourceforge.io/) o Inno Setup"
