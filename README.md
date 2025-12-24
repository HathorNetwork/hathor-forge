# Hathor Forge

A complete local blockchain development environment for Hathor Network. Think Ganache, but for Hathor.

## Features

- **One-Click Fullnode** - Start a local Hathor node with a pre-funded development wallet
- **Integrated CPU Miner** - Mine blocks instantly for testing with real-time hash rate display
- **Multi-Wallet Support** - Create and manage multiple wallets via the wallet-headless service
- **Built-in Faucet** - Send HTR from the fullnode's wallet to any address
- **Block Explorer** - Embedded explorer to browse blocks, transactions, and addresses
- **Real-Time Dashboard** - Monitor block height, hash rate, and service status
- **MCP Server** - AI integration allowing Claude to control the entire environment
- **Zero Configuration** - Works out of the box with sensible defaults

## Screenshots

The app provides a clean dashboard with:
- Node and miner status indicators
- Block height and hash rate display
- Wallet management with Fund, Seed, and Close actions
- Built-in faucet for sending HTR
- Log viewer with filtering by service
- Block and transaction explorer

## Tech Stack

| Component | Technology |
|-----------|------------|
| Desktop App | Tauri 2.x (Rust backend, ~10MB) |
| Frontend | React 18 + TypeScript + Tailwind CSS |
| Fullnode | hathor-core (Python, PyInstaller bundle) |
| Miner | cpuminer (C, SHA256d) |
| Wallet Service | wallet-headless (Node.js) |
| MCP Server | Embedded in Rust (Axum HTTP server) |

## Quick Start

### Prerequisites

- Node.js 18+
- Rust 1.70+ (for Tauri)
- Nix (recommended) or manual dependency management

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

Before running the app, build the required binaries:

```bash
# Build hathor-core standalone binary
build-core

# Build cpuminer binary
build-cpuminer

# Build wallet-headless for multi-wallet support
build-wallet-headless

# Build hathor-explorer for embedded explorer
build-explorer
```

### Production Build

```bash
build-release
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        HATHOR FORGE                              │
├─────────────────────────────────────────────────────────────────┤
│  React Frontend (port 1420 in dev)                               │
│  ├─ Dashboard with real-time stats                               │
│  ├─ Wallet Manager (create, fund, view balances)                 │
│  ├─ Block & Transaction Explorer                                 │
│  ├─ Log Viewer (filterable by service)                           │
│  └─ Faucet UI for sending HTR                                    │
├─────────────────────────────────────────────────────────────────┤
│  Tauri Backend (Rust)                                            │
│  ├─ Process management (node, miner, wallet-headless)            │
│  ├─ Event streaming via Tauri events                             │
│  ├─ HTTP proxy for fullnode API                                  │
│  └─ Embedded MCP server (port 9876)                              │
├─────────────────────────────────────────────────────────────────┤
│  Services                                                        │
│  ├─ hathor-core (fullnode, port 8080 API, port 8000 stratum)     │
│  ├─ cpuminer (connects to stratum)                               │
│  ├─ wallet-headless (port 8001, multi-wallet management)         │
│  └─ explorer-server (port 3001, embedded block explorer)         │
└─────────────────────────────────────────────────────────────────┘
```

## Services & Ports

| Service | Port | Description |
|---------|------|-------------|
| Fullnode API | 8080 | Hathor node REST API (`/v1a/`) |
| Stratum | 8000 | Mining stratum protocol |
| Wallet Headless | 8001 | Multi-wallet REST API |
| Explorer | 3001 | Block explorer web UI |
| MCP Server | 9876 | AI integration endpoint |
| Dev Server | 1420 | Vite dev server (development only) |

## Wallet Management

### Fullnode Wallet (Faucet)

The fullnode runs with a pre-configured HD wallet that receives all mining rewards:

```
Seed: avocado spot town typical traffic vault danger century property shallow divorce festival spend attack anchor afford rotate green audit adjust fade wagon depart level
```

**Default address:** `WXkMhVgRVmTXTVh47wauPKm1xcrW8Qf3Vb`

> **Warning:** This is a development-only wallet. Never use on mainnet or testnet.

### Multi-Wallet Support

The wallet-headless service enables creating multiple wallets:

1. Click "Start Service" in the Wallet tab
2. Click "New Wallet" to create a wallet (generates a new seed)
3. Use "Fund" to send HTR from the faucet
4. Use "Seed" to copy the wallet's seed phrase
5. Use "Expand" to view addresses and send transactions

## MCP Server (AI Integration)

Hathor Forge includes an embedded MCP (Model Context Protocol) server that allows AI assistants like Claude to fully control the development environment.

### Setup for Claude Code

```bash
# Add the MCP server to Claude Code
claude mcp add --transport http hathor-forge http://127.0.0.1:9876/mcp
```

Or add manually to your project's `.mcp.json`:

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

### Available MCP Tools (26 total)

