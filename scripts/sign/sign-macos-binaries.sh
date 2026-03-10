#!/usr/bin/env bash
# Sign all bundled binaries for macOS with a Developer ID certificate.
# Must be run AFTER building binaries, BEFORE building the Tauri app.
#
# Required env:
#   APPLE_SIGNING_IDENTITY - e.g. "Developer ID Application: Hathor Labs (TEAM_ID)"
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
BINARIES_DIR="$PROJECT_DIR/src-tauri/binaries"

if [[ -z "${APPLE_SIGNING_IDENTITY:-}" ]]; then
    echo "Error: APPLE_SIGNING_IDENTITY is not set."
    echo "Example: export APPLE_SIGNING_IDENTITY=\"Developer ID Application: Hathor Labs (XXXXXXXXXX)\""
    exit 1
fi

echo "=== Signing macOS binaries ==="
echo "Identity: $APPLE_SIGNING_IDENTITY"
echo "Binaries: $BINARIES_DIR"
echo ""

# Detect target triple
if [[ "$(uname -m)" == "arm64" ]]; then
    TARGET="aarch64-apple-darwin"
else
    TARGET="x86_64-apple-darwin"
fi

SIGN_FLAGS=(
    --force
    --sign "$APPLE_SIGNING_IDENTITY"
    --options runtime          # Hardened runtime (required for notarization)
    --timestamp                # Secure timestamp from Apple
)

sign_count=0

sign_file() {
    local file="$1"
    echo "  Signing: ${file#$PROJECT_DIR/}"
    codesign "${SIGN_FLAGS[@]}" "$file"
    sign_count=$((sign_count + 1))
}

# --- hathor-core onedir bundle ---
HATHOR_CORE_DIR="$BINARIES_DIR/hathor-core-$TARGET"
if [[ -d "$HATHOR_CORE_DIR" ]]; then
    echo "[hathor-core]"
    # Sign all .dylib and .so files first (innermost → outermost)
    while IFS= read -r -d '' lib; do
        sign_file "$lib"
    done < <(find "$HATHOR_CORE_DIR" -type f \( -name "*.dylib" -o -name "*.so" \) -print0)
    # Sign the main executable last
    sign_file "$HATHOR_CORE_DIR/hathor-core"
    echo ""
fi

# --- tx-mining-service onedir bundle ---
TX_MINING_DIR="$BINARIES_DIR/tx-mining-service-$TARGET"
if [[ -d "$TX_MINING_DIR" ]]; then
    echo "[tx-mining-service]"
    while IFS= read -r -d '' lib; do
        sign_file "$lib"
    done < <(find "$TX_MINING_DIR" -type f \( -name "*.dylib" -o -name "*.so" \) -print0)
    sign_file "$TX_MINING_DIR/tx-mining-service"
    echo ""
fi

# --- cpuminer ---
CPUMINER="$BINARIES_DIR/cpuminer-$TARGET"
if [[ -f "$CPUMINER" ]]; then
    echo "[cpuminer]"
    sign_file "$CPUMINER"
    echo ""
fi

# --- node ---
NODE_BIN="$BINARIES_DIR/node-$TARGET"
if [[ -f "$NODE_BIN" ]]; then
    echo "[node]"
    sign_file "$NODE_BIN"
    echo ""
fi

echo "=== Signing complete: $sign_count files signed ==="
echo ""

# Verify a sample
echo "Verifying hathor-core signature..."
if [[ -f "$HATHOR_CORE_DIR/hathor-core" ]]; then
    codesign --verify --deep --strict --verbose=2 "$HATHOR_CORE_DIR/hathor-core"
    echo "OK"
fi
