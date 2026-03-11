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
# Static link so the binary runs without MSYS2/MinGW DLLs on the target machine.
# We must supply all transitive deps of libcurl because -static requires them at
# link time (the dynamic-only check would pass but actual linking would fail).
CURL_STATIC_LIBS=$(pkg-config --libs --static libcurl 2>/dev/null || echo "-lcurl -lssl -lcrypto -lz -lws2_32 -lcrypt32 -lwldap32 -lbcrypt")
./configure CFLAGS="-O3" LDFLAGS="-static" LIBS="$CURL_STATIC_LIBS -lpthread"

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
