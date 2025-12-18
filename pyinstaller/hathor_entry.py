#!/usr/bin/env python3
"""Entry point for PyInstaller-built hathor-core binary."""

import sys
import os

# Ensure the bundled modules are found
if getattr(sys, 'frozen', False):
    # Running as compiled
    bundle_dir = sys._MEIPASS
else:
    # Running as script
    bundle_dir = os.path.dirname(os.path.abspath(__file__))

# Import and run the CLI
from hathor_cli.main import main

if __name__ == '__main__':
    main()
