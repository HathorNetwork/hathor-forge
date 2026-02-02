{ pkgs, rustToolchain }:

let
  rustPlatform = pkgs.makeRustPlatform {
    cargo = rustToolchain;
    rustc = rustToolchain;
  };
in
rustPlatform.buildRustPackage {
  pname = "hathor-forge-cli";
  version = "0.1.0";

  src = pkgs.lib.cleanSourceWith {
    src = ../src-tauri;
    filter = path: type:
      let baseName = builtins.baseNameOf path; in
      # Include Rust source, Cargo files, and build.rs
      (builtins.match ".*\\.rs$" path != null) ||
      baseName == "Cargo.toml" ||
      baseName == "Cargo.lock" ||
      baseName == "src" ||
      baseName == "src-tauri" ||
      # Include parent dirs needed for traversal
      type == "directory";
  };

  cargoLock = {
    lockFile = ../src-tauri/Cargo.lock;
  };

  # Build only the CLI binary, without Tauri
  buildNoDefaultFeatures = true;
  buildFeatures = [];
  cargoBuildFlags = [ "--bin" "hathor-forge-cli" ];

  nativeBuildInputs = with pkgs; [
    pkg-config
  ];

  buildInputs = with pkgs; [
    openssl
  ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
    pkgs.darwin.apple_sdk.frameworks.Security
    pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
  ];

  # Skip tests for now (they may require Tauri)
  doCheck = false;

  meta = with pkgs.lib; {
    description = "Hathor Forge CLI - headless blockchain development environment";
    homepage = "https://github.com/hathornetwork/hathor-forge";
    license = licenses.mit;
    platforms = platforms.unix;
    mainProgram = "hathor-forge-cli";
  };
}