| Category | Tools | Description |
|----------|-------|-------------|
| **Node** | `start_node`, `stop_node`, `get_node_status` | Control the fullnode |
| **Miner** | `start_miner`, `stop_miner`, `get_miner_status` | Control CPU mining |
| **Wallet Service** | `start_wallet_service`, `stop_wallet_service`, `get_wallet_service_status` | Control wallet-headless |
| **Wallets** | `generate_seed`, `create_wallet`, `get_wallet_seed`, `get_wallet_status`, `get_wallet_balance`, `get_wallet_addresses`, `send_from_wallet`, `close_wallet` | Manage multiple wallets |
| **Faucet** | `get_faucet_balance`, `send_from_faucet`, `fund_wallet` | Send HTR from fullnode wallet |
| **Blockchain** | `get_blocks`, `get_transaction` | Query blockchain data |
| **Utilities** | `quick_start`, `quick_stop`, `get_full_status`, `reset_data` | Convenience commands |

### Example: AI-Driven Development

Once configured, Claude can:

```
You: "Start the blockchain and create a test wallet with 50 HTR"

Claude: [Uses quick_start, create_wallet, fund_wallet tools]
        "Done! Node is running at block height 45, miner is active.
         Created wallet 'test' with 50 HTR. Seed: ..."

You: "Send 10 HTR to Wabc123..."

Claude: [Uses send_from_faucet tool]
        "Sent 10 HTR. Transaction: 6883215..."
```

### Testing with curl

```bash
# Health check
curl http://127.0.0.1:9876/health

# Quick start everything
curl -X POST http://127.0.0.1:9876/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"quick_start","arguments":{}}}'

# Get full status
curl -X POST http://127.0.0.1:9876/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_full_status","arguments":{}}}'
```

## Node Configuration

The fullnode runs with these flags for optimal local development:

| Flag | Purpose |
|------|---------|
| `--localnet` | Uses localnet genesis with fast block times |
| `--stratum 8000` | Enables stratum mining protocol |
| `--status 8080` | REST API port |
| `--allow-mining-without-peers` | Enables solo mining |
| `--test-mode-tx-weight` | Reduced transaction weight for fast testing |
| `--wallet-enable-api` | Enables wallet REST endpoints |
| `--wallet-index` | Indexes wallet transactions |
| `--unsafe-mode privatenet` | Disables security checks for local dev |

## API Reference

### Fullnode API (port 8080)

```bash
# Node status
GET http://127.0.0.1:8080/v1a/status

# Wallet balance
GET http://127.0.0.1:8080/v1a/wallet/balance

# Wallet address
GET http://127.0.0.1:8080/v1a/wallet/address

# Send transaction
POST http://127.0.0.1:8080/v1a/wallet/send_tokens/
Content-Type: application/json
{"data": {"inputs": [], "outputs": [{"address": "W...", "value": 1000}]}}

# Get transaction
GET http://127.0.0.1:8080/v1a/transaction?id=<tx_hash>

# Get block at height
GET http://127.0.0.1:8080/v1a/block_at_height?height=<n>
```

### Wallet Headless API (port 8001)

```bash
# Start a wallet
POST http://127.0.0.1:8001/start
Content-Type: application/json
{"wallet-id": "my-wallet", "seed": "24 word seed..."}

# Get wallet status (X-Wallet-Id header required)
GET http://127.0.0.1:8001/wallet/status

# Get balance
GET http://127.0.0.1:8001/wallet/balance

# Get addresses
GET http://127.0.0.1:8001/wallet/addresses

# Send transaction
POST http://127.0.0.1:8001/wallet/simple-send-tx
{"address": "W...", "value": 1000}

# Stop wallet
POST http://127.0.0.1:8001/wallet/stop
```

## Project Structure

```
hathor-dev-env/
├── src/                          # React frontend
│   ├── App.tsx                   # Main application component
│   ├── main.tsx                  # Entry point
│   └── index.css                 # Tailwind styles
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs                # Rust backend (process management, API proxy)
│   │   └── mcp.rs                # Embedded MCP server
│   ├── binaries/                 # Bundled executables (gitignored)
│   │   ├── hathor-core-*/        # Fullnode binary (PyInstaller onedir)
│   │   └── cpuminer-*            # Miner binary
│   ├── wallet-headless-dist/     # Wallet service (Node.js bundle)
│   ├── explorer-dist/            # Block explorer (static build)
│   └── tauri.conf.json           # Tauri configuration
├── scripts/
│   ├── build-hathor-core.sh      # Build fullnode binary
│   ├── build-cpuminer.sh         # Build miner binary
│   ├── build-wallet-headless.sh  # Build wallet service
│   └── build-explorer.sh         # Build block explorer
├── flake.nix                     # Nix development environment
└── package.json                  # Node.js dependencies
```

## Troubleshooting

### "Node failed to start"
- Check if ports 8080 or 8000 are already in use
- Try `reset_data` to clear blockchain data and start fresh

### "Wallet stuck at Starting"
- Ensure the fullnode is fully synced (check block height is increasing)
- The wallet needs the node to be ready before it can sync

### "Faucet has no funds"
- Start the miner and wait for a few blocks to be mined
- Mining rewards need 1 block confirmation before they're spendable

### "MCP server not connecting"
- Ensure Hathor Forge app is running
- Check `http://127.0.0.1:9876/health` returns "OK"
- Verify MCP config is in `.mcp.json`, not `settings.json`

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run `cargo check` and `npm run lint`
5. Submit a pull request

## License

MIT
