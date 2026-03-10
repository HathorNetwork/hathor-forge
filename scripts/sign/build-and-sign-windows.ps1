# Full build + sign pipeline for Windows.
#
# Required env (one of):
#   SIGN_CERT_THUMBPRINT - SHA1 thumbprint of cert in Windows cert store
#   SIGN_PFX_PATH        - Path to PFX file (+ SIGN_PFX_PASSWORD)
#
# Optional env:
#   SIGN_TIMESTAMP_URL   - Timestamp server (default: http://timestamp.digicert.com)
#   HATHOR_CORE_SRC      - Path to hathor-core source (default: ..\hathor-core)
#   CPUMINER_SRC         - Path to cpuminer source (default: ..\cpuminer)
#   etc.
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectDir = Split-Path -Parent (Split-Path -Parent $ScriptDir)

# Validate signing is configured
if (-not $env:SIGN_CERT_THUMBPRINT -and -not $env:SIGN_PFX_PATH) {
    Write-Error @"

No signing certificate configured. Set one of:

  `$env:SIGN_CERT_THUMBPRINT = "your-cert-thumbprint"
  `$env:SIGN_PFX_PATH = "C:\path\to\cert.pfx"

To find your thumbprint:
  Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert | Format-Table Thumbprint, Subject

"@
    exit 1
}

Set-Location $ProjectDir

Write-Host "============================================"
Write-Host "  Hathor Forge - Windows Build & Sign"
Write-Host "============================================"
Write-Host ""

# Step 0: Clone dependencies if not present
Write-Host "=== [0/5] Checking dependencies ==="
$deps = @(
    @{ Name = "hathor-core"; Repo = "https://github.com/HathorNetwork/hathor-core.git" },
    @{ Name = "cpuminer"; Repo = "https://github.com/HathorNetwork/cpuminer.git" },
    @{ Name = "hathor-wallet-headless"; Repo = "https://github.com/HathorNetwork/hathor-wallet-headless.git" },
    @{ Name = "tx-mining-service"; Repo = "https://github.com/HathorNetwork/tx-mining-service.git" },
    @{ Name = "hathor-explorer"; Repo = "https://github.com/HathorNetwork/hathor-explorer.git" }
)
foreach ($dep in $deps) {
    $depDir = Join-Path (Split-Path -Parent $ProjectDir) $dep.Name
    if (-not (Test-Path $depDir)) {
        Write-Host "  Cloning $($dep.Name)..."
        git clone --depth 1 $dep.Repo $depDir
    } else {
        Write-Host "  $($dep.Name) already present"
    }
}
Write-Host ""

# Step 1: Build all binaries
Write-Host "=== [1/5] Building binaries ==="
Write-Host ""

Write-Host "--- hathor-core ---"
& "$ProjectDir\scripts\windows\build-hathor-core.ps1"
Write-Host ""

# cpuminer needs MSYS2 — print instructions if not available
$msys2 = "C:\msys64\usr\bin\bash.exe"
if (Test-Path $msys2) {
    Write-Host "--- cpuminer (MSYS2) ---"
    $cpuminerSrc = Join-Path (Split-Path -Parent $ProjectDir) "cpuminer"
    & $msys2 -lc "export CPUMINER_SRC='$(($cpuminerSrc -replace '\\','/') -replace 'C:','/c')'; cd '$(($ProjectDir -replace '\\','/') -replace 'C:','/c')'; bash ./scripts/windows/build-cpuminer.sh"
    Write-Host ""
} else {
    Write-Host "--- cpuminer ---"
    Write-Host "  WARNING: MSYS2 not found at C:\msys64. Skipping cpuminer build."
    Write-Host "  Install MSYS2 from https://www.msys2.org/ and install mingw-w64-x86_64-toolchain"
    Write-Host "  Or build cpuminer manually and place it at src-tauri\binaries\cpuminer-x86_64-pc-windows-msvc.exe"
    Write-Host ""
}

Write-Host "--- wallet-headless ---"
& "$ProjectDir\scripts\windows\build-wallet-headless.ps1"
Write-Host ""

Write-Host "--- tx-mining-service ---"
& "$ProjectDir\scripts\windows\build-tx-mining-service.ps1"
Write-Host ""

Write-Host "--- explorer ---"
& "$ProjectDir\scripts\windows\build-explorer.ps1"
Write-Host ""

Write-Host "--- node binary ---"
& "$ProjectDir\scripts\windows\build-node.ps1"
Write-Host ""

# Step 2: Sign bundled binaries
Write-Host "=== [2/5] Signing bundled binaries ==="
Write-Host ""
& "$ProjectDir\scripts\sign\sign-windows-binaries.ps1"
Write-Host ""

# Step 3: Build Tauri app
Write-Host "=== [3/5] Building Tauri app ==="
Write-Host ""
npm ci
npm run tauri build
Write-Host ""

# Step 4: Sign installers
Write-Host "=== [4/5] Signing installers ==="
Write-Host ""
& "$ProjectDir\scripts\sign\sign-windows-installers.ps1"
Write-Host ""

# Step 5: Summary
Write-Host "=== [5/5] Summary ==="
$distDir = Join-Path $ProjectDir "dist"
Write-Host ""
if (Test-Path $distDir) {
    Get-ChildItem $distDir | ForEach-Object {
        $size = [math]::Round($_.Length / 1MB, 1)
        Write-Host "  $($_.Name)  (${size} MB)"
    }
    Write-Host ""
    Write-Host "Ready for distribution!"
} else {
    Write-Host "Error: dist\ directory not found"
    exit 1
}
