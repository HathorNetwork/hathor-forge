#!/usr/bin/env bash
# Build hathor-wallet-headless for multi-wallet support
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
HEADLESS_SRC="${WALLET_HEADLESS_SRC:-$PROJECT_DIR/../hathor-wallet-headless}"
BUILD_DIR="$PROJECT_DIR/build/wallet-headless"
OUTPUT_DIR="$PROJECT_DIR/src-tauri/wallet-headless-dist"

echo "=== Building hathor-wallet-headless ==="
echo "Source: $HEADLESS_SRC"
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
cp -r "$HEADLESS_SRC"/* "$BUILD_DIR/"
chmod -R u+w "$BUILD_DIR"
cd "$BUILD_DIR"

# Remove patch-package from postinstall to avoid issues
# The patches may not apply cleanly and are not critical for our use case
if [ -f "package.json" ]; then
    # Create a backup and modify postinstall
    sed -i.bak 's/"postinstall": "patch-package"/"postinstall": "echo skipping patches"/' package.json || true
    sed -i.bak 's/"postinstall": "npx patch-package"/"postinstall": "echo skipping patches"/' package.json || true
fi

# Update babel config to force CommonJS output (fixes ESM resolution issues with Node 22)
cat > babel.config.json << 'EOF'
{
  "presets": [
    [
      "@babel/preset-env",
      {
        "targets": {
          "node": "18"
        },
        "modules": "commonjs"
      }
    ]
  ],
  "plugins": ["@babel/plugin-proposal-class-properties"]
}
EOF

# Install dependencies
echo "Installing dependencies..."
npm install --ignore-scripts
npm install  # Run postinstall separately

# Build (transpile with babel to CommonJS)
echo ""
echo "Building..."
npm run build

# Copy output
echo ""
echo "Copying build to output directory..."
mkdir -p "$OUTPUT_DIR"
rm -rf "$OUTPUT_DIR"/*

# Copy dist folder (compiled code)
cp -r dist "$OUTPUT_DIR/"

# Copy node_modules (required at runtime)
cp -r node_modules "$OUTPUT_DIR/"

# Copy package.json (for reference)
cp package.json "$OUTPUT_DIR/"

echo ""
echo "=== Build complete ==="
echo "Output: $OUTPUT_DIR"
echo ""
echo "Run with: node $OUTPUT_DIR/dist/index.js --config <config_file>"
