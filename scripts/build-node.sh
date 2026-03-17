#!/usr/bin/env bash
# Copy the Nix-provided Node.js binary into src-tauri/binaries/ for bundling.
# On macOS, also bundles dependent dylibs and rewrites rpaths so the binary
# works on machines without Nix installed.
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

# On macOS, bundle Nix dylibs and rewrite rpaths so the binary is portable
if [[ "$OS" == "Darwin" ]]; then
    NODE_OUT="$OUTPUT_DIR/node-${TARGET}"
    DYLIB_DIR="$OUTPUT_DIR/node-dylibs"
    mkdir -p "$DYLIB_DIR"

    echo "Bundling dylibs for portable macOS binary..."

    # Find all non-system dylib dependencies (Nix store paths)
    otool -L "$NODE_OUT" | awk 'NR>1 {print $1}' | { grep '^/nix/' || true; } | while read -r dylib; do
        dylib_name="$(basename "$dylib")"
        echo "  Bundling $dylib_name"
        cp "$dylib" "$DYLIB_DIR/$dylib_name"
        chmod 644 "$DYLIB_DIR/$dylib_name"

        # Rewrite the node binary to look for this dylib via @executable_path
        install_name_tool -change "$dylib" "@executable_path/node-dylibs/$dylib_name" "$NODE_OUT"
    done

    # Also fix dylib cross-references (dylibs that depend on other bundled dylibs)
    for dylib_file in "$DYLIB_DIR"/*.dylib; do
        [[ -f "$dylib_file" ]] || continue
        # Fix the dylib's own install name
        dylib_name="$(basename "$dylib_file")"
        install_name_tool -id "@executable_path/node-dylibs/$dylib_name" "$dylib_file" 2>/dev/null || true

        # Fix references to other Nix dylibs
        otool -L "$dylib_file" | awk 'NR>1 {print $1}' | { grep '^/nix/' || true; } | while read -r dep; do
            dep_name="$(basename "$dep")"
            if [[ -f "$DYLIB_DIR/$dep_name" ]]; then
                install_name_tool -change "$dep" "@executable_path/node-dylibs/$dep_name" "$dylib_file" 2>/dev/null || true
            else
                # This dependency wasn't bundled yet — copy it
                echo "  Bundling transitive dep $dep_name"
                cp "$dep" "$DYLIB_DIR/$dep_name"
                chmod 644 "$DYLIB_DIR/$dep_name"
                install_name_tool -change "$dep" "@executable_path/node-dylibs/$dep_name" "$dylib_file" 2>/dev/null || true
            fi
        done
    done

    # Verify no Nix paths remain
    remaining=$(otool -L "$NODE_OUT" | grep '/nix/' || true)
    if [[ -n "$remaining" ]]; then
        echo "WARNING: Node binary still references Nix paths:"
        echo "$remaining"
    else
        echo "All Nix dylib references rewritten to @executable_path/node-dylibs/"
    fi

    echo "Bundled $(ls "$DYLIB_DIR" | wc -l | tr -d ' ') dylibs into $DYLIB_DIR"
fi

# On Linux, bundle Nix shared libraries and patch the binary's rpath
if [[ "$OS" == "Linux" ]]; then
    NODE_OUT="$OUTPUT_DIR/node-${TARGET}"
    LIB_DIR="$OUTPUT_DIR/node-dylibs"
    mkdir -p "$LIB_DIR"

    echo "Bundling shared libraries for portable Linux binary..."

    # Find all non-system .so dependencies from Nix store
    ldd "$NODE_OUT" | awk '{print $3}' | { grep '^/nix/' || true; } | while read -r lib; do
        lib_name="$(basename "$lib")"
        echo "  Bundling $lib_name"
        cp "$lib" "$LIB_DIR/$lib_name"
        chmod 644 "$LIB_DIR/$lib_name"
    done

    # Set rpath to look next to the binary
    patchelf --set-rpath '$ORIGIN/node-dylibs' "$NODE_OUT"

    # Verify
    remaining=$(ldd "$NODE_OUT" 2>&1 | grep '/nix/' || true)
    if [[ -n "$remaining" ]]; then
        echo "WARNING: Node binary still references Nix paths:"
        echo "$remaining"
    else
        echo "All Nix library references patched to \$ORIGIN/node-dylibs/"
    fi

    echo "Bundled $(ls "$LIB_DIR" | wc -l | tr -d ' ') libraries into $LIB_DIR"
fi
