#!/usr/bin/env bash
# Build hathor-explorer for basic mode + localnet
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
EXPLORER_DIR="${HATHOR_EXPLORER_SRC:-$PROJECT_DIR/../hathor-explorer}"
BUILD_DIR="$PROJECT_DIR/build/explorer"
OUTPUT_DIR="$PROJECT_DIR/src-tauri/explorer-dist"

echo "=== Building hathor-explorer for embedding ==="
echo "Source: $EXPLORER_DIR"
echo "Build:  $BUILD_DIR"
echo "Output: $OUTPUT_DIR"
echo ""

# Create build directory (make writable first if exists from previous build)
if [ -d "$BUILD_DIR" ]; then
    chmod -R u+w "$BUILD_DIR" 2>/dev/null || true
    rm -rf "$BUILD_DIR"
fi
mkdir -p "$BUILD_DIR"

# Copy source to build dir (and make writable since nix store is read-only)
cp -r "$EXPLORER_DIR"/* "$BUILD_DIR/"
chmod -R u+w "$BUILD_DIR"
cd "$BUILD_DIR"

# Install dependencies
echo "Installing dependencies..."
npm install

# Build with basic mode + localnet config
# Note: URLs point to localhost:3001 where our proxy server runs
# The proxy forwards requests to the fullnode at localhost:8080
echo ""
echo "Building with basic mode configuration..."
REACT_APP_EXPLORER_MODE=basic \
REACT_APP_BASE_URL=http://localhost:3001/v1a/ \
REACT_APP_WS_URL=ws://localhost:3001/v1a/ws/ \
REACT_APP_NETWORK=local-privatenet \
npm run build

# Copy output
echo ""
echo "Copying build to output directory..."
mkdir -p "$OUTPUT_DIR"
rm -rf "$OUTPUT_DIR"/*
cp -r build/* "$OUTPUT_DIR/"

echo ""
echo "=== Build complete ==="
echo "Output: $OUTPUT_DIR"
echo ""
echo "The explorer will be served at http://localhost:3001 when the node is running."
