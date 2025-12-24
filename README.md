# Hathor Forge

A one-click local blockchain for Hathor Network developers. Think Ganache, but for Hathor.

## Features

- **One-click fullnode** - Start a local Hathor node with pre-funded wallet
- **Integrated CPU miner** - Mine blocks instantly for testing
- **Real-time dashboard** - Monitor blocks, transactions, and hash rate
- **Pre-configured wallet** - Development wallet with HTR ready to spend
- **Zero configuration** - Works out of the box

## Tech Stack

- **Desktop app:** Tauri (Rust backend, lightweight ~10MB)
- **Frontend:** React + TypeScript + Tailwind CSS
- **Fullnode:** hathor-core (bundled as standalone binary)
- **Miner:** cpuminer (bundled)

## Quick Start

### Prerequisites

- Node.js 18+
- Rust (for Tauri)
- The bundled binaries in `src-tauri/binaries/`

### Development

```bash
# Enter dev shell (auto-loads with direnv, or run manually)
nix develop

# Install npm dependencies
npm install

# Start development server
dev-server
```

### Building Binaries

Before running the app, you need to build the hathor-core and cpuminer binaries:

```bash
# Build hathor-core standalone binary
build-core

# Build cpuminer binary
build-cpuminer
```

### Production Build

```bash
build-release
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      HATHOR FORGE                           │
├─────────────────────────────────────────────────────────────┤
│  React Frontend                                             │
│  - Dashboard with stats and recent blocks                   │
│  - Real-time log viewer                                     │
│  - Mining controls                                          │
├─────────────────────────────────────────────────────────────┤
│  Tauri Backend (Rust)                                       │
│  - Process management (spawn/kill node & miner)             │
│  - Event streaming via Tauri events                         │
│  - HTTP client for node API                                 │
├─────────────────────────────────────────────────────────────┤
│  Bundled Binaries                                           │
│  - hathor-core (Python fullnode, PyInstaller bundle)        │
│  - cpuminer (C miner)                                       │
└─────────────────────────────────────────────────────────────┘
```

## Development Wallet

The app uses a fixed HD wallet for local development:

```
Seed: avocado spot town typical traffic vault danger century property shallow divorce festival spend attack anchor afford rotate green audit adjust fade wagon depart level
```

**Default mining address:** `WXkMhVgRVmTXTVh47wauPKm1xcrW8Qf3Vb`

> **Warning:** Never use this wallet on mainnet or testnet. It's for local development only.

## Node Configuration

The fullnode runs with these flags:

- `--localnet` - Uses localnet genesis (fast block times)
- `--stratum 8000` - Mining stratum port
- `--status 8080` - REST API port
- `--allow-mining-without-peers` - Solo mining enabled
- `--test-mode-tx-weight` - Reduced tx weight for fast testing
- `--wallet-enable-api` - Wallet REST API enabled

## API Endpoints

When the node is running, you can access:

- **Node status:** `http://127.0.0.1:8080/v1a/status`
- **Wallet address:** `http://127.0.0.1:8080/v1a/wallet/address`
- **Wallet balance:** `http://127.0.0.1:8080/v1a/wallet/balance`

## MCP Server (AI Integration)

Hathor Forge includes an embedded MCP (Model Context Protocol) server that allows AI assistants like Claude to control the entire development environment. The MCP server starts automatically when the app launches.

### Connection Info

- **URL:** `http://127.0.0.1:9876/mcp`
- **Protocol:** JSON-RPC 2.0 over HTTP POST
- **SSE Endpoint:** `http://127.0.0.1:9876/mcp/sse`

### Claude Configuration

Add this to your Claude settings (`~/.claude/settings.json`):

```json
{
  "mcpServers": {
    "hathor-forge": {
      "type": "http",
      "url": "http://127.0.0.1:9876/mcp"
    }
  }
}
```

### Available Tools

The MCP server exposes 26 tools for complete control:

| Category | Tools |
|----------|-------|
| **Node** | `start_node`, `stop_node`, `get_node_status` |
| **Miner** | `start_miner`, `stop_miner`, `get_miner_status` |
| **Wallet Service** | `start_wallet_service`, `stop_wallet_service`, `get_wallet_service_status` |
| **Wallets** | `generate_seed`, `create_wallet`, `get_wallet_seed`, `get_wallet_status`, `get_wallet_balance`, `get_wallet_addresses`, `send_from_wallet`, `close_wallet` |
| **Faucet** | `get_faucet_balance`, `send_from_faucet`, `fund_wallet` |
| **Blockchain** | `get_blocks`, `get_transaction` |
| **Utilities** | `quick_start`, `quick_stop`, `get_full_status`, `reset_data` |

### Example: Using with curl

```bash
# Check if MCP server is running
curl http://127.0.0.1:9876/health

# Initialize MCP session
curl -X POST http://127.0.0.1:9876/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'

# List available tools
curl -X POST http://127.0.0.1:9876/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'

# Start the node
curl -X POST http://127.0.0.1:9876/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"start_node","arguments":{}}}'

# Quick start (node + miner + wallet service)
curl -X POST http://127.0.0.1:9876/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"quick_start","arguments":{}}}'
```

## Project Structure

```
hathor-dev-env/
├── src/                    # React frontend
│   ├── App.tsx             # Main app with dashboard
│   └── main.tsx            # Entry point
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs          # Rust backend (process management)
│   │   └── mcp.rs          # Embedded MCP server (port 9876)
│   ├── binaries/           # Bundled executables (gitignored)
│   │   ├── hathor-core-*/  # Fullnode binary bundle
│   │   └── cpuminer-*      # Miner binary
│   └── tauri.conf.json     # Tauri configuration
├── scripts/
│   ├── build-hathor-core.sh  # PyInstaller build script
│   └── build-cpuminer.sh     # Miner build script
└── package.json
```

## License

MIT
