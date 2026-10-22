#!/usr/bin/env python3
"""Fail-closed structural gate for ULTRA's own static PIE ELF64 executables."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import struct
import subprocess

LOAD, DYNAMIC, INTERP = 1, 2, 3
STACK, RELRO = 0x6474E551, 0x6474E552


def require(condition, rule):
    if not condition:
        raise ValueError(rule)


def segments(raw):
    require(len(raw) >= 64 and raw[:7] == b'\x7fELF\x02\x01\x01', 'ELF64-LE')
    require(struct.unpack_from('<HHI', raw, 16) == (3, 62, 1), 'STATIC-PIE-X86-64')
    entry, phoff = struct.unpack_from('<QQ', raw, 24)
    ehsize, phsize, count = struct.unpack_from('<HHH', raw, 52)
    require(ehsize == 64 and phsize == 56 and 1 <= count <= 128, 'PHDR-SHAPE')
    require(phoff >= 64 and phoff + count * phsize <= len(raw), 'PHDR-BOUNDS')
    result = []
    for i in range(count):
        at = phoff + i * phsize
        kind, flags, offset, va, _pa, size, memory, alignment = struct.unpack_from('<IIQQQQQQ', raw, at)
        require(offset <= len(raw) and size <= len(raw) - offset, 'SEGMENT-FILE-BOUNDS')
        require(va + memory < 1 << 64, 'SEGMENT-ADDRESS-OVERFLOW')
        if kind == LOAD:
            require(size <= memory, 'LOAD-SIZE')
            require(alignment in (0, 1) or (alignment & (alignment - 1) == 0 and
                    va % alignment == offset % alignment), 'LOAD-ALIGNMENT')
        result.append(dict(Type=kind, Flags=flags, Offset=offset, Address=va,
                           FileBytes=size, MemoryBytes=memory, HeaderOffset=at))
    return entry, result


def structural(raw):
    entry, items = segments(raw)
    loads = [s for s in items if s['Type'] == LOAD]
    require(loads and not any(s['Flags'] & 3 == 3 for s in loads), 'NO-WX-LOAD')
    require(any(s['Flags'] & 1 and s['Address'] <= entry < s['Address'] + s['FileBytes']
                for s in loads), 'ENTRY-IN-EXECUTABLE-FILE-REGION')
    require(not any(s['Type'] == INTERP for s in items), 'NO-INTERPRETER')
    stacks = [s for s in items if s['Type'] == STACK]
    require(len(stacks) == 1 and stacks[0]['Flags'] == 6, 'STACK-RW-NX')
    relro = [s for s in items if s['Type'] == RELRO]
    require(len(relro) == 1 and relro[0]['MemoryBytes'] > 0, 'RELRO-REQUIRED')
    r = relro[0]
    require(any(s['Flags'] & 2 and s['Address'] <= r['Address'] and
                r['Address'] + r['MemoryBytes'] <= s['Address'] + s['MemoryBytes']
                for s in loads), 'RELRO-IN-WRITABLE-LOAD')
    dynamic = [s for s in items if s['Type'] == DYNAMIC]
    require(len(dynamic) == 1 and dynamic[0]['FileBytes'] % 16 == 0, 'DYNAMIC-REQUIRED')
    d = dynamic[0]
    now = terminated = False
    for at in range(d['Offset'], d['Offset'] + d['FileBytes'], 16):
        tag, value = struct.unpack_from('<QQ', raw, at)
        if tag == 0:
            terminated = True
            break
        require(tag != 1, 'NO-DT-NEEDED')
        if tag == 24 or (tag == 30 and value & 8) or (tag == 0x6FFFFFFB and value & 1):
            now = True
        require(tag != 22 and not (tag == 30 and value & 4), 'NO-TEXTREL')
    require(terminated and now, 'DYNAMIC-NOW')
    return items


def run_tool(*args):
    return subprocess.check_output(args, text=True, env={**os.environ, 'LC_ALL': 'C'})


def audit(path, out):
    raw = path.read_bytes()
    items = structural(raw)
    notes = run_tool('readelf', '-W', '-n', str(path.resolve()))
    require(re.search(r'x86 feature:.*\bIBT\b.*\bSHSTK\b', notes), 'CET-PROPERTY-IBT-SHSTK')
    listing = run_tool('objdump', '-d', '-Mintel', '--insn-width=16', '--disassemble=main', str(path.resolve()))
    match = re.search(r'<main>:\s*\n\s*[0-9a-f]+:\s*((?:[0-9a-f]{2}\s+){4})', listing)
    require(match is not None and bytes.fromhex(match[1]) == bytes.fromhex('F3 0F 1E FA'), 'MAIN-ENDBR64')
    headers = run_tool('readelf', '-W', '-h', '-l', '-d', str(path.resolve()))
    report = dict(Schema=1, Binary=path.name, SHA256=hashlib.sha256(raw).hexdigest(),
                  InspectorSHA256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                  Status='PASS', Segments=items,
                  Rules=['ELF64-LE', 'STATIC-PIE-X86-64', 'PHDR-BOUNDS', 'LOAD-ALIGNMENT',
                         'NO-WX-LOAD', 'ENTRY-IN-EXECUTABLE-FILE-REGION', 'NO-INTERPRETER',
                         'STACK-RW-NX', 'RELRO-REQUIRED', 'RELRO-IN-WRITABLE-LOAD',
                         'NO-DT-NEEDED', 'NO-TEXTREL', 'DYNAMIC-NOW',
                         'CET-PROPERTY-IBT-SHSTK', 'MAIN-ENDBR64'],
                  Tools=[run_tool('readelf', '--version').splitlines()[0],
                         run_tool('objdump', '--version').splitlines()[0]],
                  Scope='Own unstripped static PIE binaries only. Structural metadata and main entry bytes; not semantic verification or proof CET is active at runtime.')
    # Publication follows all checks. An existing report is never reused.
    out.mkdir(parents=True, exist_ok=False)
    (out / 'informe.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    (out / 'readelf.txt').write_text(headers + notes, encoding='utf-8')
    (out / 'main.txt').write_text(listing, encoding='utf-8')
    print(f'ELF-PRODUCCION-OK: {path.name} {report["SHA256"]}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    try:
        audit(args.binary, args.out)
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        raise SystemExit(f'ROJO [ELF-PRODUCCION]: {error}') from error


if __name__ == '__main__':
    main()
