# Sign the Tauri-generated Windows installers (NSIS .exe and MSI).
# Must be run AFTER `npm run tauri build`.
#
# Required env (one of):
#   SIGN_CERT_THUMBPRINT - SHA1 thumbprint of cert in Windows cert store
#   SIGN_PFX_PATH        - Path to PFX file (+ SIGN_PFX_PASSWORD)
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectDir = Split-Path -Parent (Split-Path -Parent $ScriptDir)
$BundleDir = Join-Path $ProjectDir "src-tauri\target\release\bundle"
$DistDir = Join-Path $ProjectDir "dist"
$TimestampUrl = if ($env:SIGN_TIMESTAMP_URL) { $env:SIGN_TIMESTAMP_URL } else { "http://timestamp.digicert.com" }

if (-not $env:SIGN_CERT_THUMBPRINT -and -not $env:SIGN_PFX_PATH) {
    Write-Error "No signing certificate configured. See sign-windows-binaries.ps1 for details."
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

function Sign-File {
    param([string]$File)
    Write-Host "  Signing: $(Split-Path -Leaf $File)"
    $signArgs = @("sign", "/fd", "SHA256", "/tr", $TimestampUrl, "/td", "SHA256")
    if ($env:SIGN_CERT_THUMBPRINT) {
        $signArgs += "/sha1", $env:SIGN_CERT_THUMBPRINT
    } else {
        $signArgs += "/f", $env:SIGN_PFX_PATH
        if ($env:SIGN_PFX_PASSWORD) {
            $signArgs += "/p", $env:SIGN_PFX_PASSWORD
        }
    }
    $signArgs += $File
    & $SignToolPath @signArgs | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Failed to sign: $File" }
}

Write-Host "=== Signing Windows installers ==="
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

# Verify
if (Test-Path (Join-Path $DistDir "Hathor-Forge-signed-setup.exe")) {
    Write-Host "Verifying NSIS installer signature..."
    & $SignToolPath verify /pa (Join-Path $DistDir "Hathor-Forge-signed-setup.exe") 2>&1 | Select-Object -Last 3
}
