#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Passive C symbol inspection; shares the existing TALK-inspired HEX extractor."""
import hashlib
import json
from pathlib import Path
import runpy
import subprocess
import sys
import struct

binary, out = Path(sys.argv[1]), Path(sys.argv[2])
extract = runpy.run_path(str(Path(__file__).with_name('inspeccionar-asm-hex.py')))['extract']
raw = binary.read_bytes()
if len(raw) < 64 or raw[:6] != b'\x7fELF\x02\x01' or int.from_bytes(raw[18:20], 'little') != 62:
    raise ValueError('requires ELF64 little-endian x86-64')
records, listings = [], []
# Resolve symbols through ELF section/symbol tables independently of objdump.
section_offset = struct.unpack_from('<Q', raw, 40)[0]
section_size, section_count = struct.unpack_from('<HH', raw, 58)
if section_size != 64 or section_offset + section_size*section_count > len(raw):
    raise ValueError('invalid ELF section table')
sections = [struct.unpack_from('<IIQQQQIIQQ', raw, section_offset+i*64)
            for i in range(section_count)]
symbols = {}
for section in sections:
    if section[1] != 2:  # SHT_SYMTAB, including static local functions
        continue
    if section[6] >= len(sections) or section[9] != 24:
        raise ValueError('invalid ELF symbol table')
    strings = sections[section[6]]
    names = raw[strings[4]:strings[4]+strings[5]]
    for offset in range(section[4], section[4]+section[5], 24):
        name, info, other, index, address, size = struct.unpack_from('<IBBHQQ', raw, offset)
        if info & 15 != 2 or not size or index >= len(sections):
            continue
        end = names.find(b'\0', name)
        if end < 0:
            raise ValueError('invalid ELF symbol name')
        segment = sections[index]
        file_offset = segment[4]+address-segment[3]
        if file_offset < segment[4] or file_offset+size > segment[4]+segment[5] or file_offset+size > len(raw):
            raise ValueError('invalid ELF symbol range')
        symbols[names[name:end].decode()] = raw[file_offset:file_offset+size]
requested = sys.argv[3:] or ['uc_begin', 'uc_complete', 'uc_revoke',
                            'ultra_owner_alloc', 'ultra_owner_free', 'ultra_owner_destroy']
for symbol in requested:
    listing = subprocess.check_output(['objdump', '-d', '-Mintel', '--insn-width=16',
        '--disassemble='+symbol, str(binary.resolve())], text=True, env={'PATH': '/usr/bin', 'LC_ALL': 'C'})
    data = extract(listing, symbol)
    if symbols.get(symbol) != data:
        raise ValueError('ELF-BYTES-MISMATCH: '+symbol)
    records.append(dict(Symbol=symbol, Bytes=len(data), Hex=data.hex(' ').upper(),
                        SHA256=hashlib.sha256(data).hexdigest(), ELFBytesMatch=True))
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
print(f'CUSTODIA-HEX-OK: {len(records)} simbolos C; bytes ELF contrastados, desensamblado y hashes; sin prueba semantica automatica')
