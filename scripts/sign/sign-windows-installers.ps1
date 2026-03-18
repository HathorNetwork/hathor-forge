# Sign the Tauri-generated Windows installers (NSIS .exe and MSI)
# using SSL.com eSigner (CodeSignTool).
# Must be run AFTER `npm run tauri build`.
#
# Required env:
#   ESIGNER_USERNAME      - SSL.com account email
#   ESIGNER_PASSWORD      - SSL.com account password
#   ESIGNER_TOTP_SECRET   - TOTP secret for automated OTP
#   ESIGNER_CREDENTIAL_ID - Certificate credential ID
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectDir = Split-Path -Parent (Split-Path -Parent $ScriptDir)
$BundleDir = Join-Path $ProjectDir "src-tauri\target\release\bundle"
$DistDir = Join-Path $ProjectDir "dist"

# Validate eSigner credentials
$requiredVars = @("ESIGNER_USERNAME", "ESIGNER_PASSWORD", "ESIGNER_TOTP_SECRET", "ESIGNER_CREDENTIAL_ID")
foreach ($var in $requiredVars) {
    if (-not (Get-Item "env:$var" -ErrorAction SilentlyContinue)) {
        Write-Error "Missing required environment variable: $var. See sign-windows-binaries.ps1 for details."
        exit 1
    }
}

# Find CodeSignTool
$CodeSignTool = $null
if ($env:CODESIGNTOOL_PATH) {
    $CodeSignTool = Join-Path $env:CODESIGNTOOL_PATH "CodeSignTool.bat"
} else {
    $CodeSignTool = Get-Command "CodeSignTool.bat" -ErrorAction SilentlyContinue
    if ($CodeSignTool) { $CodeSignTool = $CodeSignTool.Source }
}
if (-not $CodeSignTool -or -not (Test-Path $CodeSignTool)) {
    Write-Error "CodeSignTool.bat not found. Set `$env:CODESIGNTOOL_PATH or add it to PATH."
    exit 1
}

function Sign-File {
    param([string]$File)
    Write-Host "  Signing: $(Split-Path -Leaf $File)"
    & $CodeSignTool sign `
        -username $env:ESIGNER_USERNAME `
        -password $env:ESIGNER_PASSWORD `
        -totp_secret $env:ESIGNER_TOTP_SECRET `
        -credential_id $env:ESIGNER_CREDENTIAL_ID `
        -input_file_path $File `
        -override
    if ($LASTEXITCODE -ne 0) { throw "Failed to sign: $File" }
}

Write-Host "=== Signing Windows installers (SSL.com eSigner) ==="
Write-Host ""

New-Item -ItemType Directory -Force -Path $DistDir | Out-Null

# Sign and copy NSIS installer
$NsisDir = Join-Path $BundleDir "nsis"
if (Test-Path $NsisDir) {
    $NsisExe = Get-ChildItem $NsisDir -Filter "*.exe" | Select-Object -First 1
    if ($NsisExe) {
        Write-Host "[NSIS installer]"
        Sign-File $NsisExe.FullName
        $DestName = "Hathor-Forge-signed-setup.exe"
        Copy-Item $NsisExe.FullName (Join-Path $DistDir $DestName)
        Write-Host "  Output: dist\$DestName"
        Write-Host ""
    }
}

# Sign and copy MSI installer
$MsiDir = Join-Path $BundleDir "msi"
if (Test-Path $MsiDir) {
    $MsiFile = Get-ChildItem $MsiDir -Filter "*.msi" | Select-Object -First 1
    if ($MsiFile) {
        Write-Host "[MSI installer]"
        Sign-File $MsiFile.FullName
        $DestName = "Hathor-Forge-signed.msi"
        Copy-Item $MsiFile.FullName (Join-Path $DistDir $DestName)
        Write-Host "  Output: dist\$DestName"
        Write-Host ""
    }
}

# Also sign the main Tauri executable
$MainExe = Join-Path $ProjectDir "src-tauri\target\release\hathor-forge.exe"
if (Test-Path $MainExe) {
    Write-Host "[Main executable]"
    Sign-File $MainExe
    Write-Host ""
}

Write-Host "=== Done ==="
Write-Host ""

# Verify using signtool if available
$SignTool = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin\*\x64\signtool.exe" -ErrorAction SilentlyContinue |
    Sort-Object FullName -Descending | Select-Object -First 1
if (-not $SignTool) {
    $SignTool = Get-Command signtool.exe -ErrorAction SilentlyContinue
}
if ($SignTool -and (Test-Path (Join-Path $DistDir "Hathor-Forge-signed-setup.exe"))) {
    $SignToolPath = if ($SignTool.FullName) { $SignTool.FullName } else { $SignTool.Source }
    Write-Host "Verifying NSIS installer signature..."
    & $SignToolPath verify /pa (Join-Path $DistDir "Hathor-Forge-signed-setup.exe") 2>&1 | Select-Object -Last 3
}
