{ pkgs, src }:

pkgs.stdenv.mkDerivation rec {
  pname = "hathor-cpuminer";
  version = "2.5.0";

  inherit src;

  nativeBuildInputs = with pkgs; [
    autoreconfHook
    pkg-config
  ];

  buildInputs = with pkgs; [
    curl
    jansson
    openssl
  ];

  configureFlags = [
    "--with-crypto"
    "--with-curl"
  ];

  enableParallelBuilding = true;

  # The binary is called "minerd"
  postInstall = ''
    # Create a more descriptive symlink
    ln -s $out/bin/minerd $out/bin/hathor-miner
  '';

  meta = with pkgs.lib; {
    description = "CPU miner for Hathor Network";
    homepage = "https://github.com/HathorNetwork/cpuminer";
    license = licenses.gpl2;
    platforms = platforms.unix;
  };
}
