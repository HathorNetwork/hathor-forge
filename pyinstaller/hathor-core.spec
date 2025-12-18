# -*- mode: python ; coding: utf-8 -*-
# PyInstaller spec file for hathor-core

import sys
from pathlib import Path

# Path to hathor-core source
HATHOR_CORE_PATH = Path('../hathor-core').resolve()

block_cipher = None

# Collect all hathor and hathor_cli packages
a = Analysis(
    [str(HATHOR_CORE_PATH / 'hathor_cli' / 'main.py')],
    pathex=[str(HATHOR_CORE_PATH)],
    binaries=[],
    datas=[
        # Include OpenAPI JSON files
        (str(HATHOR_CORE_PATH / 'hathor' / '_openapi' / '*.json'), 'hathor/_openapi'),
    ],
    hiddenimports=[
        # Twisted imports (reactor and protocols)
        'twisted.internet.reactor',
        'twisted.internet.selectreactor',
        'twisted.internet.pollreactor',
        'twisted.internet.epollreactor',
        'twisted.internet.kqreactor',
        'twisted.internet.asyncioreactor',
        'twisted.internet.ssl',
        'twisted.internet.tcp',
        'twisted.internet.udp',
        'twisted.internet.unix',
        'twisted.internet.protocol',
        'twisted.internet.defer',
        'twisted.internet.task',
        'twisted.internet.threads',
        'twisted.internet.endpoints',
        'twisted.web.server',
        'twisted.web.resource',
        'twisted.web.static',
        'twisted.web.http',
        'twisted.web.client',
        'twisted.python.log',
        'twisted.python.failure',
        'twisted.protocols.basic',
        'twisted.protocols.policies',

        # Autobahn
        'autobahn.twisted.websocket',
        'autobahn.twisted.resource',
        'autobahn.websocket.protocol',

        # Structlog
        'structlog',
        'structlog.stdlib',
        'structlog.processors',
        'structlog.dev',

        # Cryptography
        'cryptography',
        'cryptography.hazmat.primitives',
        'cryptography.hazmat.primitives.asymmetric',
        'cryptography.hazmat.primitives.ciphers',
        'cryptography.hazmat.primitives.hashes',
        'cryptography.hazmat.primitives.kdf',
        'cryptography.hazmat.primitives.serialization',
        'cryptography.hazmat.backends',
        'cryptography.hazmat.backends.openssl',

        # RocksDB
        'rocksdb',

        # Pycoin
        'pycoin',
        'pycoin.ecdsa',
        'pycoin.encoding',
        'pycoin.key',

        # Aiohttp
        'aiohttp',
        'aiohttp.web',

        # Other dependencies
        'mnemonic',
        'base58',
        'colorama',
        'configargparse',
        'prometheus_client',
        'service_identity',
        'sortedcontainers',
        'setproctitle',
        'pydantic',
        'yaml',
        'hathorlib',
        'healthchecklib',

        # Hathor internal modules (ensure all CLI commands are included)
        'hathor_cli.run_node',
        'hathor_cli.mining',
        'hathor_cli.stratum_mining',
        'hathor_cli.merged_mining',
        'hathor_cli.wallet',
        'hathor_cli.peer_id',
        'hathor_cli.shell',
        'hathor_cli.quick_test',

        # Hathor core modules
        'hathor.manager',
        'hathor.p2p',
        'hathor.transaction',
        'hathor.wallet',
        'hathor.stratum',
        'hathor.mining',
        'hathor.consensus',
        'hathor.storage',
        'hathor.crypto',
        'hathor.conf',
    ],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[
        # Exclude test modules
        'hathor_tests',
        'pytest',
        'pytest_cov',
        # Exclude dev dependencies
        'mypy',
        'flake8',
        'isort',
        # Exclude IPython (large, not needed for CLI)
        'IPython',
        'ipykernel',
        'jupyter',
    ],
    win_no_prefer_redirects=False,
    win_private_assemblies=False,
    cipher=block_cipher,
    noarchive=False,
)

pyz = PYZ(a.pure, a.zipped_data, cipher=block_cipher)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.zipfiles,
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
