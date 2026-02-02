{ pkgs, src }:

# wallet-headless wrapper that builds the Node.js app on first run.
# The build is cached in ~/.cache/hathor-forge/wallet-headless/
pkgs.writeShellScriptBin "wallet-headless-setup" ''
  set -euo pipefail

  CACHE_DIR="''${XDG_CACHE_HOME:-$HOME/.cache}/hathor-forge"
  BUILD_DIR="$CACHE_DIR/wallet-headless"
  STAMP="$BUILD_DIR/.installed-0.36.1"
  SRC="${src}"

  if [ ! -f "$STAMP" ]; then
    echo "Setting up wallet-headless (first run, may take a minute)..." >&2
    rm -rf "$BUILD_DIR"
    mkdir -p "$BUILD_DIR"

    cp -r "$SRC"/* "$BUILD_DIR/"
    chmod -R u+w "$BUILD_DIR"
    cd "$BUILD_DIR"

    # Remove patch-package from postinstall
    ${pkgs.gnused}/bin/sed -i 's/"postinstall": "patch-package"/"postinstall": "echo skipping patches"/' package.json || true
    ${pkgs.gnused}/bin/sed -i 's/"postinstall": "npx patch-package"/"postinstall": "echo skipping patches"/' package.json || true

    # Force CommonJS output for Node 22 compat
    cat > babel.config.json << 'EOF'
{
  "presets": [["@babel/preset-env", {"targets": {"node": "18"}, "modules": "commonjs"}]],
  "plugins": ["@babel/plugin-proposal-class-properties"]
}
EOF

    ${pkgs.nodejs_22}/bin/npm install --ignore-scripts >&2
    ${pkgs.nodejs_22}/bin/npm install >&2
    ${pkgs.nodejs_22}/bin/npm run build >&2

    touch "$STAMP"
    echo "wallet-headless ready." >&2
  fi

  echo "$BUILD_DIR"
''
