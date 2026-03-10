# Signing Hathor Forge Releases

This guide explains how to build and sign Hathor Forge for distribution on macOS and Windows.

## Overview

Hathor Forge bundles several binaries that all need to be signed:

| Binary | Type | Platforms |
|--------|------|-----------|
| `hathor-core` | PyInstaller onedir (Python + .dylib/.dll/.so) | All |
| `cpuminer` | Native C binary | All |
| `tx-mining-service` | PyInstaller onedir | All |
| `node` | Node.js binary | All |
| `hathor-forge` | Tauri app (Rust) | All |
| Installer | DMG (macOS), NSIS .exe / MSI (Windows) | Per-platform |

> **Why sign?** On macOS, unsigned apps are blocked by Gatekeeper. On Windows, `cpuminer.exe` is flagged by Windows Defender as mining malware unless signed with a trusted certificate.

---

## Prerequisites

### macOS

1. **Apple Developer account** (paid, $99/year) — https://developer.apple.com
2. **Developer ID Application certificate** — for signing the app
3. **Developer ID Installer certificate** — for signing the DMG (optional but recommended)
4. **App-specific password** for notarization — https://appleid.apple.com > Sign-In and Security > App-Specific Passwords
5. **Nix** installed — the build uses `nix develop` for reproducible dependencies
6. **Xcode command line tools** — `xcode-select --install`

To create the certificates:
- Open **Keychain Access** > **Certificate Assistant** > **Request a Certificate from a Certificate Authority**
- Go to https://developer.apple.com/account/resources/certificates > create **Developer ID Application** and **Developer ID Installer** certificates
- Download and double-click to install in your Keychain

Verify your certificates:
```bash
security find-identity -v -p codesigning
# Should show: "Developer ID Application: Your Name (TEAM_ID)"
```

### Windows

1. **Code signing certificate** — EV (Extended Validation) recommended, standard OV also works
   - Providers: DigiCert, Sectigo, GlobalSign, SSL.com
   - EV certificates eliminate SmartScreen warnings immediately
   - OV certificates require reputation building
2. **Windows SDK** — provides `signtool.exe`
3. **Node.js 22**, **Python 3.12**, **Rust (stable)**, **MSYS2** — for building
4. The certificate must be installed in the Windows certificate store or available as a PFX file

---

## macOS: Build & Sign

### Quick Start

```bash
# 1. Enter the Nix dev shell
cd hathor-forge
nix develop

# 2. Set your signing identity and notarization credentials
export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAM_ID)"
export APPLE_ID="your@email.com"
export APPLE_PASSWORD="xxxx-xxxx-xxxx-xxxx"  # App-specific password
export APPLE_TEAM_ID="XXXXXXXXXX"

# 3. Build and sign everything
./scripts/sign/build-and-sign-macos.sh
```

The script will:
1. Build all binaries (hathor-core, cpuminer, tx-mining-service, wallet-headless, explorer, node)
2. Deep-sign all bundled binaries and libraries with your Developer ID
3. Build the Tauri app (which uses the pre-signed binaries)
4. Sign the final `.app` bundle
5. Create a signed DMG
6. Submit for Apple notarization and wait for approval
7. Staple the notarization ticket to the DMG

Output: `dist/Hathor-Forge-signed.dmg`

### Manual Step-by-Step

If you prefer to run each step manually:

```bash
# Build binaries
./scripts/build-hathor-core.sh
./scripts/build-cpuminer.sh
./scripts/build-tx-mining-service.sh
./scripts/build-wallet-headless.sh
./scripts/build-explorer.sh
./scripts/build-node.sh

# Sign the bundled binaries
./scripts/sign/sign-macos-binaries.sh

# Build the Tauri app
npm ci
npm run tauri build -- --bundles app

# Sign, create DMG, notarize
./scripts/sign/sign-macos-app.sh
```

### Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `APPLE_SIGNING_IDENTITY` | Yes | Certificate name, e.g. `Developer ID Application: Hathor Labs (TEAM_ID)` |
| `APPLE_ID` | Yes | Apple ID email for notarization |
| `APPLE_PASSWORD` | Yes | App-specific password for notarization |
| `APPLE_TEAM_ID` | Yes | 10-character Team ID from developer.apple.com |

