# Hathor Forge Development Tasks

# Default recipe - show help
default:
    @just --list

# Enter Nix development shell
dev:
    nix develop

# Install npm dependencies
install:
    npm install

# Run frontend in dev mode
frontend:
    npm run dev

# Run Tauri in dev mode (includes frontend)
tauri-dev:
    npm run tauri dev

# Build release
build:
    npm run tauri build

# Build with Nix
nix-build:
    nix build

# Build just the runtime (hathor-core + cpuminer)
build-runtime:
    nix build .#runtime

# Build hathor-core
build-core:
    nix build .#hathor-core

# Build cpuminer
build-miner:
    nix build .#cpuminer

# Run tests
test:
    cargo test --manifest-path src-tauri/Cargo.toml
    npm run test

# Format code
fmt:
    cargo fmt --manifest-path src-tauri/Cargo.toml
    npm run lint:fix 2>/dev/null || true

# Check code
check:
    cargo check --manifest-path src-tauri/Cargo.toml
    npm run lint 2>/dev/null || true

# Clean build artifacts
clean:
    rm -rf node_modules dist src-tauri/target result

# Start a local Hathor node (for testing)
run-node port="8080" stratum="8000":
    hathor-cli run_node \
        --localnet \
        --status {{port}} \
        --stratum {{stratum}} \
        --data ./data \
        --allow-mining-without-peers \
        --test-mode-tx-weight \
        --unsafe-mode privatenet

# Start the miner (connects to local node)
run-miner address threads="4" stratum="8000":
    minerd \
        --url stratum+tcp://127.0.0.1:{{stratum}} \
        --user {{address}} \
        --threads {{threads}}
