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
# Use -static to link all libraries (libcurl, libwinpthread, etc.) into the
# binary so it runs without needing MSYS2/MinGW DLLs on the target machine.
./configure CFLAGS="-O3" LDFLAGS="-static"

echo "Building..."
make -j$(nproc)

# Copy output
echo ""
echo "Copying binary to output directory..."
mkdir -p "$OUTPUT_DIR"
cp minerd.exe "$OUTPUT_DIR/cpuminer-$TARGET.exe" 2>/dev/null || cp minerd "$OUTPUT_DIR/cpuminer-$TARGET.exe"

echo ""
echo "=== Build complete ==="
echo "Binary: $OUTPUT_DIR/cpuminer-$TARGET.exe"
