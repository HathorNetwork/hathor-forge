#!/usr/bin/env bash
# Build hathor-core as a standalone binary using PyInstaller
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
HATHOR_CORE_DIR="${HATHOR_CORE_SRC:-$PROJECT_DIR/../hathor-core}"
BUILD_DIR="$PROJECT_DIR/build/hathor-core"
OUTPUT_DIR="$PROJECT_DIR/src-tauri/binaries"

echo "=== Building hathor-core standalone binary ==="
echo "Source: $HATHOR_CORE_DIR"
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
echo "Installing hathor-core and dependencies..."
pip install --upgrade pip wheel

# Install hathor-core in editable mode
cd "$HATHOR_CORE_DIR"
pip install -e .

# Install pyinstaller
pip install pyinstaller

# Go back to build dir
cd "$BUILD_DIR"

# Create a minimal entry script
cat > hathor_entry.py << 'EOF'
#!/usr/bin/env python3
"""Entry point for PyInstaller-built hathor-core binary."""
import multiprocessing
import os
import sys

# Set platform-specific library path so subprocesses find bundled libraries instead of system ones
# This prevents macOS from aborting on "unversioned libcrypto" loads in multiprocessing subprocesses
if getattr(sys, 'frozen', False):
    bundle_dir = os.path.dirname(sys.executable)
    internal_dir = os.path.join(bundle_dir, '_internal')
    if os.path.isdir(internal_dir):
        if sys.platform == 'darwin':
            # macOS uses DYLD_FALLBACK_LIBRARY_PATH
            env_var = 'DYLD_FALLBACK_LIBRARY_PATH'
        elif sys.platform == 'linux':
            # Linux uses LD_LIBRARY_PATH
            env_var = 'LD_LIBRARY_PATH'
        else:
            # Windows handles DLL loading via PATH or same directory
            env_var = None

        if env_var:
            current_path = os.environ.get(env_var, '')
            if internal_dir not in current_path:
                os.environ[env_var] = internal_dir + (':' + current_path if current_path else '')

# CRITICAL: Must be called before any other code in frozen PyInstaller builds
# This handles multiprocessing child process spawning correctly
if __name__ == '__main__':
    multiprocessing.freeze_support()

from hathor_cli.main import main

if __name__ == '__main__':
    main()
EOF

# Create runtime hook to fix missing builtins in frozen environment
cat > pyi_rth_builtins.py << 'EOF'
# PyInstaller runtime hook to ensure all expected builtins exist
# This MUST run before any other imports
import builtins

class _DisabledBuiltin:
    """Placeholder for builtins that don't exist in frozen environment."""
    def __init__(self, name):
        self._name = name
    def __call__(self, *args, **kwargs):
        raise RuntimeError(f"The builtin '{self._name}' is not available in frozen environment")
    def __repr__(self):
        return f"<disabled builtin '{self._name}'>"

# These builtins are missing in PyInstaller frozen environment
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
    --name "hathor-core" \
    --clean \
    --noconfirm \
    --runtime-hook=pyi_rth_builtins.py \
    --hidden-import=_contextvars \
    --hidden-import=rocksdb._rocksdb \
    --hidden-import=rocksdb.interfaces \
    --hidden-import=rocksdb.errors \
    --hidden-import=cryptography.hazmat.bindings._rust \
    --collect-all=rocksdb \
    --collect-all=cryptography \
    --collect-submodules=structlog \
    --collect-submodules=twisted \
    --collect-all hathor \
    --collect-all hathor_cli \
    --collect-all hathorlib \
    --exclude-module pytest \
    --exclude-module hathor_tests \
    --exclude-module IPython \
    --exclude-module ipykernel \
    --exclude-module jupyter \
    hathor_entry.py

# Deactivate venv
deactivate

# Copy output (onedir creates a folder with the binary inside)
echo ""
echo "Copying binary bundle to output directory..."
mkdir -p "$OUTPUT_DIR"
rm -rf "$OUTPUT_DIR/hathor-core-$TARGET"
cp -r "dist/hathor-core" "$OUTPUT_DIR/hathor-core-$TARGET"
chmod +x "$OUTPUT_DIR/hathor-core-$TARGET/hathor-core"

# On macOS, sign all binaries and libraries to avoid libcrypto security abort
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo ""
    echo "Signing binaries and libraries for macOS..."
    # Sign all .dylib and .so files first
    find "$OUTPUT_DIR/hathor-core-$TARGET" -type f \( -name "*.dylib" -o -name "*.so" \) -exec codesign --force --sign - {} \; 2>/dev/null || true
    # Sign the main binary last
    codesign --force --sign - "$OUTPUT_DIR/hathor-core-$TARGET/hathor-core" 2>/dev/null || true
    echo "Signing complete"
fi

echo ""
echo "=== Build complete ==="
echo "Binary bundle: $OUTPUT_DIR/hathor-core-$TARGET/"
echo "Executable: $OUTPUT_DIR/hathor-core-$TARGET/hathor-core"
echo ""
echo "Test with:"
echo "  $OUTPUT_DIR/hathor-core-$TARGET/hathor-core help"
