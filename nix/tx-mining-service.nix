{ pkgs, src }:

# tx-mining-service wrapper that sets up a Python venv on first run.
# The venv is cached in ~/.cache/hathor-forge/tx-mining-venv/
pkgs.writeShellScriptBin "tx-mining-service" ''
  set -euo pipefail

  CACHE_DIR="''${XDG_CACHE_HOME:-$HOME/.cache}/hathor-forge"
  VENV_DIR="$CACHE_DIR/tx-mining-venv"
  STAMP="$VENV_DIR/.installed-0.22.0"
  SRC="${src}"

  if [ ! -f "$STAMP" ]; then
    echo "Setting up tx-mining-service (first run, may take a minute)..." >&2
    rm -rf "$VENV_DIR"
    mkdir -p "$CACHE_DIR"

    ${pkgs.python312}/bin/python3 -m venv "$VENV_DIR"
    "$VENV_DIR/bin/pip" install --upgrade pip wheel setuptools >&2

    # Install deps
    "$VENV_DIR/bin/pip" install configargparse colorama aiohttp base58 structlog prometheus-client requests "hathorlib[client]" python-healthchecklib >&2

    # Install txstratum from source
    TMPDIR=$(mktemp -d)
    cp -r "$SRC"/* "$TMPDIR/"
    chmod -R u+w "$TMPDIR"
    cd "$TMPDIR"

    # Make it installable
    if [ ! -f setup.py ]; then
      cat > setup.py << 'SETUP'
from setuptools import setup, find_packages
setup(
    name="tx-mining-service",
    version="0.22.0",
    packages=find_packages(),
    entry_points={"console_scripts": ["tx-mining-service=txstratum.cli:main"]},
)
SETUP
    fi

    "$VENV_DIR/bin/pip" install . >&2
    rm -rf "$TMPDIR"

    # Copy log.conf
    cp "$SRC/log.conf" "$VENV_DIR/" 2>/dev/null || true

    touch "$STAMP"
    echo "tx-mining-service ready." >&2
  fi

  # tx-mining-service looks for log.conf in cwd
  cd "$VENV_DIR"
  exec "$VENV_DIR/bin/tx-mining-service" "$@"
''
