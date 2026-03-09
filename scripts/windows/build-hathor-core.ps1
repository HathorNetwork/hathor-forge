# Build hathor-core as a standalone binary using PyInstaller (Windows)
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectDir = Split-Path -Parent (Split-Path -Parent $ScriptDir)
$HathorCoreDir = if ($env:HATHOR_CORE_SRC) { $env:HATHOR_CORE_SRC } else { Join-Path (Split-Path -Parent $ProjectDir) "hathor-core" }
$BuildDir = Join-Path $ProjectDir "build\hathor-core"
$OutputDir = Join-Path $ProjectDir "src-tauri\binaries"
$Target = "x86_64-pc-windows-msvc"

Write-Host "=== Building hathor-core standalone binary (Windows) ==="
Write-Host "Source: $HathorCoreDir"
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
Write-Host "Installing hathor-core and dependencies..."
python -m pip install --upgrade pip wheel

Set-Location $HathorCoreDir
# Use non-editable install so packages are physically in site-packages
# (editable installs use .pth links that PyInstaller can't follow on Windows)
pip install .

pip install pyinstaller

Set-Location $BuildDir

# Create entry script
@'
#!/usr/bin/env python3
"""Entry point for PyInstaller-built hathor-core binary."""
import multiprocessing
import os
import sys

if getattr(sys, 'frozen', False):
    bundle_dir = os.path.dirname(sys.executable)
    internal_dir = os.path.join(bundle_dir, '_internal')
    # Windows handles DLL loading via PATH or same directory

if __name__ == '__main__':
    multiprocessing.freeze_support()

from hathor_cli.main import main

if __name__ == '__main__':
    main()
'@ | Set-Content -Path "hathor_entry.py" -Encoding UTF8

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
    --name "hathor-core" `
    --clean `
    --noconfirm `
    --runtime-hook=pyi_rth_builtins.py `
    --hidden-import=_contextvars `
    --hidden-import=rocksdb._rocksdb `
    --hidden-import=rocksdb.interfaces `
    --hidden-import=rocksdb.errors `
    --hidden-import=cryptography.hazmat.bindings._rust `
    --collect-all=rocksdb `
    --collect-all=cryptography `
    --collect-submodules=structlog `
    --collect-submodules=twisted `
    --collect-all hathor `
    --collect-all hathor_cli `
    --collect-all hathorlib `
    --exclude-module pytest `
    --exclude-module hathor_tests `
    --exclude-module IPython `
    --exclude-module ipykernel `
    --exclude-module jupyter `
    hathor_entry.py

deactivate

# Copy output
Write-Host ""
Write-Host "Copying binary bundle to output directory..."
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$TargetDir = Join-Path $OutputDir "hathor-core-$Target"
if (Test-Path $TargetDir) { Remove-Item -Recurse -Force $TargetDir }
Copy-Item -Recurse "dist\hathor-core" $TargetDir

Write-Host ""
Write-Host "=== Build complete ==="
Write-Host "Binary bundle: $TargetDir\"
Write-Host "Executable: $TargetDir\hathor-core.exe"
