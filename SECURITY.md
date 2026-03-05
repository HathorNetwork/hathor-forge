# Security Policy

## Important Notice

Hathor Forge is a **local development tool** designed for use with localnet/privatenet configurations only. It is **not intended for use with mainnet or testnet** and includes development-only defaults (pre-configured wallets, reduced transaction weights, disabled security checks) that are unsafe for production use.

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 0.1.x   | Yes                |

## Reporting a Vulnerability

If you discover a security vulnerability in Hathor Forge, please report it responsibly.

**Do not open a public GitHub issue for security vulnerabilities.**

Instead, please send an email to:

**security@hathor.network**

Include the following in your report:

- A description of the vulnerability
- Steps to reproduce the issue
- The potential impact
- Any suggested fixes (optional)

## Response Timeline

- **Acknowledgment**: Within 3 business days of receiving the report.
- **Assessment**: We will evaluate the severity and impact within 10 business days.
- **Resolution**: Fixes for confirmed vulnerabilities will be prioritized based on severity.

## Scope

The following are in scope:

- The Tauri desktop application and CLI
- The embedded MCP server
- Build scripts and CI/CD pipeline
- Process management and service orchestration logic

The following are **out of scope** (report to the respective projects):

- Vulnerabilities in hathor-core (the Hathor fullnode) -- report to [hathor-core](https://github.com/HathorNetwork/hathor-core)
- Vulnerabilities in wallet-headless -- report to [hathor-wallet-headless](https://github.com/HathorNetwork/hathor-wallet-headless)
- Vulnerabilities in cpuminer

## Disclosure Policy

We follow a coordinated disclosure approach. We ask that you:

1. Allow us reasonable time to investigate and fix the issue before public disclosure.
2. Make a good-faith effort to avoid privacy violations, data destruction, or service disruption.
3. Do not exploit the vulnerability beyond what is necessary to demonstrate it.

We will credit reporters in the release notes unless they prefer to remain anonymous.
