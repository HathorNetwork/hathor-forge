# Build tx-mining-service as a standalone binary using PyInstaller (Windows)
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectDir = Split-Path -Parent (Split-Path -Parent $ScriptDir)
$TxMiningDir = if ($env:TX_MINING_SERVICE_SRC) { $env:TX_MINING_SERVICE_SRC } else { Join-Path (Split-Path -Parent $ProjectDir) "tx-mining-service" }
$BuildDir = Join-Path $ProjectDir "build\tx-mining-service"
$OutputDir = Join-Path $ProjectDir "src-tauri\binaries"
$Target = "x86_64-pc-windows-msvc"

Write-Host "=== Building tx-mining-service standalone binary (Windows) ==="
Write-Host "Source: $TxMiningDir"
Write-Host "Build:  $BuildDir"
Write-Host "Output: $OutputDir"
Write-Host "Target: $Target"
Write-Host ""

# Create build directory
if (Test-Path $BuildDir) { Remove-Item -Recurse -Force $BuildDir }
New-Item -ItemType Directory -Force -Path $BuildDir | Out-Null
Set-Location $BuildDir

# Create virtual environment
Write-Host "Creating virtual environment..."
python -m venv venv
& ".\venv\Scripts\Activate.ps1"

# Install dependencies
Write-Host "Installing tx-mining-service and dependencies..."
pip install --upgrade pip wheel

Set-Location $TxMiningDir

pip install configargparse colorama aiohttp base58 structlog prometheus-client idna_ssl asynctest python-healthchecklib
pip install "hathorlib[client]"

# Install tx-mining-service package so PyInstaller can find txstratum.
# The project uses package-mode=false, so editable installs fail.
# Use non-editable install; if that also fails, copy txstratum into site-packages.
try {
    pip install . 2>&1 | Out-Null
} catch {
    Write-Host "pip install . failed, copying txstratum into site-packages..."
    $SitePackages = python -c "import site; print(site.getsitepackages()[0])"
    Copy-Item -Recurse "$TxMiningDir\txstratum" "$SitePackages\txstratum"
}

pip install pyinstaller

Set-Location $BuildDir

# Copy log.conf
if (Test-Path "$TxMiningDir\log.conf") {
    Copy-Item "$TxMiningDir\log.conf" .
}

# Create entry script
@'
#!/usr/bin/env python3
"""Entry point for PyInstaller-built tx-mining-service binary."""
import multiprocessing
import os
import sys

if getattr(sys, 'frozen', False):
    bundle_dir = os.path.dirname(sys.executable)
    internal_dir = os.path.join(bundle_dir, '_internal')
    # Windows handles DLL loading via PATH or same directory

if __name__ == '__main__':
    multiprocessing.freeze_support()

# On Windows, asyncio does not support add_signal_handler (Unix-only).
# Monkey-patch the event loop so tx-mining-service's register_signal_handlers
# silently succeeds instead of raising NotImplementedError.
if sys.platform == 'win32':
    import asyncio
    _orig_loop_class = asyncio.ProactorEventLoop
    _orig_add = _orig_loop_class.add_signal_handler
    def _noop_add_signal_handler(self, sig, callback, *args):
        pass
    def _noop_remove_signal_handler(self, sig):
        return False
    _orig_loop_class.add_signal_handler = _noop_add_signal_handler
    _orig_loop_class.remove_signal_handler = _noop_remove_signal_handler

from txstratum.cli import main

if __name__ == '__main__':
    main()
'@ | Set-Content -Path "tx_mining_entry.py" -Encoding UTF8

# Create runtime hook
@'
import builtins

class _DisabledBuiltin:
    """Placeholder for builtins that don't exist in frozen environment."""
    def __init__(self, name):
        self._name = name
    def __call__(self, *args, **kwargs):
        raise RuntimeError(f"The builtin '{self._name}' is not available in frozen environment")
    def __repr__(self):
        return f"<disabled builtin '{self._name}'>"

_missing_builtins = ['copyright', 'credits', 'exit', 'help', 'license', 'quit']

for name in _missing_builtins:
    if not hasattr(builtins, name) or getattr(builtins, name) is None:
        setattr(builtins, name, _DisabledBuiltin(name))
'@ | Set-Content -Path "pyi_rth_builtins.py" -Encoding UTF8

# Run PyInstaller
Write-Host ""
Write-Host "Running PyInstaller..."
pyinstaller `
    --onedir `
    --name "tx-mining-service" `
    --clean `
    --noconfirm `
    --runtime-hook=pyi_rth_builtins.py `
    --hidden-import=_contextvars `
    --paths "$TxMiningDir" `
    --collect-all txstratum `
    --collect-all hathorlib `
    --collect-all configargparse `
    --collect-all structlog `
    --exclude-module pytest `
    --exclude-module IPython `
    tx_mining_entry.py

deactivate

# Copy output
Write-Host ""
Write-Host "Copying binary bundle to output directory..."
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$TargetDir = Join-Path $OutputDir "tx-mining-service-$Target"
if (Test-Path $TargetDir) { Remove-Item -Recurse -Force $TargetDir }
Copy-Item -Recurse "dist\tx-mining-service" $TargetDir

# Copy log.conf into bundle
if (Test-Path "$TxMiningDir\log.conf") {
    Copy-Item "$TxMiningDir\log.conf" $TargetDir
}

Write-Host ""
Write-Host "=== Build complete ==="
Write-Host "Binary bundle: $TargetDir\"
Write-Host "Executable: $TargetDir\tx-mining-service.exe"
