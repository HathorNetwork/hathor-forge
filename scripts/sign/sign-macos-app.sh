#!/usr/bin/env bash
# Sign the Tauri .app bundle, create a DMG, and notarize it.
# Must be run AFTER `npm run tauri build -- --bundles app`.
#
# Required env:
#   APPLE_SIGNING_IDENTITY - e.g. "Developer ID Application: Hathor Labs (TEAM_ID)"
#   APPLE_ID               - Apple ID email for notarization
#   APPLE_PASSWORD          - App-specific password
#   APPLE_TEAM_ID           - 10-char Team ID
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
BUNDLE_DIR="$PROJECT_DIR/src-tauri/target/release/bundle/macos"
APP_PATH="$BUNDLE_DIR/Hathor Forge.app"
DIST_DIR="$PROJECT_DIR/dist"
DMG_PATH="$DIST_DIR/Hathor-Forge-signed.dmg"

for var in APPLE_SIGNING_IDENTITY APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID; do
    if [[ -z "${!var:-}" ]]; then
        echo "Error: $var is not set."
        exit 1
    fi
done

if [[ ! -d "$APP_PATH" ]]; then
    echo "Error: App bundle not found at: $APP_PATH"
    echo "Run 'npm run tauri build -- --bundles app' first."
    exit 1
fi

echo "=== Signing Hathor Forge.app ==="
echo "Identity: $APPLE_SIGNING_IDENTITY"
echo ""

# Step 1: Deep-sign the .app bundle
# Tauri may have ad-hoc signed it; we replace with our Developer ID.
echo "[1/4] Signing .app bundle..."
codesign --force --deep --sign "$APPLE_SIGNING_IDENTITY" \
    --options runtime \
    --timestamp \
    --entitlements /dev/stdin \
    "$APP_PATH" << 'ENTITLEMENTS'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.cs.allow-unsigned-executable-memory</key>
    <true/>
    <key>com.apple.security.cs.allow-jit</key>
    <true/>
    <key>com.apple.security.cs.disable-library-validation</key>
    <true/>
    <key>com.apple.security.network.client</key>
    <true/>
    <key>com.apple.security.network.server</key>
    <true/>
    <key>com.apple.security.files.user-selected.read-write</key>
    <true/>
</dict>
</plist>
ENTITLEMENTS

echo ""

# Verify
echo "Verifying .app signature..."
codesign --verify --deep --strict --verbose=2 "$APP_PATH"
echo ""

# Step 2: Create DMG
echo "[2/4] Creating DMG..."
mkdir -p "$DIST_DIR"
rm -f "$DMG_PATH"
hdiutil create \
    -volname "Hathor Forge" \
    -srcfolder "$APP_PATH" \
    -ov \
    -format UDZO \
    "$DMG_PATH"
echo ""

# Step 3: Sign the DMG
echo "[3/4] Signing DMG..."
codesign --force --sign "$APPLE_SIGNING_IDENTITY" --timestamp "$DMG_PATH"
echo ""

# Step 4: Notarize
echo "[4/4] Submitting for notarization..."
echo "This may take a few minutes..."
xcrun notarytool submit "$DMG_PATH" \
    --apple-id "$APPLE_ID" \
    --password "$APPLE_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" \
    --wait

echo ""

# Staple the notarization ticket to the DMG
echo "Stapling notarization ticket..."
xcrun stapler staple "$DMG_PATH"

echo ""
echo "=== Done ==="
echo "Signed and notarized DMG: $DMG_PATH"
echo ""
echo "Verify with:"
echo "  spctl --assess --type open --context context:primary-signature -v \"$DMG_PATH\""
