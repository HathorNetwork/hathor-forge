{
  description = "Hathor Forge - Local development environment for Hathor Network";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    hathor-core-src = {
      url = "github:hathornetwork/hathor-core";
      flake = false;
    };
    cpuminer-src = {
      url = "github:hathornetwork/cpuminer";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, hathor-core-src, cpuminer-src }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Rust toolchain for Tauri
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
          targets = [ "aarch64-apple-darwin" "x86_64-apple-darwin" ];
        };

        # Build cpuminer from GitHub
        cpuminer = import ./nix/cpuminer.nix {
          inherit pkgs;
          src = cpuminer-src;
        };

        # Build hathor-core from GitHub
        hathorCore = import ./nix/hathor-core.nix {
          inherit pkgs;
          src = hathor-core-src;
        };

      in {
        packages = {
          default = self.packages.${system}.hathor-forge;
          hathor-core = hathorCore;
          cpuminer = cpuminer;

          # Combined runtime bundle
          runtime = pkgs.symlinkJoin {
            name = "hathor-forge-runtime";
            paths = [ hathorCore cpuminer ];
          };

          # Placeholder for the full Tauri app (will be built via cargo)
          hathor-forge = pkgs.stdenv.mkDerivation {
            pname = "hathor-forge";
            version = "0.1.0";
            src = ./.;

            nativeBuildInputs = with pkgs; [
              rustToolchain
              nodejs_20
              pkg-config
            ];

            buildInputs = with pkgs; [
              openssl
            ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
              pkgs.darwin.apple_sdk.frameworks.WebKit
              pkgs.darwin.apple_sdk.frameworks.Security
              pkgs.darwin.apple_sdk.frameworks.CoreServices
            ];

            buildPhase = ''
              export HOME=$(mktemp -d)
              npm ci
              npm run tauri build
            '';

            installPhase = ''
              mkdir -p $out
              cp -r src-tauri/target/release/bundle/* $out/
            '';
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain
            rustToolchain

            # Node.js
            nodejs_20

            # Tauri dependencies
            pkg-config
            openssl

            # Python for hathor-core development
            python312
            poetry

            # Build tools
            just
            autoconf
            automake

            # cpuminer dependencies
            curl

            # Database and native deps for hathor-core
            rocksdb
            snappy
            lz4
            bzip2
            xz
            zlib
            cmake
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.WebKit
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.darwin.apple_sdk.frameworks.CoreServices
            pkgs.darwin.apple_sdk.frameworks.AppKit
          ] ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
            pkgs.webkitgtk
            pkgs.gtk3
            pkgs.libsoup
            pkgs.glib
          ];

          shellHook = ''
            echo "Hathor Forge Development Environment"
            echo "====================================="
            echo ""
            echo "Available commands:"
            echo "  npm run tauri dev          - Start development server"
            echo "  npm run tauri build        - Build release"
            echo "  nix build .#cpuminer       - Build cpuminer"
            echo "  ./scripts/build-hathor-core.sh - Build hathor-core standalone binary"
            echo ""

            # Set up environment for RocksDB and native builds
            export CFLAGS="-I${pkgs.rocksdb}/include -I${pkgs.snappy}/include -I${pkgs.lz4}/include"
            export LDFLAGS="-L${pkgs.rocksdb}/lib -L${pkgs.snappy}/lib -L${pkgs.lz4}/lib"
            export ROCKSDB_INCLUDE_DIR="${pkgs.rocksdb}/include"
            export ROCKSDB_LIB_DIR="${pkgs.rocksdb}/lib"
          '';
        };
      }
    );
}
