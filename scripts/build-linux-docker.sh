#!/usr/bin/env bash
# Build Linux binaries using Docker (from macOS or any host)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Building Hathor Forge for Linux using Docker ==="
echo ""

# Check if Docker is available
if ! command -v docker &> /dev/null; then
    echo "Error: Docker is not installed or not in PATH"
    echo "Install Docker Desktop: https://www.docker.com/products/docker-desktop/"
    exit 1
fi

# Use nixos/nix image for reproducible builds
IMAGE="nixos/nix:2.24.10"

echo "Pulling Nix Docker image..."
docker pull "$IMAGE"

echo ""
echo "Starting Linux build container..."

# Run the build inside Docker
# Mount the project directory and run the Nix build
docker run --rm -it \
    -v "$PROJECT_DIR:/workspace" \
    -w /workspace \
    --platform linux/amd64 \
    "$IMAGE" \
    sh -c '
        echo "=== Inside Linux container ==="
        echo ""

        # Enable flakes
        mkdir -p ~/.config/nix
        echo "experimental-features = nix-command flakes" > ~/.config/nix/nix.conf

        # Trust the workspace
        git config --global --add safe.directory /workspace

        echo "Building hathor-core..."
        nix develop --command ./scripts/build-hathor-core.sh || echo "hathor-core build skipped (source may not be available)"

        echo ""
        echo "Building cpuminer..."
        nix develop --command ./scripts/build-cpuminer.sh || echo "cpuminer build skipped (source may not be available)"

        echo ""
        echo "Building wallet-headless..."
        nix develop --command ./scripts/build-wallet-headless.sh || echo "wallet-headless build skipped"

        echo ""
        echo "Building explorer..."
        nix develop --command ./scripts/build-explorer.sh || echo "explorer build skipped"

        echo ""
        echo "Installing npm dependencies..."
        nix develop --command npm ci

        echo ""
        echo "Building Tauri app for Linux..."
        nix develop --command npm run tauri build || echo "Tauri build failed (may need binaries first)"

        echo ""
        echo "=== Build complete ==="
        ls -la src-tauri/binaries/ 2>/dev/null || echo "No binaries built yet"
        ls -la src-tauri/target/release/bundle/ 2>/dev/null || echo "No Tauri bundle yet"
    '

echo ""
echo "=== Docker build finished ==="
echo "Check src-tauri/binaries/ for Linux binaries"
echo "Check src-tauri/target/release/bundle/ for .deb and .AppImage"
