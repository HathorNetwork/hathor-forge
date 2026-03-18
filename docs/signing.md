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

1. **SSL.com EV Code Signing certificate** — https://www.ssl.com/certificates/ev-code-signing/
   - EV certificates eliminate SmartScreen warnings immediately
   - The certificate is stored in SSL.com's cloud (eSigner) — no USB token or local PFX needed
2. **SSL.com CodeSignTool** — CLI for cloud-based signing via eSigner
   - Download from https://www.ssl.com/developer-tools/codesigntool-command-line-tool/
   - Extract to a known location (e.g. `C:\CodeSignTool\`)
   - Or add to PATH
3. **Node.js 22**, **Python 3.12**, **Rust (stable)**, **MSYS2** — for building
4. **TOTP secret** — for automated eSigner authentication (avoids manual OTP entry)
   - In SSL.com dashboard: go to your EV certificate order > eSigner > enable **TOTP** and save the secret

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
# 1. Set SSL.com eSigner credentials
$env:ESIGNER_USERNAME = "your@email.com"           # SSL.com account email
$env:ESIGNER_PASSWORD = "your-password"             # SSL.com account password
$env:ESIGNER_TOTP_SECRET = "your-totp-secret"       # TOTP secret from eSigner dashboard
$env:ESIGNER_CREDENTIAL_ID = "your-credential-id"   # Certificate credential ID

# 2. (Optional) Set CodeSignTool path if not in PATH
$env:CODESIGNTOOL_PATH = "C:\CodeSignTool"

# 3. Build and sign everything
.\scripts\sign\build-and-sign-windows.ps1
```

The script will:
1. Clone dependency repos (hathor-core, cpuminer, etc.)
2. Build all binaries (hathor-core, cpuminer, tx-mining-service, wallet-headless, explorer, node)
3. Sign all `.exe` and `.dll` files using SSL.com eSigner (cloud-based EV signing)
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
| `ESIGNER_USERNAME` | Yes | SSL.com account email |
| `ESIGNER_PASSWORD` | Yes | SSL.com account password |
| `ESIGNER_TOTP_SECRET` | Yes | TOTP secret for automated OTP generation (from eSigner dashboard) |
| `ESIGNER_CREDENTIAL_ID` | Yes | Certificate credential ID (from eSigner dashboard) |
| `CODESIGNTOOL_PATH` | No | Path to CodeSignTool directory (default: searches PATH) |

### Finding Your Credential ID

1. Log in to https://www.ssl.com/
2. Go to **Orders** > select your EV Code Signing certificate order
3. Click **eSigner** > the **Credential ID** is shown on that page
4. Enable **TOTP** and save the secret — this allows automated signing without manual OTP entry

### Setting Up CodeSignTool

```powershell
# Download from SSL.com
# Extract to C:\CodeSignTool (or any directory)
# Verify it works:
C:\CodeSignTool\CodeSignTool.bat get_credential_ids `
  -username "your@email.com" `
  -password "your-password" `
  -totp_secret "your-totp-secret"
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

With an EV certificate from SSL.com, SmartScreen warnings should be bypassed immediately. If you still see warnings, verify the signature is valid with `signtool verify /pa /v yourfile.exe`.

### Windows: CodeSignTool errors

- **"Invalid TOTP"**: The TOTP secret may have been regenerated. Log in to SSL.com dashboard and re-copy the secret.
- **"Credential ID not found"**: Make sure you're using the credential ID from the eSigner page, not the order number.
- **Timeout errors**: SSL.com's eSigner is cloud-based; signing requires network access. Check your internet connection and firewall.
- **"Malware detected"**: eSigner runs a malware scan before signing. If `cpuminer.exe` is rejected, you may need to submit a false-positive report to SSL.com support.

### Windows: "cpuminer.exe" flagged by antivirus

Even with a valid signature, some antivirus engines flag mining software heuristically. Code signing significantly reduces false positives. Users may still need to add an exclusion in their antivirus for the app's install directory.
