# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Hathor Forge is a local blockchain development environment for Hathor Network (similar to Ganache for Ethereum). It's a Tauri 2.x desktop application with a React frontend and Rust backend that manages multiple services: a Hathor fullnode, CPU miner, wallet-headless service, and block explorer.

## Development Commands

All commands assume you're in the Nix development shell (`nix develop` or auto-loaded via direnv).

```bash
# Start development server (frontend + Tauri)
dev-server

# Build production release
build-release

# Build required binaries (must be done before running the app)
build-core            # hathor-core fullnode binary
build-cpuminer        # CPU miner binary
build-wallet-headless # Wallet service
build-explorer        # Block explorer

# Code checks
cargo check --manifest-path src-tauri/Cargo.toml   # Rust type checking
cargo fmt --manifest-path src-tauri/Cargo.toml     # Rust formatting
cargo test --manifest-path src-tauri/Cargo.toml    # Rust tests

# Alternative using justfile
just check    # Run cargo check and npm lint
just fmt      # Format Rust and JS code
just test     # Run all tests
```

## Architecture

### Tech Stack
- **Frontend**: React 19 + TypeScript + Tailwind CSS 4 + Zustand (state) + React Query
- **Backend**: Tauri 2.x (Rust) with Axum for HTTP/WebSocket servers
- **Services**: hathor-core (Python), cpuminer (C), wallet-headless (Node.js)

### Key Files
- `src/App.tsx` - Main React component (~60KB, contains all UI logic)
- `src-tauri/src/lib.rs` - Rust backend with Tauri commands for process management
- `src-tauri/src/mcp.rs` - MCP server implementation (JSON-RPC over HTTP)

### Service Ports
| Service | Port |
|---------|------|
| Fullnode API | 8080 |
| Stratum (mining) | 8000 |
| Wallet Headless | 8001 |
| Explorer | 3001 |
| MCP Server | 9876 |
| Vite Dev Server | 1420 |

### Tauri Commands
The Rust backend exposes these commands to the frontend via `#[tauri::command]`:
- Node: `start_node`, `stop_node`, `get_node_status`, `reset_data`
- Miner: `start_miner`, `stop_miner`, `get_miner_status`
- Wallet Headless: `start_headless`, `stop_headless`, `get_headless_status`
- Wallet Operations: `create_headless_wallet`, `get_headless_wallet_status`, `get_headless_wallet_balance`, `get_headless_wallet_addresses`, `headless_wallet_send_tx`, `close_headless_wallet`
- Fullnode Wallet: `get_fullnode_balance`, `send_tx`, `get_wallet_addresses`
- Explorer: `start_explorer_server`, `stop_explorer_server`
- Utilities: `generate_seed`, `get_state`

### MCP Integration
The embedded MCP server (port 9876) allows AI assistants to control the environment. It implements JSON-RPC 2.0 and provides 26 tools for node/miner/wallet management.

## Development Notes

### Path Alias
The frontend uses `@/` as an alias for `src/` (configured in vite.config.ts).

### Binary Locations
Bundled binaries go in `src-tauri/binaries/` (gitignored):
- `hathor-core-*/` - PyInstaller onedir bundle
- `cpuminer-*` - Native binary
- `wallet-headless-dist/` - Node.js bundle
- `explorer-dist/` - Static build

### Default Development Wallet
The fullnode runs with a pre-funded HD wallet (for faucet functionality):
- Seed: `avocado spot town typical traffic vault danger century property shallow divorce festival spend attack anchor afford rotate green audit adjust fade wagon depart level`
- Address: `WXkMhVgRVmTXTVh47wauPKm1xcrW8Qf3Vb`

### Data Directory
Node data is stored in the user's local data directory: `~/.local/share/hathor-forge/data` (or equivalent on macOS/Windows).

## Cross-Platform Support

### Supported Platforms
- **macOS**: Full support via Nix (aarch64-apple-darwin, x86_64-apple-darwin)
- **Linux**: Full support via Nix (aarch64-unknown-linux-gnu, x86_64-unknown-linux-gnu)
- **Windows**: Experimental support via GitHub Actions CI (x86_64-pc-windows-msvc)

### Platform-Specific Code
The Rust backend handles platform differences in `src-tauri/src/lib.rs`:
- `set_library_path_env()` - Sets `DYLD_FALLBACK_LIBRARY_PATH` on macOS, `LD_LIBRARY_PATH` on Linux
- `get_binary_path()` - Adds `.exe` suffix for Windows binaries
- Process termination uses `kill -TERM` on Unix, `taskkill` on Windows

### Building Linux from macOS
Use Docker to build Linux binaries without a Linux machine:
```bash
build-linux  # Runs Nix build inside a Linux container
```
Requires Docker Desktop installed.

### CI/CD
GitHub Actions workflow (`.github/workflows/build.yml`) builds for all platforms:
- Linux/macOS: Uses Nix for reproducible builds
- Windows: Uses native MSVC toolchain with MSYS2 for cpuminer
