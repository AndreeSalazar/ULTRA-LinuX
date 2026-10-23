#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Passive C symbol inspection; shares the existing TALK-inspired HEX extractor."""
import hashlib
import json
from pathlib import Path
import runpy
import subprocess
import sys

binary, out = Path(sys.argv[1]), Path(sys.argv[2])
extract = runpy.run_path(str(Path(__file__).with_name('inspeccionar-asm-hex.py')))['extract']
raw = binary.read_bytes()
if len(raw) < 64 or raw[:6] != b'\x7fELF\x02\x01' or int.from_bytes(raw[18:20], 'little') != 62:
    raise ValueError('requires ELF64 little-endian x86-64')
records, listings = [], []
for symbol in ('uc_begin', 'uc_complete', 'uc_revoke'):
    listing = subprocess.check_output(['objdump', '-d', '-Mintel', '--insn-width=16',
        '--disassemble='+symbol, str(binary.resolve())], text=True, env={'PATH': '/usr/bin', 'LC_ALL': 'C'})
    data = extract(listing, symbol)
    records.append(dict(Symbol=symbol, Bytes=len(data), Hex=data.hex(' ').upper(),
                        SHA256=hashlib.sha256(data).hexdigest()))
    listings.append(listing)
out.mkdir(exist_ok=False)
for record in records:
    (out/(record['Symbol']+'.hex')).write_text(record['Hex']+'\n')
(out/'desensamblado.txt').write_text('\n'.join(listings))
(out/'informe.json').write_text(json.dumps(dict(Schema=1,
    BinarySHA256=hashlib.sha256(raw).hexdigest(),
    InspectorSHA256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    ExtractorSHA256=hashlib.sha256(Path(__file__).with_name('inspeccionar-asm-hex.py').read_bytes()).hexdigest(),
    Tool=subprocess.check_output(['objdump', '--version'], text=True).splitlines()[0],
    Symbols=records, Scope='Passive inventory, not an exact byte/semantic contract, no execution or speed claim.'), indent=2)+'\n')
print('CUSTODIA-HEX-OK: tres simbolos C; bytes, desensamblado y hashes; sin contrato semantico automatico')
