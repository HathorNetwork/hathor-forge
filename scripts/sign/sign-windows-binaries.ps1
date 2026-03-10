# Sign all bundled binaries for Windows with an Authenticode certificate.
# Must be run AFTER building binaries, BEFORE building the Tauri app.
#
# Required env (one of):
#   SIGN_CERT_THUMBPRINT - SHA1 thumbprint of cert in Windows cert store
#   SIGN_PFX_PATH        - Path to PFX file (+ SIGN_PFX_PASSWORD)
#
# Optional env:
#   SIGN_TIMESTAMP_URL   - Timestamp server (default: http://timestamp.digicert.com)
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectDir = Split-Path -Parent (Split-Path -Parent $ScriptDir)
$BinariesDir = Join-Path $ProjectDir "src-tauri\binaries"
$Target = "x86_64-pc-windows-msvc"
$TimestampUrl = if ($env:SIGN_TIMESTAMP_URL) { $env:SIGN_TIMESTAMP_URL } else { "http://timestamp.digicert.com" }

# Validate signing method
if (-not $env:SIGN_CERT_THUMBPRINT -and -not $env:SIGN_PFX_PATH) {
    Write-Error @"
No signing certificate configured. Set one of:
  `$env:SIGN_CERT_THUMBPRINT = "your-cert-thumbprint"
  `$env:SIGN_PFX_PATH = "C:\path\to\cert.pfx"
"@
    exit 1
}

# Find signtool.exe
$SignTool = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin\*\x64\signtool.exe" -ErrorAction SilentlyContinue |
    Sort-Object FullName -Descending | Select-Object -First 1
if (-not $SignTool) {
    $SignTool = Get-Command signtool.exe -ErrorAction SilentlyContinue
}
if (-not $SignTool) {
    Write-Error "signtool.exe not found. Install the Windows SDK."
    exit 1
}
$SignToolPath = if ($SignTool.FullName) { $SignTool.FullName } else { $SignTool.Source }

Write-Host "=== Signing Windows binaries ==="
Write-Host "SignTool: $SignToolPath"
Write-Host "Timestamp: $TimestampUrl"
Write-Host ""

# Build signtool arguments
function Get-SignArgs {
    param([string]$File)
    $args = @("sign", "/fd", "SHA256", "/tr", $TimestampUrl, "/td", "SHA256")
    if ($env:SIGN_CERT_THUMBPRINT) {
        $args += "/sha1", $env:SIGN_CERT_THUMBPRINT
    } else {
        $args += "/f", $env:SIGN_PFX_PATH
        if ($env:SIGN_PFX_PASSWORD) {
            $args += "/p", $env:SIGN_PFX_PASSWORD
        }
    }
    $args += $File
    return $args
}

$signCount = 0

function Sign-File {
    param([string]$File)
    if (-not (Test-Path $File)) { return }
    $relPath = $File.Replace($ProjectDir + "\", "")
    Write-Host "  Signing: $relPath"
    $signArgs = Get-SignArgs -File $File
    & $SignToolPath @signArgs | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Failed to sign: $File" }
    $script:signCount++
}

# --- hathor-core onedir bundle ---
$HathorCoreDir = Join-Path $BinariesDir "hathor-core-$Target"
if (Test-Path $HathorCoreDir) {
    Write-Host "[hathor-core]"
    # Sign all .exe and .dll files
    Get-ChildItem $HathorCoreDir -Recurse -Include "*.exe","*.dll","*.pyd" | ForEach-Object {
        Sign-File $_.FullName
    }
    Write-Host ""
}

# --- tx-mining-service onedir bundle ---
$TxMiningDir = Join-Path $BinariesDir "tx-mining-service-$Target"
if (Test-Path $TxMiningDir) {
    Write-Host "[tx-mining-service]"
    Get-ChildItem $TxMiningDir -Recurse -Include "*.exe","*.dll","*.pyd" | ForEach-Object {
        Sign-File $_.FullName
    }
    Write-Host ""
}

# --- cpuminer ---
$Cpuminer = Join-Path $BinariesDir "cpuminer-$Target.exe"
if (Test-Path $Cpuminer) {
    Write-Host "[cpuminer]"
    Sign-File $Cpuminer
    Write-Host ""
}

# --- node ---
$NodeBin = Join-Path $BinariesDir "node-$Target.exe"
if (Test-Path $NodeBin) {
    Write-Host "[node]"
    Sign-File $NodeBin
    Write-Host ""
}

Write-Host "=== Signing complete: $signCount files signed ==="
Write-Host ""

# Verify a sample
Write-Host "Verifying cpuminer signature..."
if (Test-Path $Cpuminer) {
    & $SignToolPath verify /pa /v $Cpuminer 2>&1 | Select-Object -Last 3
}
