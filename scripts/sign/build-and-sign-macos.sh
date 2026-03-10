#!/usr/bin/env bash
# Full build + sign pipeline for macOS.
# Run inside `nix develop` shell.
#
# Required env:
#   APPLE_SIGNING_IDENTITY - e.g. "Developer ID Application: Hathor Labs (TEAM_ID)"
#   APPLE_ID               - Apple ID email for notarization
#   APPLE_PASSWORD          - App-specific password
#   APPLE_TEAM_ID           - 10-char Team ID
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

for var in APPLE_SIGNING_IDENTITY APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID; do
    if [[ -z "${!var:-}" ]]; then
        echo "Error: $var is not set."
        echo ""
        echo "Usage:"
        echo "  export APPLE_SIGNING_IDENTITY=\"Developer ID Application: Your Name (TEAM_ID)\""
        echo "  export APPLE_ID=\"your@email.com\""
        echo "  export APPLE_PASSWORD=\"xxxx-xxxx-xxxx-xxxx\""
        echo "  export APPLE_TEAM_ID=\"XXXXXXXXXX\""
        echo "  $0"
        exit 1
    fi
done

# Verify we're in nix develop
if ! command -v hathor-cli &>/dev/null && [[ -z "${HATHOR_CORE_SRC:-}" ]]; then
    echo "Warning: It looks like you're not in a nix develop shell."
    echo "Run: nix develop"
    echo ""
    read -p "Continue anyway? [y/N] " -r
    [[ $REPLY =~ ^[Yy]$ ]] || exit 1
fi

cd "$PROJECT_DIR"

echo "============================================"
echo "  Hathor Forge — macOS Build & Sign"
echo "============================================"
echo ""

# Step 1: Build all binaries
echo "=== [1/5] Building binaries ==="
echo ""

echo "--- hathor-core ---"
bash ./scripts/build-hathor-core.sh
echo ""

echo "--- cpuminer ---"
bash ./scripts/build-cpuminer.sh
echo ""

echo "--- tx-mining-service ---"
bash ./scripts/build-tx-mining-service.sh
echo ""

echo "--- wallet-headless ---"
bash ./scripts/build-wallet-headless.sh
echo ""

echo "--- explorer ---"
bash ./scripts/build-explorer.sh
echo ""

echo "--- node binary ---"
bash ./scripts/build-node.sh
echo ""

# Step 2: Sign bundled binaries
echo "=== [2/5] Signing bundled binaries ==="
echo ""
bash ./scripts/sign/sign-macos-binaries.sh
echo ""

# Step 3: Build Tauri app
echo "=== [3/5] Building Tauri app ==="
echo ""
npm ci
npm run tauri build -- --bundles app
echo ""

# Step 4: Sign app + create DMG + notarize
echo "=== [4/5] Signing app and creating DMG ==="
echo ""
bash ./scripts/sign/sign-macos-app.sh
echo ""

# Step 5: Summary
echo "=== [5/5] Summary ==="
DMG_PATH="$PROJECT_DIR/dist/Hathor-Forge-signed.dmg"
if [[ -f "$DMG_PATH" ]]; then
    echo "Signed DMG: $DMG_PATH"
    echo "Size: $(du -h "$DMG_PATH" | cut -f1)"
    echo ""
    echo "Ready for distribution!"
else
    echo "Error: DMG not found at $DMG_PATH"
    exit 1
fi
