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

# Copy output binary and its required MinGW DLLs.
# cpuminer is built with MinGW/MSYS2 and dynamically links against MinGW runtime
# DLLs (libwinpthread, libcurl, etc.) that don't exist on end-user Windows machines.
# We place the DLLs in a cpuminer-deps/ directory that gets bundled as a Tauri resource,
# and the Rust code adds it to PATH before spawning cpuminer.
echo ""
echo "Copying binary and DLLs to output directory..."
mkdir -p "$OUTPUT_DIR"
cp minerd.exe "$OUTPUT_DIR/cpuminer-$TARGET.exe" 2>/dev/null || cp minerd "$OUTPUT_DIR/cpuminer-$TARGET.exe"

# Find and copy all required MinGW DLLs into a deps directory
DEPS_DIR="$OUTPUT_DIR/../cpuminer-deps"
rm -rf "$DEPS_DIR"
mkdir -p "$DEPS_DIR"

echo "Resolving runtime DLL dependencies..."
MINGW_BIN="/mingw64/bin"
ldd "$OUTPUT_DIR/cpuminer-$TARGET.exe" 2>/dev/null | \
    grep -i "$MINGW_BIN" | \
    awk '{print $3}' | \
    while read -r dll; do
        dll_name=$(basename "$dll")
        echo "  Bundling: $dll_name"
        cp "$dll" "$DEPS_DIR/$dll_name"
    done

# Create a .keep file so the resource glob always matches
touch "$DEPS_DIR/.keep"

echo ""
echo "Bundled DLLs:"
ls -la "$DEPS_DIR/"

echo ""
echo "=== Build complete ==="
echo "Binary: $OUTPUT_DIR/cpuminer-$TARGET.exe"
