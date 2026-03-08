# Build hathor-wallet-headless (Windows)
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectDir = Split-Path -Parent (Split-Path -Parent $ScriptDir)
$HeadlessSrc = if ($env:WALLET_HEADLESS_SRC) { $env:WALLET_HEADLESS_SRC } else { Join-Path (Split-Path -Parent $ProjectDir) "hathor-wallet-headless" }
$BuildDir = Join-Path $ProjectDir "build\wallet-headless"
$OutputDir = Join-Path $ProjectDir "src-tauri\wallet-headless-dist"

Write-Host "=== Building hathor-wallet-headless (Windows) ==="
Write-Host "Source: $HeadlessSrc"
Write-Host "Build:  $BuildDir"
Write-Host "Output: $OutputDir"
Write-Host ""

# Create build directory
if (Test-Path $BuildDir) { Remove-Item -Recurse -Force $BuildDir }
New-Item -ItemType Directory -Force -Path $BuildDir | Out-Null

# Copy source to build dir
Copy-Item -Recurse "$HeadlessSrc\*" $BuildDir
Set-Location $BuildDir

# Remove patch-package from postinstall
$pkgJson = Get-Content "package.json" -Raw
$pkgJson = $pkgJson -replace '"postinstall":\s*"(npx )?patch-package"', '"postinstall": "echo skipping patches"'
Set-Content -Path "package.json" -Value $pkgJson -Encoding UTF8

# Update babel config for CommonJS output
@'
{
  "presets": [
    [
      "@babel/preset-env",
      {
        "targets": {
          "node": "18"
        },
        "modules": "commonjs"
      }
    ]
  ],
  "plugins": ["@babel/plugin-proposal-class-properties"]
}
'@ | Set-Content -Path "babel.config.json" -Encoding UTF8

# Install dependencies
Write-Host "Installing dependencies..."
npm install --ignore-scripts
npm install

# Build
Write-Host ""
Write-Host "Building..."
npm run build

# Copy output
Write-Host ""
Write-Host "Copying build to output directory..."
if (Test-Path $OutputDir) { Remove-Item -Recurse -Force $OutputDir }
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

Copy-Item -Recurse "dist" "$OutputDir\dist"
Copy-Item -Recurse "node_modules" "$OutputDir\node_modules"
Copy-Item "package.json" "$OutputDir\package.json"

Write-Host ""
Write-Host "=== Build complete ==="
Write-Host "Output: $OutputDir"
