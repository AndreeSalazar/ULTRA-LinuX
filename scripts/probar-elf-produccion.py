#!/usr/bin/env python3
"""Mutation tests of the production gate; never execute modified ELF files."""
import importlib.util
from pathlib import Path
import struct
import sys
import tempfile

spec = importlib.util.spec_from_file_location('gate', Path(__file__).with_name('auditar-elf-produccion.py'))
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)
binary = Path(sys.argv[1])
raw = binary.read_bytes()
items = gate.structural(raw)
cases = []


def changed(label, at, fmt, value, rule):
    data = bytearray(raw)
    struct.pack_into(fmt, data, at, value)
    cases.append((label, data, rule))


stack = next(s for s in items if s['Type'] == gate.STACK)
load = next(s for s in items if s['Type'] == gate.LOAD and s['Flags'] & 1)
relro = next(s for s in items if s['Type'] == gate.RELRO)
dyn = next(s for s in items if s['Type'] == gate.DYNAMIC)
changed('stack executable', stack['HeaderOffset'] + 4, '<I', 7, 'STACK-RW-NX')
changed('load writable executable', load['HeaderOffset'] + 4, '<I', 7, 'NO-WX-LOAD')
changed('missing relro', relro['HeaderOffset'], '<I', 0, 'RELRO-REQUIRED')
changed('interpreter present', relro['HeaderOffset'], '<I', 3, 'NO-INTERPRETER')
changed('wrong architecture', 18, '<H', 3, 'STATIC-PIE-X86-64')
changed('entry outside executable', 24, '<Q', 0, 'ENTRY-IN-EXECUTABLE-FILE-REGION')
changed('phdr beyond file', 32, '<Q', len(raw) + 1, 'PHDR-BOUNDS')
changed('file span overflow', load['HeaderOffset'] + 32, '<Q', len(raw) + 1, 'SEGMENT-FILE-BOUNDS')
cases.append(('truncated', raw[:40], 'ELF64-LE'))
data = bytearray(raw)
for at in range(dyn['Offset'], dyn['Offset'] + dyn['FileBytes'], 16):
    tag, value = struct.unpack_from('<QQ', data, at)
    if tag == 24:
        struct.pack_into('<Q', data, at, 21)
    elif tag in (30, 0x6FFFFFFB):
        struct.pack_into('<Q', data, at + 8, value & ~(8 if tag == 30 else 1))
cases.append(('missing now', data, 'DYNAMIC-NOW'))
for at in range(dyn['Offset'], dyn['Offset'] + dyn['FileBytes'], 16):
    if struct.unpack_from('<Q', raw, at)[0] == 21:
        changed('needed library', at, '<Q', 1, 'NO-DT-NEEDED')
        changed('text relocations', at, '<Q', 22, 'NO-TEXTREL')
        break
else:
    raise AssertionError('fixture lacks DT_DEBUG')
prop = next(s for s in items if s['Type'] == 0x6474E553)
at = raw.find(struct.pack('<II', 0xC0000002, 4), prop['Offset'], prop['Offset'] + prop['FileBytes'])
assert at >= 0
changed('missing cet bits', at + 8, '<I', 0, 'CET-PROPERTY-IBT-SHSTK')
# Locate main via symbol metadata, then mutate its actual file byte.
symbols = gate.run_tool('nm', '-S', '--defined-only', str(binary.resolve()))
main = next(line.split() for line in symbols.splitlines() if line.split()[-1] == 'main')
va = int(main[0], 16)
owner = next(s for s in items if s['Type'] == gate.LOAD and s['Address'] <= va < s['Address'] + s['FileBytes'])
changed('main entry changed', owner['Offset'] + va - owner['Address'], '<B', 0x90, 'MAIN-ENDBR64')

with tempfile.TemporaryDirectory(prefix='ultra-elf-tests-') as temp:
    root = Path(temp)
    gate.audit(binary, root / 'valid')
    for i, (label, data, expected) in enumerate(cases):
        path, out = root / f'bad-{i}', root / f'report-{i}'
        path.write_bytes(data)
        try:
            gate.audit(path, out)
        except ValueError as error:
            assert str(error) == expected, (label, str(error), expected)
            assert not out.exists(), 'failed audit published a report'
            print(f'RECHAZO-OK: {label}: {expected}')
        else:
            raise AssertionError('mutation accepted: ' + label)
    try:
        gate.audit(binary, root / 'valid')
    except FileExistsError:
        print('RECHAZO-OK: existing report cannot be reused')
    else:
        raise AssertionError('old report reused')
print(f'ELF-TESTS-OK: valid control, {len(cases)} mutations, stale output rejected; no mutated ELF executed')
