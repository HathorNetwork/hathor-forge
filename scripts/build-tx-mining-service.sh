#!/usr/bin/env bash
# Build tx-mining-service as a standalone binary using PyInstaller
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TX_MINING_DIR="${TX_MINING_SERVICE_SRC:-$PROJECT_DIR/../tx-mining-service}"
BUILD_DIR="$PROJECT_DIR/build/tx-mining-service"
OUTPUT_DIR="$PROJECT_DIR/src-tauri/binaries"

echo "=== Building tx-mining-service standalone binary ==="
echo "Source: $TX_MINING_DIR"
echo "Build:  $BUILD_DIR"
echo "Output: $OUTPUT_DIR"
echo ""

# Detect target triple
if [[ "$OSTYPE" == "darwin"* ]]; then
    if [[ "$(uname -m)" == "arm64" ]]; then
        TARGET="aarch64-apple-darwin"
    else
        TARGET="x86_64-apple-darwin"
    fi
elif [[ "$OSTYPE" == "linux"* ]]; then
    if [[ "$(uname -m)" == "aarch64" ]]; then
        TARGET="aarch64-unknown-linux-gnu"
    else
        TARGET="x86_64-unknown-linux-gnu"
    fi
else
    TARGET="x86_64-pc-windows-msvc"
fi

echo "Target: $TARGET"
echo ""

# Create build directory
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"
cd "$BUILD_DIR"

# Create a virtual environment for building
echo "Creating virtual environment..."
python3 -m venv venv
source venv/bin/activate

# Install dependencies
echo "Installing tx-mining-service and dependencies..."
pip install --upgrade pip wheel

# Install tx-mining-service dependencies via poetry export or direct install
cd "$TX_MINING_DIR"

# Install dependencies - poetry project with package-mode=false
# We need to install deps and make txstratum importable
pip install configargparse colorama aiohttp base58 structlog prometheus-client idna_ssl asynctest python-healthchecklib
pip install "hathorlib[client]"

# Make txstratum available as a package by installing in the venv
pip install -e . 2>/dev/null || {
    # If -e . fails (no setup.py/package), add source to PYTHONPATH instead
    export PYTHONPATH="$TX_MINING_DIR:${PYTHONPATH:-}"
}

# Install pyinstaller
pip install pyinstaller

# Go back to build dir
cd "$BUILD_DIR"

# Copy the log.conf file (needed at runtime)
cp "$TX_MINING_DIR/log.conf" . 2>/dev/null || true

# Create a minimal entry script
cat > tx_mining_entry.py << 'EOF'
#!/usr/bin/env python3
"""Entry point for PyInstaller-built tx-mining-service binary."""
import multiprocessing
import os
import sys

# Set platform-specific library path so subprocesses find bundled libraries
if getattr(sys, 'frozen', False):
    bundle_dir = os.path.dirname(sys.executable)
    internal_dir = os.path.join(bundle_dir, '_internal')
    if os.path.isdir(internal_dir):
        if sys.platform == 'darwin':
            env_var = 'DYLD_FALLBACK_LIBRARY_PATH'
        elif sys.platform == 'linux':
            env_var = 'LD_LIBRARY_PATH'
        else:
            env_var = None

        if env_var:
            current_path = os.environ.get(env_var, '')
            if internal_dir not in current_path:
                os.environ[env_var] = internal_dir + (':' + current_path if current_path else '')

if __name__ == '__main__':
    multiprocessing.freeze_support()

from txstratum.cli import main

if __name__ == '__main__':
    main()
EOF

# Create runtime hook to fix missing builtins in frozen environment
cat > pyi_rth_builtins.py << 'EOF'
import builtins

class _DisabledBuiltin:
    """Placeholder for builtins that don't exist in frozen environment."""
    def __init__(self, name):
        self._name = name
    def __call__(self, *args, **kwargs):
        raise RuntimeError(f"The builtin '{self._name}' is not available in frozen environment")
    def __repr__(self):
        return f"<disabled builtin '{self._name}'>"

_missing_builtins = ['copyright', 'credits', 'exit', 'help', 'license', 'quit']

for name in _missing_builtins:
    if not hasattr(builtins, name) or getattr(builtins, name) is None:
        setattr(builtins, name, _DisabledBuiltin(name))
EOF

# Run PyInstaller
echo ""
echo "Running PyInstaller..."
pyinstaller \
    --onedir \
    --name "tx-mining-service" \
    --clean \
    --noconfirm \
    --runtime-hook=pyi_rth_builtins.py \
    --hidden-import=_contextvars \
    --collect-all txstratum \
    --collect-all hathorlib \
    --collect-all configargparse \
    --collect-all structlog \
    --exclude-module pytest \
    --exclude-module IPython \
    tx_mining_entry.py

# Deactivate venv
deactivate

# Copy output
echo ""
echo "Copying binary bundle to output directory..."
mkdir -p "$OUTPUT_DIR"
rm -rf "$OUTPUT_DIR/tx-mining-service-$TARGET"
cp -r "dist/tx-mining-service" "$OUTPUT_DIR/tx-mining-service-$TARGET"
chmod +x "$OUTPUT_DIR/tx-mining-service-$TARGET/tx-mining-service"
# Ensure all files are readable (PyInstaller bundles may have restrictive permissions)
chmod -R u+r "$OUTPUT_DIR/tx-mining-service-$TARGET"

# Copy log.conf into the bundle (tx-mining-service looks for it in cwd)
cp "$TX_MINING_DIR/log.conf" "$OUTPUT_DIR/tx-mining-service-$TARGET/" 2>/dev/null || true

# On macOS, sign all binaries and libraries
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo ""
    echo "Signing binaries and libraries for macOS..."
    find "$OUTPUT_DIR/tx-mining-service-$TARGET" -type f \( -name "*.dylib" -o -name "*.so" \) -exec codesign --force --sign - {} \; 2>/dev/null || true
    codesign --force --sign - "$OUTPUT_DIR/tx-mining-service-$TARGET/tx-mining-service" 2>/dev/null || true
    echo "Signing complete"
fi

echo ""
echo "=== Build complete ==="
echo "Binary bundle: $OUTPUT_DIR/tx-mining-service-$TARGET/"
echo "Executable: $OUTPUT_DIR/tx-mining-service-$TARGET/tx-mining-service"
echo ""
echo "Test with:"
echo "  $OUTPUT_DIR/tx-mining-service-$TARGET/tx-mining-service --help"
