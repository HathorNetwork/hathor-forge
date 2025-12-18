#!/usr/bin/env bash
# Build cpuminer binary
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
CPUMINER_DIR="${CPUMINER_SRC:-$PROJECT_DIR/../cpuminer}"
BUILD_DIR="$PROJECT_DIR/build/cpuminer"
OUTPUT_DIR="$PROJECT_DIR/src-tauri/binaries"

echo "=== Building cpuminer binary ==="
echo "Source: $CPUMINER_DIR"
echo "Build:  $BUILD_DIR"
echo "Output: $OUTPUT_DIR"
echo ""

# Detect target triple
if [[ "$OSTYPE" == "darwin"* ]]; then
    if [[ "$(uname -m)" == "arm64" ]]; then
        TARGET="aarch64-apple-darwin"
    else
        TARGET="x86_64-apple-darwin"
    fi
elif [[ "$OSTYPE" == "linux"* ]]; then
    if [[ "$(uname -m)" == "aarch64" ]]; then
        TARGET="aarch64-unknown-linux-gnu"
    else
        TARGET="x86_64-unknown-linux-gnu"
    fi
else
    TARGET="x86_64-pc-windows-msvc"
fi

echo "Target: $TARGET"
echo ""

# Create build directory
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"

# Copy source to build dir (needed because autotools modifies source dir)
cp -r "$CPUMINER_DIR"/* "$BUILD_DIR/"
cd "$BUILD_DIR"

# Build
echo "Running autogen..."
./autogen.sh

echo "Running configure..."
./configure CFLAGS="-O3"

echo "Building..."
make -j$(nproc 2>/dev/null || sysctl -n hw.ncpu)

# Copy output
echo ""
echo "Copying binary to output directory..."
mkdir -p "$OUTPUT_DIR"
cp minerd "$OUTPUT_DIR/cpuminer-$TARGET"
chmod +x "$OUTPUT_DIR/cpuminer-$TARGET"

echo ""
echo "=== Build complete ==="
echo "Binary: $OUTPUT_DIR/cpuminer-$TARGET"
echo ""
echo "Test with:"
echo "  $OUTPUT_DIR/cpuminer-$TARGET --help"
