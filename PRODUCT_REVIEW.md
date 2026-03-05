# Product Review: Hathor Forge

## Release Blockers

1. **No code signing or notarization** - macOS builds are unsigned (Gatekeeper will block), Windows has no Authenticode signing (SmartScreen warnings). Users can't install without workarounds.

We'll deal with this next week. You can open an issue. Don't mention "claude" and be concise.

2. **CSP disabled** (`tauri.conf.json` has `"csp": null`) - The webview has zero content security restrictions. Critical security regression for an app controlling wallets and nodes.

I don't give a fuck, this is a development app, not a production-facing app.

3. **No release pipeline** - No GitHub Releases, no versioning automation, no changelog. The path from "merge to master" to "users download it" is completely manual and undocumented.

Ok, create an issue so I can spin up an agent to do it.

4. **Windows build is a placeholder** - Every step has `continue-on-error: true`. It always "passes" regardless of actual outcome.

Also create an issue. thanks.

5. **No auto-update mechanism** - Tauri 2.x supports `tauri-plugin-updater` but it's not configured. Users would need to manually check for updates.

I love it, create an issue describing this.

---

## Backend (Rust) - High Priority

6. **Race condition in `start_node_internal`** - Two concurrent start calls can spawn two nodes (TOCTOU between checking `node_running` and setting it).

Create an issue

7. **`kill_process` blocks the async runtime** - Uses `std::thread::sleep` while holding a `tokio::sync::Mutex`, starving other tasks for up to 5 seconds.

Create an issue

8. **MCP `reset_data` deletes the wrong directory** - Uses `~/.hathor-forge` instead of the actual data dir at `~/.local/share/hathor-forge/data`.

Create an issue

9. **No HTTP timeouts in MCP handlers** - If the fullnode hangs, all MCP calls hang indefinitely. Also creates a new `reqwest::Client` per request (no connection pooling).

Create an issue

10. **Nano contract API endpoint mismatch** - Commands use `nano-contract/create` (singular) while MCP uses `nano-contracts/create` (plural). Only one is correct.

Create an issue

11. **No structured logging** - Backend uses `eprintln!` everywhere. No log levels, no filtering, no MCP request audit trail.

Create an issue

12. **`node` binary assumed in PATH** - Wallet-headless spawns `node` directly. Fails silently on systems using nvm/volta/custom Node installs.

Create an issue

---

## Frontend - High Priority

13. **Wallet send form uses `document.getElementById`** instead of React controlled inputs - Anti-pattern, untestable, no validation.


14. **Dashboard stats are hardcoded** - "Transactions: 0" and "Tokens: 1" never update. Recent blocks show no real data (no hash, timestamp, or tx count).

15. **No transaction history** anywhere in the app. Core expectation for a blockchain dev tool.

16. **No block detail view** - Block list is decorative. Can't see transactions within a block.

17. **No blueprint deploy/publish flow** - Can view blueprints on network but can't upload new ones from the UI.

18. **Persisted nano contracts become stale after reset** - `useNanoContractStore` persists to localStorage but blockchain reset doesn't clear it.

19. **Settings page exit confirmation is dead code** - Modal exists but no button triggers it. `isShuttingDownRef` is declared but never read.

20. **`pollWalletStatus` has no cancellation** - Fire-and-forget polling loop runs for 30s with no cleanup if user navigates away.

Create a single issue for all of those

---

## Frontend - Medium Priority

21. **ErrorBanner has no dismiss button** - Stays visible until next successful operation.
22. **No toast/notification system** - Success/error feedback is easy to miss.
23. **No keyboard shortcuts or Escape-to-close** on modals.
24. **No onboarding/guidance** for first-time users (need to know: start node -> start miner -> wait for blocks -> then faucet has funds).
25. **No initial state reconciliation** - App always starts assuming everything is stopped, even if services are already running.
26. **Sidebar log badge shows total count, not unread.**
27. **Double padding** on Wallet/NanoContracts/Settings pages (`p-8` inside the global `p-6`).
28. **`any` types pervasive** in NanoContractsPage where proper types already exist.
29. **`NanoContract` type duplicated** between `types/nano-contracts.ts` and `useNanoContractStore.ts`.
30. **Node start logic duplicated** between DashboardPage and ExplorerPage.

Another single issue for all of those

---

## Testing & Quality

31. **Zero tests** - No Rust unit tests, no frontend tests, no integration tests. `npm run test` doesn't even exist in package.json.
32. **No linting in CI** - No ESLint config, no `tsc --noEmit`, no Clippy.
33. **No `cargo audit` or `npm audit`** - Security vulnerabilities in dependencies go undetected. GitHub already flags 20 vulnerabilities on the repo.

Great, single issue of all of those

---

## DevEx & Documentation

34. **README version errors** - Says React 18 (actually 19), Node.js 18+ (actually 22).
35. **No CHANGELOG, no CONTRIBUTING.md, no SECURITY.md, no issue templates.**
36. **No screenshots** in README (placeholder text only).
37. **`justfile` lint/test targets silently fail** - `npm run lint 2>/dev/null || true` swallows all errors.
38. **Three version strings** (Cargo.toml, tauri.conf.json, package.json) with no sync mechanism.
39. **Missing `[profile.release]`** in Cargo.toml (no LTO, no strip, no optimization settings).

Great, single issue of all of those

---

## Suggested Prioritization

1. **Security first**: Restore CSP, add HTTP timeouts, fix `reset_data` path
2. **Fix correctness bugs**: Race conditions, endpoint mismatches, stale state after reset
3. **Code signing + release pipeline**: This unblocks distribution
4. **Add basic tests + CI linting**: Prevent regressions
5. **UX gaps**: Onboarding, transaction history, block details, toast notifications
6. **Polish**: Keyboard shortcuts, error dismiss, responsive layout, state reconciliation
