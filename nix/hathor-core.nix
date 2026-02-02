{ pkgs, src }:

# hathor-core wrapper that sets up a Python venv on first run.
# The venv is cached in ~/.cache/hathor-forge/hathor-core-venv/
pkgs.writeShellScriptBin "hathor-core" ''
  set -euo pipefail

  CACHE_DIR="''${XDG_CACHE_HOME:-$HOME/.cache}/hathor-forge"
  VENV_DIR="$CACHE_DIR/hathor-core-venv"
  STAMP="$VENV_DIR/.installed-0.69.0"

  if [ ! -f "$STAMP" ]; then
    echo "Setting up hathor-core (first run, may take a minute)..." >&2
    rm -rf "$VENV_DIR"
    mkdir -p "$CACHE_DIR"

    export CFLAGS="-I${pkgs.rocksdb}/include -I${pkgs.snappy}/include"
    export LDFLAGS="-L${pkgs.rocksdb}/lib -L${pkgs.snappy}/lib"
    export ROCKSDB_INCLUDE_DIR="${pkgs.rocksdb}/include"
    export ROCKSDB_LIB_DIR="${pkgs.rocksdb}/lib"

    ${pkgs.python312}/bin/python3 -m venv "$VENV_DIR"
    "$VENV_DIR/bin/pip" install --upgrade pip wheel setuptools >&2
    "$VENV_DIR/bin/pip" install "${src}" >&2

    touch "$STAMP"
    echo "hathor-core ready." >&2
  fi

  export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath [ pkgs.rocksdb pkgs.openssl pkgs.snappy pkgs.zlib ]}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
  exec "$VENV_DIR/bin/hathor-cli" "$@"
''
