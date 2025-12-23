{
  description = "Hathor Forge - Local development environment for Hathor Network";

  inputs = {
nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
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
    hathor-explorer-src = {
      url = "github:hathornetwork/hathor-explorer";
      flake = false;
    };
    wallet-headless-src = {
      url = "github:hathornetwork/hathor-wallet-headless";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, hathor-core-src, cpuminer-src, hathor-explorer-src, wallet-headless-src }:
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
              nodejs_22
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

        # Runnable apps
        apps = {
          dev = {
            type = "app";
            program = toString (pkgs.writeShellScript "dev" ''
              cd ${toString ./.}
              ${pkgs.nodejs_22}/bin/npx tauri dev
            '');
          };
          build = {
            type = "app";
            program = toString (pkgs.writeShellScript "build" ''
              cd ${toString ./.}
              ${pkgs.nodejs_22}/bin/npx tauri build
            '');
          };
          build-core = {
            type = "app";
            program = toString (pkgs.writeShellScript "build-core" ''
              cd ${toString ./.}
              ./scripts/build-hathor-core.sh
            '');
          };
          build-cpuminer = {
            type = "app";
            program = toString (pkgs.writeShellScript "build-cpuminer" ''
              cd ${toString ./.}
              ./scripts/build-cpuminer.sh
            '');
          };
          build-wallet-headless = {
            type = "app";
            program = toString (pkgs.writeShellScript "build-wallet-headless" ''
              cd ${toString ./.}
              ./scripts/build-wallet-headless.sh
            '');
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain
            rustToolchain

            # Node.js (22+ required for hathor-explorer)
            nodejs_22

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
            bun

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
            # Add local scripts to PATH
            export PATH="$PWD/scripts/bin:$PATH"

            # Point to GitHub sources from flake inputs
            export HATHOR_CORE_SRC="${hathor-core-src}"
            export CPUMINER_SRC="${cpuminer-src}"
            export HATHOR_EXPLORER_SRC="${hathor-explorer-src}"
            export WALLET_HEADLESS_SRC="${wallet-headless-src}"

            echo "Hathor Forge Development Environment"
            echo "====================================="
            echo ""
            echo "Available commands:"
            echo "  dev-server            - Start development server"
            echo "  build-release         - Build release"
            echo "  build-core            - Build hathor-core binary"
            echo "  build-cpuminer        - Build cpuminer binary"
            echo "  build-explorer        - Build hathor-explorer for embedding"
            echo "  build-wallet-headless - Build wallet-headless for multi-wallet support"
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
