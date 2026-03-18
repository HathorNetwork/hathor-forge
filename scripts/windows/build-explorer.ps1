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

# Patch wallet-lib bigIntReviver for WebKit/Safari compatibility
$BigIntFile = "node_modules\@hathor\wallet-lib\lib\utils\bigint.js"
if (Test-Path $BigIntFile) {
    Write-Host "Patching wallet-lib bigIntReviver for WebKit compatibility..."
    $f = ($BigIntFile.Replace('\','/'))
    node -e "const fs=require('fs');let c=fs.readFileSync('$f','utf8');c=c.replace(/if \(e instanceof SyntaxError && \(e\.message ===.*?\)\) \{/s,'if (e instanceof SyntaxError) {');fs.writeFileSync('$f',c)"
}

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

# Inject WebKit polyfill for JSON.parse reviver context.source
$PolyfillSrc = Join-Path $ProjectDir "scripts\explorer-patches\webkit-bigint-polyfill.js"
if (Test-Path $PolyfillSrc) {
    Write-Host "Injecting WebKit BigInt polyfill..."
    Copy-Item $PolyfillSrc (Join-Path $OutputDir "static\js\webkit-bigint-polyfill.js")
    $indexHtml = Join-Path $OutputDir "index.html"
    (Get-Content $indexHtml -Raw) -replace '<script defer="defer"', '<script src="/static/js/webkit-bigint-polyfill.js"></script><script defer="defer"' | Set-Content $indexHtml -NoNewline
}

Write-Host ""
Write-Host "=== Build complete ==="
Write-Host "Output: $OutputDir"
