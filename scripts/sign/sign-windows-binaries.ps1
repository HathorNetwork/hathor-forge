# Sign all bundled binaries for Windows using SSL.com eSigner (CodeSignTool).
# Must be run AFTER building binaries, BEFORE building the Tauri app.
#
# Required env:
#   ESIGNER_USERNAME      - SSL.com account email
#   ESIGNER_PASSWORD      - SSL.com account password
#   ESIGNER_TOTP_SECRET   - TOTP secret for automated OTP
#   ESIGNER_CREDENTIAL_ID - Certificate credential ID
#
# Optional env:
#   CODESIGNTOOL_PATH     - Path to CodeSignTool directory (default: searches PATH)
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectDir = Split-Path -Parent (Split-Path -Parent $ScriptDir)
$BinariesDir = Join-Path $ProjectDir "src-tauri\binaries"
$Target = "x86_64-pc-windows-msvc"

# Validate eSigner credentials
$requiredVars = @("ESIGNER_USERNAME", "ESIGNER_PASSWORD", "ESIGNER_TOTP_SECRET", "ESIGNER_CREDENTIAL_ID")
foreach ($var in $requiredVars) {
    if (-not (Get-Item "env:$var" -ErrorAction SilentlyContinue)) {
        Write-Error @"
Missing required environment variable: $var

Required variables for SSL.com eSigner:
  `$env:ESIGNER_USERNAME      = "your@email.com"
  `$env:ESIGNER_PASSWORD      = "your-password"
  `$env:ESIGNER_TOTP_SECRET   = "your-totp-secret"
  `$env:ESIGNER_CREDENTIAL_ID = "your-credential-id"
"@
        exit 1
    }
}

# Find CodeSignTool
$CodeSignTool = $null
if ($env:CODESIGNTOOL_PATH) {
    $CodeSignTool = Join-Path $env:CODESIGNTOOL_PATH "CodeSignTool.bat"
} else {
    # Search PATH
    $CodeSignTool = Get-Command "CodeSignTool.bat" -ErrorAction SilentlyContinue
    if ($CodeSignTool) { $CodeSignTool = $CodeSignTool.Source }
}
if (-not $CodeSignTool -or -not (Test-Path $CodeSignTool)) {
    Write-Error @"
CodeSignTool.bat not found.
Download from: https://www.ssl.com/developer-tools/codesigntool-command-line-tool/
Set `$env:CODESIGNTOOL_PATH to the directory containing CodeSignTool.bat
"@
    exit 1
}

Write-Host "=== Signing Windows binaries (SSL.com eSigner) ==="
Write-Host "CodeSignTool: $CodeSignTool"
Write-Host ""

$signCount = 0

function Sign-File {
    param([string]$File)
    if (-not (Test-Path $File)) { return }
    $relPath = $File.Replace($ProjectDir + "\", "")
    Write-Host "  Signing: $relPath"
    & $CodeSignTool sign `
        -username $env:ESIGNER_USERNAME `
        -password $env:ESIGNER_PASSWORD `
        -totp_secret $env:ESIGNER_TOTP_SECRET `
        -credential_id $env:ESIGNER_CREDENTIAL_ID `
        -input_file_path $File `
        -override
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

# Verify a sample using signtool if available
$SignTool = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin\*\x64\signtool.exe" -ErrorAction SilentlyContinue |
    Sort-Object FullName -Descending | Select-Object -First 1
if (-not $SignTool) {
    $SignTool = Get-Command signtool.exe -ErrorAction SilentlyContinue
}
if ($SignTool -and (Test-Path $Cpuminer)) {
    $SignToolPath = if ($SignTool.FullName) { $SignTool.FullName } else { $SignTool.Source }
    Write-Host "Verifying cpuminer signature..."
    & $SignToolPath verify /pa /v $Cpuminer 2>&1 | Select-Object -Last 3
}
