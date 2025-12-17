#!/usr/bin/env bash
set -euo pipefail

# Build script for Hathor Forge embedded binaries
# This creates standalone executables for hathor-core and cpuminer

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
HATHOR_CORE_DIR="$PROJECT_ROOT/../hathor-core"
CPUMINER_DIR="$PROJECT_ROOT/../cpuminer"
OUTPUT_DIR="$PROJECT_ROOT/src-tauri/binaries"

# Detect platform
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Darwin)
        PLATFORM="darwin"
        ;;
    Linux)
        PLATFORM="linux"
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

case "$ARCH" in
    x86_64)
        ARCH_SUFFIX="x86_64"
        ;;
    arm64|aarch64)
        ARCH_SUFFIX="aarch64"
        ;;
    *)
        echo "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

TARGET_TRIPLE="${ARCH_SUFFIX}-apple-${PLATFORM}"
if [ "$PLATFORM" = "linux" ]; then
    TARGET_TRIPLE="${ARCH_SUFFIX}-unknown-linux-gnu"
fi

echo "Building for: $TARGET_TRIPLE"
echo "Output directory: $OUTPUT_DIR"

mkdir -p "$OUTPUT_DIR"

# ============================================
# Build hathor-core with PyInstaller
# ============================================
build_hathor_core() {
    echo ""
    echo "=========================================="
    echo "Building hathor-core..."
    echo "=========================================="

    cd "$HATHOR_CORE_DIR"

    # Ensure we're in a virtual environment with dependencies
    if [ ! -d ".venv" ]; then
        echo "Creating virtual environment..."
        python3 -m venv .venv
    fi

    source .venv/bin/activate

    # Install dependencies
    echo "Installing dependencies..."
    pip install --upgrade pip
    pip install poetry pyinstaller
    poetry install --no-interaction

    # Build with PyInstaller
    echo "Running PyInstaller..."
    pyinstaller \
        --name "hathor-core-$TARGET_TRIPLE" \
        --onefile \
        --console \
        --hidden-import=hathor \
        --hidden-import=hathor_cli \
        --hidden-import=twisted.internet.reactor \
        --hidden-import=twisted.internet.epollreactor \
        --hidden-import=twisted.internet.kqreactor \
        --hidden-import=autobahn.twisted.websocket \
        --hidden-import=rocksdb \
        --collect-all hathor \
        --collect-all hathor_cli \
        hathor_cli/main.py

    # Copy to output
    cp "dist/hathor-core-$TARGET_TRIPLE" "$OUTPUT_DIR/"

    deactivate
    echo "hathor-core built successfully!"
}

# ============================================
# Build cpuminer
# ============================================
build_cpuminer() {
    echo ""
    echo "=========================================="
    echo "Building cpuminer..."
    echo "=========================================="

    cd "$CPUMINER_DIR"

    # Clean previous build
    make clean 2>/dev/null || true

    # Build
    if [ ! -f "configure" ]; then
        ./autogen.sh
    fi

    ./configure CFLAGS="-O3"
    make -j$(nproc 2>/dev/null || sysctl -n hw.ncpu)

    # Copy to output with target triple name
    cp minerd "$OUTPUT_DIR/cpuminer-$TARGET_TRIPLE"

    echo "cpuminer built successfully!"
}

# ============================================
# Main
# ============================================
echo "Hathor Forge Binary Builder"
echo "==========================="

case "${1:-all}" in
    hathor-core)
        build_hathor_core
        ;;
    cpuminer)
        build_cpuminer
        ;;
    all)
        build_hathor_core
        build_cpuminer
        ;;
    *)
        echo "Usage: $0 [hathor-core|cpuminer|all]"
        exit 1
        ;;
esac

echo ""
echo "=========================================="
echo "Build complete!"
echo "Binaries are in: $OUTPUT_DIR"
echo "=========================================="
ls -la "$OUTPUT_DIR"
