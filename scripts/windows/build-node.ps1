# Copy the Node.js binary into src-tauri/binaries/ for bundling (Windows)
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectDir = Split-Path -Parent (Split-Path -Parent $ScriptDir)
$OutputDir = Join-Path $ProjectDir "src-tauri\binaries"
$Target = "x86_64-pc-windows-msvc"

$NodeBin = (Get-Command node -ErrorAction Stop).Source

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
Copy-Item $NodeBin (Join-Path $OutputDir "node-$Target.exe")

$NodeVersion = & node --version
Write-Host "Copied $NodeVersion -> $OutputDir\node-$Target.exe"

# Create node-dylibs directory with a placeholder so the Tauri resource glob doesn't fail
$DylibDir = Join-Path $OutputDir "node-dylibs"
New-Item -ItemType Directory -Force -Path $DylibDir | Out-Null
Set-Content -Path (Join-Path $DylibDir ".gitkeep") -Value ""
