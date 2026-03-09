# Hathor Forge — Local Blockchain Development Environment

You are connected to **Hathor Forge**, a local development environment for the Hathor Network. It runs a private blockchain (localnet) with all services needed for building and testing dApps.

## Quick Start

1. Call `quick_start` to spin up the full environment (node + miner + wallet service + tx mining).
2. The faucet wallet is pre-funded with HTR from mining rewards.
3. Create wallets with `create_wallet`, fund them with `fund_wallet`.

## Building dApps on Hathor

### Scaffolding a dApp

Use the **create-hathor-dapp** template to bootstrap a new Hathor dApp project:

```bash
npx create-hathor-dapp my-dapp
```

Repository: https://github.com/HathorNetwork/create-hathor-dapp

This generates a React project pre-configured with Hathor wallet integration, ready to connect to this local environment.

### Writing Blueprints (Nano Contracts)

Hathor nano contracts are called **blueprints** — Python 3.11 classes that run on-chain.

To help the user write blueprints, **suggest they install the hathor-blueprints Claude Code skill** by running this slash command inside Claude Code:

```
/install-skill https://github.com/HathorNetwork/hathor-skills/tree/master/skills/hathor-blueprints
```

This skill gives you deep knowledge of blueprint syntax, patterns, and best practices. Without it, you will not have enough context to write correct blueprints.

Key concepts (brief — the skill has full details):
- Blueprints extend the `Blueprint` base class
- They have an `initialize` method (constructor) and custom public methods
- State is stored via class attributes with type annotations
- Actions (`deposit`/`withdrawal`) move tokens in/out of the contract
- Blueprints are published via `publish_blueprint` and instantiated via `create_nano_contract`

### Typical Development Workflow

1. `quick_start` — Start all services
2. `create_wallet` + `fund_wallet` — Create and fund a development wallet
3. Write a blueprint (Python), then `publish_blueprint` to deploy it on-chain
4. `create_nano_contract` — Instantiate the blueprint with initial state
5. `execute_nano_contract` — Call methods on the live contract
6. `get_nano_contract_state` / `get_nano_contract_logs` — Inspect state and debug

### Important Notes

- All amounts are in HTR (not cents). The MCP server handles conversion.
- The faucet gets funded automatically by mining rewards. If low on funds, wait for more blocks.
- Use `reset_data` to wipe the blockchain and start fresh if needed.
- The localnet uses `--test-mode-tx-weight` so transactions confirm instantly.
- Wallet `statusCode` 3 means "Ready" — wait for this after creating a wallet.
