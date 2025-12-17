{ pkgs, src }:

let
  python = pkgs.python312;
in
pkgs.stdenv.mkDerivation rec {
  pname = "hathor-core";
  version = "0.69.0";

  inherit src;

  nativeBuildInputs = with pkgs; [
    python
    poetry
    pkg-config
    cmake
  ];

  buildInputs = with pkgs; [
    rocksdb
    snappy
    openssl
    readline
    zlib
    xz
    bzip2
    lz4
  ];

  # Don't run the normal build phases - we'll use poetry
  dontBuild = true;
  dontConfigure = true;

  installPhase = ''
    mkdir -p $out/bin $out/lib/hathor-core

    # Copy source
    cp -r . $out/lib/hathor-core/

    # Create wrapper script that sets up the environment
    cat > $out/bin/hathor-cli << EOF
#!/usr/bin/env bash
SCRIPT_DIR="\$(cd "\$(dirname "\''${BASH_SOURCE[0]}")" && pwd)"
HATHOR_DIR="\$SCRIPT_DIR/../lib/hathor-core"

# Set up environment
export CFLAGS="-I${pkgs.rocksdb}/include"
export LDFLAGS="-L${pkgs.rocksdb}/lib"
export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath [ pkgs.rocksdb pkgs.openssl pkgs.snappy ]}:\$LD_LIBRARY_PATH"
export DYLD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath [ pkgs.rocksdb pkgs.openssl pkgs.snappy ]}:\$DYLD_LIBRARY_PATH"

# Use poetry to run hathor-cli
cd "\$HATHOR_DIR"
exec ${pkgs.poetry}/bin/poetry run hathor-cli "\$@"
EOF
    chmod +x $out/bin/hathor-cli
  '';

  meta = with pkgs.lib; {
    description = "Hathor Network full-node";
    homepage = "https://github.com/HathorNetwork/hathor-core";
    license = licenses.asl20;
    platforms = platforms.unix;
  };
}