---

## Windows: Build & Sign

### Quick Start

Open **PowerShell as Administrator**:

```powershell
# 1. Set your certificate thumbprint (from certmgr.msc or signtool)
$env:SIGN_CERT_THUMBPRINT = "XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
# Or use a PFX file:
# $env:SIGN_PFX_PATH = "C:\path\to\certificate.pfx"
# $env:SIGN_PFX_PASSWORD = "your-password"

# 2. Set timestamp server (default: DigiCert)
$env:SIGN_TIMESTAMP_URL = "http://timestamp.digicert.com"

# 3. Build and sign everything
.\scripts\sign\build-and-sign-windows.ps1
```

The script will:
1. Clone dependency repos (hathor-core, cpuminer, etc.)
2. Build all binaries (hathor-core, cpuminer, tx-mining-service, wallet-headless, explorer, node)
3. Sign all `.exe` and `.dll` files in the bundles
4. Build the Tauri app (NSIS + MSI installers)
5. Sign the installers

Output: `dist\Hathor-Forge-signed-setup.exe` and `dist\Hathor-Forge-signed.msi`

### Manual Step-by-Step

```powershell
# Build binaries
.\scripts\windows\build-hathor-core.ps1
.\scripts\windows\build-cpuminer.sh   # Run in MSYS2
.\scripts\windows\build-wallet-headless.ps1
.\scripts\windows\build-tx-mining-service.ps1
.\scripts\windows\build-explorer.ps1
.\scripts\windows\build-node.ps1

# Sign the bundled binaries
.\scripts\sign\sign-windows-binaries.ps1

# Build the Tauri app
npm ci
npm run tauri build

# Sign the installers
.\scripts\sign\sign-windows-installers.ps1
```

### Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `SIGN_CERT_THUMBPRINT` | Yes* | SHA1 thumbprint of the certificate in the Windows cert store |
| `SIGN_PFX_PATH` | Yes* | Path to PFX file (alternative to thumbprint) |
| `SIGN_PFX_PASSWORD` | If PFX | Password for the PFX file |
| `SIGN_TIMESTAMP_URL` | No | Timestamp server URL (default: `http://timestamp.digicert.com`) |

\* Provide either `SIGN_CERT_THUMBPRINT` or `SIGN_PFX_PATH`.

### Finding Your Certificate Thumbprint

```powershell
# List all code signing certificates
Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert | Format-Table Thumbprint, Subject

# Or use signtool
signtool list /v
```

---

## Verifying Signatures

### macOS

```bash
# Verify code signature
codesign --verify --deep --strict --verbose=2 "Hathor Forge.app"

# Check notarization status
spctl --assess --type open --context context:primary-signature -v "Hathor Forge.app"

# Verify DMG
spctl --assess --type open --context context:primary-signature -v Hathor-Forge-signed.dmg
```

### Windows

```powershell
# Verify signature on an executable
signtool verify /pa /v .\hathor-core.exe

# Check signature in PowerShell
Get-AuthenticodeSignature .\hathor-core.exe
```

---

## Troubleshooting

### macOS: "Developer ID Application" certificate not found

Make sure the certificate is installed in your login keychain (not system):
```bash
security find-identity -v -p codesigning
```

If it shows "0 valid identities found", re-download the certificate from developer.apple.com.

### macOS: Notarization fails with "The binary is not signed"

All binaries inside the `.app` must be signed — including `.dylib` and `.so` files inside PyInstaller bundles. The `sign-macos-binaries.sh` script handles this.

### macOS: "hardened runtime" errors

The signing scripts enable hardened runtime (`--options runtime`) which is required for notarization. If a binary crashes after signing, it may need entitlements (e.g., `com.apple.security.cs.allow-unsigned-executable-memory`).

### Windows: SmartScreen still shows warning

- **EV certificates**: SmartScreen warnings are bypassed immediately.
- **OV certificates**: SmartScreen builds reputation over time. After enough downloads, the warning goes away.

### Windows: "cpuminer.exe" flagged by antivirus

Even with a valid signature, some antivirus engines flag mining software heuristically. Code signing significantly reduces false positives. Users may still need to add an exclusion in their antivirus for the app's install directory.
