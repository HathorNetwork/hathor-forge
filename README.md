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
# Install dependencies
npm install

# Run in development mode
npm run tauri dev
```

### Building Binaries

Before running the app, you need to build the hathor-core and cpuminer binaries:

```bash
# Build hathor-core (requires hathor-core source at ../hathor-core)
./scripts/build-hathor-core.sh

# Build cpuminer (requires cpuminer source at ../cpuminer)
./scripts/build-cpuminer.sh
```

### Production Build

```bash
npm run tauri build
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

## Project Structure

```
hathor-dev-env/
├── src/                    # React frontend
│   ├── App.tsx             # Main app with dashboard
│   └── main.tsx            # Entry point
├── src-tauri/
│   ├── src/
│   │   └── lib.rs          # Rust backend (process management)
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
