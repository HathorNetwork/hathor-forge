#!/usr/bin/env bash
# Copy the Nix-provided Node.js binary into src-tauri/binaries/ for bundling.
# Must be run inside `nix develop` (or via direnv).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
OUTPUT_DIR="$PROJECT_DIR/src-tauri/binaries"

# Resolve the target triple
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Darwin)
        [[ "$ARCH" == "arm64" ]] && TARGET="aarch64-apple-darwin" || TARGET="x86_64-apple-darwin"
        ;;
    Linux)
        [[ "$ARCH" == "aarch64" ]] && TARGET="aarch64-unknown-linux-gnu" || TARGET="x86_64-unknown-linux-gnu"
        ;;
    *)
        echo "Unsupported OS: $OS" >&2
        exit 1
        ;;
esac

NODE_BIN="$(command -v node 2>/dev/null || true)"
if [[ -z "$NODE_BIN" ]]; then
    echo "Error: 'node' not found in PATH. Run this inside 'nix develop'." >&2
    exit 1
fi

mkdir -p "$OUTPUT_DIR"
cp "$NODE_BIN" "$OUTPUT_DIR/node-${TARGET}"
chmod +x "$OUTPUT_DIR/node-${TARGET}"

echo "Copied $(node --version) -> $OUTPUT_DIR/node-${TARGET}"
