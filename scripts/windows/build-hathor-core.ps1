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

# -------------------------------------------------------------------
# Step 1: Build RocksDB and compression libraries via vcpkg
# -------------------------------------------------------------------
Write-Host "=== Step 1: Installing RocksDB via vcpkg ==="

# vcpkg is pre-installed on GitHub Actions runners at C:\vcpkg
$VcpkgRoot = if ($env:VCPKG_INSTALLATION_ROOT) { $env:VCPKG_INSTALLATION_ROOT } else { "C:\vcpkg" }
$VcpkgExe = Join-Path $VcpkgRoot "vcpkg.exe"

if (-not (Test-Path $VcpkgExe)) {
    Write-Error "vcpkg not found at $VcpkgRoot. Please install vcpkg or set VCPKG_INSTALLATION_ROOT."
    exit 1
}

Write-Host "Using vcpkg at: $VcpkgRoot"

# Pin vcpkg's port tree to a commit where rocksdb is still at 10.10.1.
# RocksDB 11.0 changed DB::Open / OpenForReadOnly / OpenAsSecondary signatures
# (DB** -> unique_ptr<DB>*), which breaks HathorNetwork's python-rocksdb fork.
# This pin keeps rocksdb compatible until python-rocksdb is updated for v11.
# If you bump this SHA, also bump the cache-key suffix in .github/workflows/release.yml.
$VcpkgPinSha = "9b4f30e02bef7b0cfb90d1810ba4cdc6b4da1728"
$VcpkgHeadSha = (& git -C $VcpkgRoot rev-parse HEAD).Trim()
if ($VcpkgHeadSha -ne $VcpkgPinSha) {
    Write-Host "Pinning vcpkg from $VcpkgHeadSha to $VcpkgPinSha..."
    & git -C $VcpkgRoot fetch --depth 1 origin $VcpkgPinSha
    if ($LASTEXITCODE -ne 0) { throw "vcpkg fetch failed" }
    & git -C $VcpkgRoot checkout $VcpkgPinSha
    if ($LASTEXITCODE -ne 0) { throw "vcpkg checkout failed" }
    & "$VcpkgRoot\bootstrap-vcpkg.bat" -disableMetrics
    if ($LASTEXITCODE -ne 0) { throw "vcpkg bootstrap failed" }
} else {
    Write-Host "vcpkg already at pinned SHA"
}

# Install rocksdb with all compression backends using static-md triplet
# static-md = static libraries with dynamic CRT (/MD) — avoids DLL dependency issues
# while remaining compatible with Python's dynamic CRT requirement
$VcpkgTriplet = "x64-windows-static-md"
& $VcpkgExe install "rocksdb[snappy,lz4,bzip2,zlib]:$VcpkgTriplet"
if ($LASTEXITCODE -ne 0) { throw "vcpkg install failed" }

# Locate installed files
$VcpkgInstalled = Join-Path $VcpkgRoot "installed\$VcpkgTriplet"
$VcpkgInclude = Join-Path $VcpkgInstalled "include"
$VcpkgLib = Join-Path $VcpkgInstalled "lib"
# No bin dir for static triplet (no DLLs produced)

Write-Host "vcpkg include: $VcpkgInclude"
Write-Host "vcpkg lib:     $VcpkgLib"

# Verify key files exist
if (-not (Test-Path "$VcpkgLib\rocksdb.lib")) {
    Write-Host "Available libs:"
    Get-ChildItem $VcpkgLib -Filter "rocksdb*" | ForEach-Object { Write-Host "  $_" }
    Get-ChildItem $VcpkgLib -Filter "snappy*" | ForEach-Object { Write-Host "  $_" }
    Get-ChildItem $VcpkgLib -Filter "lz4*" | ForEach-Object { Write-Host "  $_" }
    Write-Error "rocksdb.lib not found in vcpkg installed dir"
    exit 1
}
Write-Host "rocksdb.lib found OK"
Write-Host ""

# -------------------------------------------------------------------
# Step 2: Create venv and install python-rocksdb with MSVC support
# -------------------------------------------------------------------
Write-Host "=== Step 2: Building python-rocksdb for MSVC ==="

# Create build directory
if (Test-Path $BuildDir) { Remove-Item -Recurse -Force $BuildDir }
New-Item -ItemType Directory -Force -Path $BuildDir | Out-Null
Set-Location $BuildDir

# Create virtual environment
Write-Host "Creating virtual environment..."
python -m venv venv
& ".\venv\Scripts\Activate.ps1"

