# -*- mode: python ; coding: utf-8 -*-
# PyInstaller spec file for hathor-core

import sys
from pathlib import Path

# Path to hathor-core source
hathor_core_path = Path('../../../hathor-core').resolve()

a = Analysis(
    [str(hathor_core_path / 'hathor_cli' / 'main.py')],
    pathex=[str(hathor_core_path)],
    binaries=[],
    datas=[
        # Include any data files hathor-core needs
    ],
    hiddenimports=[
        'hathor',
        'hathor_cli',
        'hathor.manager',
        'hathor.p2p',
        'hathor.transaction',
        'hathor.wallet',
        'hathor.mining',
        'hathor.consensus',
        'twisted.internet.reactor',
        'twisted.internet.ssl',
        'twisted.web.server',
        'twisted.web.resource',
        'autobahn.twisted.websocket',
        'cryptography',
        'rocksdb',
        'pycoin',
        'mnemonic',
        'structlog',
        'pydantic',
        'aiohttp',
        'requests',
    ],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[],
    noarchive=False,
)

pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.datas,
    [],
    name='hathor-core',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=True,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
)
