#!/usr/bin/env bash
#
# Verify that the version string is consistent across Cargo.toml,
# tauri.conf.json, and package.json.
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

cargo_version=$(grep '^version' "$REPO_ROOT/src-tauri/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
tauri_version=$(grep '"version"' "$REPO_ROOT/src-tauri/tauri.conf.json" | head -1 | sed 's/.*"\([0-9][^"]*\)".*/\1/')
npm_version=$(grep '"version"' "$REPO_ROOT/package.json" | head -1 | sed 's/.*"\([0-9][^"]*\)".*/\1/')

echo "Cargo.toml:      $cargo_version"
echo "tauri.conf.json:  $tauri_version"
echo "package.json:     $npm_version"

if [ "$cargo_version" = "$tauri_version" ] && [ "$cargo_version" = "$npm_version" ]; then
    echo "All versions match."
    exit 0
else
    echo "ERROR: Version mismatch detected!" >&2
    exit 1
fi
