#!/usr/bin/env bash
# Download Node.js binary for bundling with the app
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
OUTPUT_DIR="$PROJECT_DIR/src-tauri/binaries"

# Node.js version to bundle
NODE_VERSION="${NODE_VERSION:-22.14.0}"

echo "=== Downloading Node.js $NODE_VERSION binary ==="

# Detect platform
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Darwin)
        NODE_OS="darwin"
        if [[ "$ARCH" == "arm64" ]]; then
            TARGET="aarch64-apple-darwin"
            NODE_ARCH="arm64"
        else
            TARGET="x86_64-apple-darwin"
            NODE_ARCH="x64"
        fi
        ;;
    Linux)
        NODE_OS="linux"
        if [[ "$ARCH" == "aarch64" ]]; then
            TARGET="aarch64-unknown-linux-gnu"
            NODE_ARCH="arm64"
        else
            TARGET="x86_64-unknown-linux-gnu"
            NODE_ARCH="x64"
        fi
        ;;
    MINGW*|MSYS*|CYGWIN*)
        NODE_OS="win"
        TARGET="x86_64-pc-windows-msvc"
        NODE_ARCH="x64"
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

echo "Platform: $NODE_OS-$NODE_ARCH"
echo "Target:   $TARGET"
echo "Output:   $OUTPUT_DIR"
echo ""

mkdir -p "$OUTPUT_DIR"

DOWNLOAD_DIR="$PROJECT_DIR/build/node"
rm -rf "$DOWNLOAD_DIR"
mkdir -p "$DOWNLOAD_DIR"

if [[ "$NODE_OS" == "win" ]]; then
    ARCHIVE="node-v${NODE_VERSION}-${NODE_OS}-${NODE_ARCH}.zip"
    URL="https://nodejs.org/dist/v${NODE_VERSION}/${ARCHIVE}"
    echo "Downloading $URL ..."
    curl -fsSL "$URL" -o "$DOWNLOAD_DIR/$ARCHIVE"
    unzip -q "$DOWNLOAD_DIR/$ARCHIVE" -d "$DOWNLOAD_DIR"
    cp "$DOWNLOAD_DIR/node-v${NODE_VERSION}-${NODE_OS}-${NODE_ARCH}/node.exe" \
       "$OUTPUT_DIR/node-${TARGET}.exe"
    echo "Binary: $OUTPUT_DIR/node-${TARGET}.exe"
else
    ARCHIVE="node-v${NODE_VERSION}-${NODE_OS}-${NODE_ARCH}.tar.gz"
    URL="https://nodejs.org/dist/v${NODE_VERSION}/${ARCHIVE}"
    echo "Downloading $URL ..."
    curl -fsSL "$URL" -o "$DOWNLOAD_DIR/$ARCHIVE"
    tar -xzf "$DOWNLOAD_DIR/$ARCHIVE" -C "$DOWNLOAD_DIR"
    cp "$DOWNLOAD_DIR/node-v${NODE_VERSION}-${NODE_OS}-${NODE_ARCH}/bin/node" \
       "$OUTPUT_DIR/node-${TARGET}"
    chmod +x "$OUTPUT_DIR/node-${TARGET}"
    echo "Binary: $OUTPUT_DIR/node-${TARGET}"
fi

# Clean up
rm -rf "$DOWNLOAD_DIR"

echo ""
echo "=== Download complete ==="
echo ""
echo "Test with:"
echo "  $OUTPUT_DIR/node-${TARGET} --version"
