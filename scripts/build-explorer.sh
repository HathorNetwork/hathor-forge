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

# Patch wallet-lib bigIntReviver for WebKit/Safari compatibility.
# Safari throws "Failed to parse String to BigInt" for float-to-BigInt,
# but the upstream code only matches V8's error message. Broadening the
# catch to all SyntaxErrors is safe since any SyntaxError from BigInt()
# means the value can't be a BigInt.
BIGINT_FILE="node_modules/@hathor/wallet-lib/lib/utils/bigint.js"
if [ -f "$BIGINT_FILE" ]; then
    echo "Patching wallet-lib bigIntReviver for WebKit compatibility..."
    node -e "
      const fs = require('fs');
      let code = fs.readFileSync('$BIGINT_FILE', 'utf8');
      code = code.replace(
        /if \(e instanceof SyntaxError && \(e\.message ===.*?\)\) \{/s,
        'if (e instanceof SyntaxError) {'
      );
      fs.writeFileSync('$BIGINT_FILE', code);
    "
fi

# Build with basic mode + localnet config
# Note: URLs point to localhost:49081 where our proxy server runs
# The proxy forwards requests to the fullnode at localhost:49080
echo ""
echo "Building with basic mode configuration..."
REACT_APP_EXPLORER_MODE=basic \
REACT_APP_BASE_URL=http://localhost:49081/v1a/ \
REACT_APP_WS_URL=ws://localhost:49081/v1a/ws/ \
REACT_APP_NETWORK=local-privatenet \
npm run build

# Copy output
echo ""
echo "Copying build to output directory..."
mkdir -p "$OUTPUT_DIR"
rm -rf "$OUTPUT_DIR"/*
cp -r build/* "$OUTPUT_DIR/"

# Inject WebKit polyfill for JSON.parse reviver context.source
# Tauri on macOS uses WebKit which may lack context.source support (needs Safari 18.4+).
# The wallet-lib bigIntReviver depends on it; without it BigInt(undefined) crashes.
POLYFILL_SRC="$PROJECT_DIR/scripts/explorer-patches/webkit-bigint-polyfill.js"
if [ -f "$POLYFILL_SRC" ]; then
    echo "Injecting WebKit BigInt polyfill..."
    cp "$POLYFILL_SRC" "$OUTPUT_DIR/static/js/webkit-bigint-polyfill.js"
    sed -i.bak 's|<script defer="defer"|<script src="/static/js/webkit-bigint-polyfill.js"></script><script defer="defer"|' "$OUTPUT_DIR/index.html"
    rm -f "$OUTPUT_DIR/index.html.bak"
fi

echo ""
echo "=== Build complete ==="
echo "Output: $OUTPUT_DIR"
echo ""
echo "The explorer will be served at http://localhost:49081 when the node is running."
