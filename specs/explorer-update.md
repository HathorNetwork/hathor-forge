# Spec: Update Explorer with Master

## 1. Overview
The embedded Hathor Explorer in Hathor Forge should be kept in sync with the latest features, bug fixes, and improvements from the official `hathor-explorer` repository.

## 2. Goals
- Update the `hathor-explorer` source reference to the latest commit on the `master` branch.
- Ensure the build process remains compatible with the latest explorer code.
- Verify that the proxy server in Hathor Forge correctly handles any new API routes or WebSocket patterns used by the updated explorer.

## 3. Technical Tasks

### 3.1. Source Update
- Update `flake.nix` to fetch the latest master of `hathornetwork/hathor-explorer`.
- Run `nix flake update` to synchronize the lockfile.

### 3.2. Build Process Verification
- Review `scripts/build-explorer.sh`.
- Check if any new environment variables are required by the latest explorer version (e.g., new `REACT_APP_*` flags).
- Verify that `npm install` and `npm run build` still work as expected within the Nix environment.

### 4.3. Backend Proxy Compatibility
- The Hathor Forge backend (`src-tauri/src/lib.rs`) currently proxies requests to the fullnode on port 3001.
- Check if the new explorer version introduces:
    - New API endpoints that need specific proxy handling.
    - Changes to WebSocket handshake or messaging patterns.
    - Different routing logic that might conflict with the `iframe` embedding.

### 3.4. Testing
- Run the fullnode and miner.
- Access the Explorer tab in Hathor Forge.
- Verify all main features:
    - Block listing and details.
    - Transaction listing and details.
    - Token listing and details.
    - Search functionality.
    - Real-time updates via WebSockets.

## 4. Risks & Mitigations
- **Breaking UI Changes**: The new explorer might have a layout that doesn't fit well in the current `iframe` view. *Mitigation*: Adjust `src/App.tsx` styling or the explorer's `basic` mode configuration.
- **Dependency Conflicts**: Newer explorer versions might require a newer Node.js version. *Mitigation*: The project already uses Node 22 in `flake.nix`, which should be sufficient.
