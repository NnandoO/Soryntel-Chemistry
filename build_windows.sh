#!/usr/bin/env bash
# Script para compilar y empaquetar CMA Chemistry para Windows usando MinGW-w64
#
# Instalación de herramientas requeridas:
#   Arch/CachyOS:
#     sudo pacman -S mingw-w64-gcc mingw-w64-winpthreads cmake ninja zip
#   Ubuntu/Debian:
#     sudo apt install -y g++-mingw-w64-x86-64 cmake ninja-build zip libeigen3-dev

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BUILD_DIR="${SCRIPT_DIR}/build_win"

echo "=== Configurando CMA Chemistry para Windows (x86_64) ==="
cmake -S "${SCRIPT_DIR}" -B "${BUILD_DIR}" \
    -DCMAKE_TOOLCHAIN_FILE="${SCRIPT_DIR}/Toolchain-mingw64.cmake" \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMA_BUILD_GUI=ON \
    -DCMA_WITH_IPHREEQC=ON

echo ""
echo "=== Compilando CMA Chemistry para Windows ==="
cmake --build "${BUILD_DIR}" -j"$(nproc)"

echo ""
echo "=== Generando paquete ZIP con CPack ==="
cpack --config "${BUILD_DIR}/CPackConfig.cmake"

echo ""
echo "=== Proceso completado con éxito ==="
echo "Ejecutable: ${BUILD_DIR}/cma_chemistry.exe"
echo "Paquete ZIP listo para distribución generado en el directorio actual."