python -m pip install --upgrade pip wheel setuptools "Cython>=3.0.0"

# Clone Hathor's python-rocksdb fork
Write-Host "Cloning python-rocksdb..."
git clone --depth 1 https://github.com/HathorNetwork/python-rocksdb.git python-rocksdb
Set-Location python-rocksdb

# Patch setup.py for MSVC compatibility
Write-Host "Patching setup.py for MSVC..."
$SetupPy = @'
import os
import platform
import subprocess
import sys
from Cython.Build import cythonize
from setuptools import setup, Extension

def get_brew_prefix(package):
    try:
        return subprocess.check_output(["brew", "--prefix", package], universal_newlines=True).strip()
    except (subprocess.CalledProcessError, FileNotFoundError):
        return None

include_dirs = []
library_dirs = []
extra_compile_args = []
libraries = []

if platform.system() == "Windows":
    # MSVC-compatible flags
    extra_compile_args = [
        "/std:c++20",
        "/O2",
        "/GR-",       # disable RTTI (equivalent of -fno-rtti)
        "/EHsc",      # standard C++ exception handling
    ]
    # vcpkg paths (set via env vars or defaults)
    vcpkg_include = os.environ.get("VCPKG_INCLUDE", "")
    vcpkg_lib = os.environ.get("VCPKG_LIB", "")
    if vcpkg_include:
        include_dirs.append(vcpkg_include)
    if vcpkg_lib:
        library_dirs.append(vcpkg_lib)
    # Static libraries from vcpkg x64-windows-static-md triplet
    libraries = ["rocksdb", "snappy", "lz4", "zlib", "bz2"]
    # RocksDB on Windows also needs these system libs
    libraries.extend(["shlwapi", "rpcrt4"])
elif platform.system() == "Darwin":
    extra_compile_args = [
        "-std=c++20",
        "-O2",
        "-fno-strict-aliasing",
        "-fno-rtti",
        "-Wno-unreachable-code-fallthrough",
        "-mmacosx-version-min=11.0",
        "-stdlib=libc++",
        "-Wno-unreachable-code",
    ]
    libraries = ["rocksdb", "snappy", "bz2", "z", "lz4"]
    for dep in ["rocksdb", "snappy", "lz4"]:
        dep_prefix = get_brew_prefix(dep)
        if dep_prefix:
            include_dirs.append(os.path.join(dep_prefix, "include"))
            library_dirs.append(os.path.join(dep_prefix, "lib"))
else:
    # Linux
    extra_compile_args = [
        "-std=c++20",
        "-O2",
        "-fno-strict-aliasing",
        "-fno-rtti",
        "-Wno-unreachable-code-fallthrough",
        "-Wno-dangling-pointer",
        "-Wno-maybe-uninitialized",
    ]
    libraries = ["rocksdb", "snappy", "bz2", "z", "lz4"]

setup(ext_modules=cythonize([Extension(
    "rocksdb._rocksdb",
    ["rocksdb/_rocksdb.pyx"],
    include_dirs=include_dirs,
    library_dirs=library_dirs,
    extra_compile_args=extra_compile_args,
    language="c++",
    libraries=libraries,
)]))
'@
$SetupPy | Set-Content -Path "setup.py" -Encoding UTF8

# Set vcpkg paths for setup.py
$env:VCPKG_INCLUDE = $VcpkgInclude
$env:VCPKG_LIB = $VcpkgLib

# Build and install python-rocksdb
# Use --no-build-isolation so Cython and env vars (VCPKG_INCLUDE/LIB) are available
# Use -v for verbose output to see compilation errors
Write-Host "Building python-rocksdb..."
Write-Host "  VCPKG_INCLUDE=$env:VCPKG_INCLUDE"
Write-Host "  VCPKG_LIB=$env:VCPKG_LIB"
pip install --no-build-isolation -v .
if ($LASTEXITCODE -ne 0) { throw "python-rocksdb build failed" }

# cd out of source dir so local rocksdb/ doesn't shadow the installed package
Set-Location $BuildDir

# With static-md triplet, all libraries are statically linked into _rocksdb.pyd.
# No DLL copying needed — only the MSVC CRT DLLs are required (provided by the OS/runner).

# Verify it imports
Write-Host "Verifying python-rocksdb import..."
python -c "import rocksdb; print('rocksdb imported OK')"
if ($LASTEXITCODE -ne 0) { throw "python-rocksdb import verification failed" }

Write-Host ""

