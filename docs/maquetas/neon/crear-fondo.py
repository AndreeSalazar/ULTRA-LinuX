#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Atajo para las maquetas: el generador real vive en src/rootfs/bin/ultra-crear-fondo."""
from pathlib import Path
import runpy
import sys

real = Path(__file__).resolve().parents[3] / 'src/rootfs/bin/ultra-crear-fondo'
sys.argv[0] = str(real)
runpy.run_path(str(real), run_name='__main__')
