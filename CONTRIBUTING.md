# Contributing to Hathor Forge

Thank you for your interest in contributing to Hathor Forge! This guide will help you get started.

## Development Environment Setup

### Prerequisites

- [Nix](https://nixos.org/download.html) package manager (recommended)
- Alternatively: Node.js 22+, Rust 1.70+, and platform-specific dependencies

### Getting Started

```bash
# Clone the repository
git clone https://github.com/HathorNetwork/hathor-forge.git
cd hathor-forge

# Enter the Nix development shell (auto-loads with direnv)
nix develop

# Install npm dependencies
npm install

# Build required service binaries
build-core            # hathor-core fullnode binary
build-cpuminer        # CPU miner binary
build-wallet-headless # Wallet service
build-explorer        # Block explorer

# Start the development server
dev-server
```

## Code Style

### Rust

- Format with `cargo fmt --manifest-path src-tauri/Cargo.toml`
- Check with `cargo check --manifest-path src-tauri/Cargo.toml`
- Run tests with `cargo test --manifest-path src-tauri/Cargo.toml`

### TypeScript / React

- Lint with `npm run lint`
- Auto-fix with `npm run lint:fix`
- The frontend uses the `@/` path alias for `src/` (configured in `vite.config.ts`)

### Quick Check

Run all checks at once using the justfile:

```bash
just check    # cargo check + npm lint + version sync
just fmt      # cargo fmt + npm lint:fix
just test     # cargo test + npm test
```

## Commit Message Format

Use clear, descriptive commit messages. Follow these conventions:

- **feat:** A new feature
- **fix:** A bug fix
- **docs:** Documentation-only changes
- **refactor:** Code changes that neither fix a bug nor add a feature
- **test:** Adding or updating tests
- **chore:** Build process, tooling, or dependency changes

Examples:

```
feat: add token creation support to MCP server
fix: resolve miner crash when node disconnects
docs: update API reference with new wallet endpoints
```

## Pull Request Guidelines

1. **Create a feature branch** from `master`:
   ```bash
   git checkout -b feat/my-feature
   ```

2. **Make focused changes** -- keep PRs small and reviewable.

3. **Run checks before submitting**:
   ```bash
   just check
   just test
   ```

4. **Write a clear PR description** explaining what changed and why.

5. **Link related issues** using `Fixes #123` or `Closes #123` in the PR body.

## Building Binaries

The app bundles several external services as binaries. Each has a dedicated build script:

| Binary | Build Command | Source |
|--------|--------------|--------|
| hathor-core | `build-core` | PyInstaller bundle of the Hathor fullnode |
| cpuminer | `build-cpuminer` | Native C miner (SHA256d) |
| wallet-headless | `build-wallet-headless` | Node.js wallet service |
| explorer | `build-explorer` | Static React build of the block explorer |

Built binaries are placed in `src-tauri/binaries/` (gitignored).

## Project Structure

- `src/` -- React frontend (TypeScript, Tailwind CSS)
- `src-tauri/src/lib.rs` -- Shared Rust backend (process management, state)
- `src-tauri/src/mcp.rs` -- MCP server implementation
- `src-tauri/src/cli_main.rs` -- CLI entry point
- `src-tauri/src/tauri_app.rs` -- Tauri GUI entry point
- `scripts/` -- Build scripts for service binaries
- `justfile` -- Development task runner

## Questions?

Open an issue on GitHub or start a discussion. We are happy to help!