# -------------------------------------------------------------------
# Step 3: Install hathor-core
# -------------------------------------------------------------------
Write-Host "=== Step 3: Installing hathor-core ==="
Set-Location $HathorCoreDir
pip install .
if ($LASTEXITCODE -ne 0) { throw "hathor-core install failed" }

pip install pyinstaller

Set-Location $BuildDir

# -------------------------------------------------------------------
# Step 4: Run PyInstaller
# -------------------------------------------------------------------
Write-Host "=== Step 4: Running PyInstaller ==="

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

# Create runtime hook to stub Unix-only modules on Windows.
# Uses a catch-all __getattr__ so any unknown attribute returns a no-op
# instead of crashing — no more whack-a-mole with missing attributes.
@'
import sys
import types
import platform

if platform.system() == "Windows":

    class _StubModule(types.ModuleType):
        """A module stub where any unknown attribute returns a no-op callable
        or 0, so code that touches Unix-only APIs never crashes."""
        class _Noop:
            """Returns 0 in numeric contexts, no-op when called."""
            def __call__(self, *a, **kw): return 0
            def __int__(self): return 0
            def __or__(self, other): return other
            def __ror__(self, other): return other
            def __and__(self, other): return 0
            def __rand__(self, other): return 0
            def __bool__(self): return False
            def __repr__(self): return "0"
        _noop_instance = _Noop()
        def __getattr__(self, name):
            if name.startswith("__") and name.endswith("__"):
                raise AttributeError(name)
            return self._noop_instance

    # _curses: curses/__init__.py does "from _curses import *" and checks
    # for has_key, error, etc.
    if "_curses" not in sys.modules:
        mod = _StubModule("_curses")
        mod.error = type("error", (Exception,), {})
        mod.ERR = -1
        mod.OK = 0
        mod.version = b"0.0"
        mod.has_key = lambda key: False
        mod.LINES = 24
        mod.COLS = 80
        sys.modules["_curses"] = mod

    # resource: used by run_node.py (fd limits) and prometheus_client (getpagesize)
    if "resource" not in sys.modules:
        mod = _StubModule("resource")
        mod.RLIMIT_NOFILE = 7
        mod.getrlimit = lambda _: (8192, 8192)
        mod.setrlimit = lambda *_: None
        mod.getpagesize = lambda: 4096
        sys.modules["resource"] = mod

    # termios: Python's tty module does "from termios import *" and uses
    # constants as default parameter values (e.g. TCSAFLUSH), so __getattr__
    # alone isn't enough — we must set them in __dict__.
    if "termios" not in sys.modules:
        mod = _StubModule("termios")
        mod.error = type("error", (Exception,), {})
        mod.tcgetattr = lambda fd: [0, 0, 0, 0, 0, 0, [b"\x00"] * 32]
        mod.tcsetattr = lambda *a: None
        # Constants used by tty.py (from termios import *)
        _termios_consts = {
            "TCSAFLUSH": 2, "TCSANOW": 0, "TCSADRAIN": 1,
            "IGNBRK": 1, "BRKINT": 2, "IGNPAR": 4, "PARMRK": 8,
            "INPCK": 16, "ISTRIP": 32, "INLCR": 64, "IGNCR": 128,
            "ICRNL": 256, "IXON": 512, "IXOFF": 1024, "IXANY": 2048,
            "OPOST": 1, "PARENB": 4096, "CSIZE": 768, "CS8": 768,
            "ECHO": 8, "ECHOE": 2, "ECHOK": 4, "ECHONL": 16,
            "ICANON": 256, "IEXTEN": 1024, "ISIG": 128,
            "NOFLSH": 2147483648, "TOSTOP": 4194304,
            "VMIN": 16, "VTIME": 17,
        }
        for k, v in _termios_consts.items():
            setattr(mod, k, v)
        sys.modules["termios"] = mod

    # pty/tty: Unix-only; imported by twisted.internet.process
    for mod_name in ("_curses_panel", "fcntl", "pty", "tty"):
        if mod_name not in sys.modules:
            sys.modules[mod_name] = _StubModule(mod_name)
'@ | Set-Content -Path "pyi_rth_unix_stubs.py" -Encoding UTF8

Write-Host ""
Write-Host "Running PyInstaller..."
pyinstaller `
    --onedir `
    --name "hathor-core" `
    --clean `
    --noconfirm `
    --runtime-hook=pyi_rth_builtins.py `
    --runtime-hook=pyi_rth_unix_stubs.py `
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

if ($LASTEXITCODE -ne 0) { throw "PyInstaller failed" }

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
