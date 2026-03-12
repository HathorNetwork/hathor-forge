# Build hathor-explorer for basic mode + localnet (Windows)
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectDir = Split-Path -Parent (Split-Path -Parent $ScriptDir)
$ExplorerDir = if ($env:HATHOR_EXPLORER_SRC) { $env:HATHOR_EXPLORER_SRC } else { Join-Path (Split-Path -Parent $ProjectDir) "hathor-explorer" }
$BuildDir = Join-Path $ProjectDir "build\explorer"
$OutputDir = Join-Path $ProjectDir "src-tauri\explorer-dist"

Write-Host "=== Building hathor-explorer for embedding (Windows) ==="
Write-Host "Source: $ExplorerDir"
Write-Host "Build:  $BuildDir"
Write-Host "Output: $OutputDir"
Write-Host ""

# Create build directory
if (Test-Path $BuildDir) { Remove-Item -Recurse -Force $BuildDir }
New-Item -ItemType Directory -Force -Path $BuildDir | Out-Null

# Copy source to build dir
Copy-Item -Recurse "$ExplorerDir\*" $BuildDir
Set-Location $BuildDir

# Install dependencies
Write-Host "Installing dependencies..."
npm install

# Build with basic mode + localnet config
Write-Host ""
Write-Host "Building with basic mode configuration..."
$env:REACT_APP_EXPLORER_MODE = "basic"
$env:REACT_APP_BASE_URL = "http://localhost:49081/v1a/"
$env:REACT_APP_WS_URL = "ws://localhost:49081/v1a/ws/"
$env:REACT_APP_NETWORK = "local-privatenet"
npm run build

# Copy output
Write-Host ""
Write-Host "Copying build to output directory..."
if (Test-Path $OutputDir) { Remove-Item -Recurse -Force $OutputDir }
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
Copy-Item -Recurse "build\*" $OutputDir

Write-Host ""
Write-Host "=== Build complete ==="
Write-Host "Output: $OutputDir"
