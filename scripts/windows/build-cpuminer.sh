#!/usr/bin/env bash
# Build cpuminer binary (Windows/MSYS2)
# This script runs inside the MSYS2 MINGW64 shell on GitHub Actions.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
CPUMINER_DIR="${CPUMINER_SRC:-$PROJECT_DIR/../cpuminer}"
BUILD_DIR="$PROJECT_DIR/build/cpuminer"
OUTPUT_DIR="$PROJECT_DIR/src-tauri/binaries"
TARGET="x86_64-pc-windows-msvc"

echo "=== Building cpuminer binary (Windows/MSYS2) ==="
echo "Source: $CPUMINER_DIR"
echo "Build:  $BUILD_DIR"
echo "Output: $OUTPUT_DIR"
echo "Target: $TARGET"
echo ""

# Create build directory
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"

# Copy source to build dir (autotools modifies source dir)
cp -r "$CPUMINER_DIR"/* "$BUILD_DIR/"
cd "$BUILD_DIR"

# Build
echo "Running autogen..."
./autogen.sh

echo "Running configure..."
./configure CFLAGS="-O3"

echo "Building..."
make -j$(nproc)

# Copy output binary and its required MinGW DLLs into a self-contained directory.
# cpuminer is built with MinGW/MSYS2 and dynamically links against MinGW runtime
# DLLs that don't exist on end-user Windows machines. We bundle them alongside
# the executable (similar to hathor-core's onedir layout) so Tauri's resource
# bundling keeps them together.
echo ""
echo "Copying binary and DLLs to output directory..."
CPUMINER_DIR_OUT="$OUTPUT_DIR/cpuminer-$TARGET"
rm -rf "$CPUMINER_DIR_OUT"
mkdir -p "$CPUMINER_DIR_OUT"
cp minerd.exe "$CPUMINER_DIR_OUT/cpuminer.exe" 2>/dev/null || cp minerd "$CPUMINER_DIR_OUT/cpuminer.exe"

# Find and copy all required MinGW DLLs
echo "Resolving runtime DLL dependencies..."
MINGW_BIN="/mingw64/bin"
ldd "$CPUMINER_DIR_OUT/cpuminer.exe" 2>/dev/null | \
    grep -i "$MINGW_BIN" | \
    awk '{print $3}' | \
    while read -r dll; do
        dll_name=$(basename "$dll")
        echo "  Bundling: $dll_name"
        cp "$dll" "$CPUMINER_DIR_OUT/$dll_name"
    done

echo ""
echo "Contents of cpuminer bundle:"
ls -la "$CPUMINER_DIR_OUT/"

# Also create the externalBin sidecar (Tauri expects it for the sidecar protocol)
cp "$CPUMINER_DIR_OUT/cpuminer.exe" "$OUTPUT_DIR/cpuminer-$TARGET.exe"

echo ""
echo "=== Build complete ==="
echo "Binary: $CPUMINER_DIR_OUT/cpuminer.exe"
echo "Sidecar: $OUTPUT_DIR/cpuminer-$TARGET.exe"
