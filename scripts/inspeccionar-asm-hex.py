#!/usr/bin/env python3
"""Inspect four known ELF symbols without executing the inspected binary."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

SYMBOLS = ('ultra_abi_probe', 'ultra_abi_good', 'ultra_abi_bad',
           'ultra_abi_bad_alignment')
# GNU assembler encoding of: mov rsp,rax; and $15,eax; ret.
# Deliberately bounded contract, not a general x86 decoder/safety proof.
GOOD = bytes.fromhex('48 89 E0 83 E0 0F C3')


def check_good(data):
    if data != GOOD:
        raise ValueError('HEX-CONTRACT: ultra_abi_good differs from reviewed bytes')


def extract(disassembly, symbol):
    header = re.compile(r'^\s*[0-9a-f]+ <' + re.escape(symbol) + r'>:$')
    found = False
    result = bytearray()
    for line in disassembly.splitlines():
        if header.fullmatch(line):
            found = True
            continue
        if not found:
            continue
        if re.match(r'^\s*[0-9a-f]+ <[^>]+>:$', line):
            break
        fields = line.split('\t')
        if len(fields) >= 3 and re.fullmatch(r'\s*[0-9a-f]+:', fields[0]):
            tokens = fields[1].split()
            if not tokens or not all(re.fullmatch(r'[0-9a-f]{2}', t) for t in tokens):
                raise ValueError('OBJDUMP-FORMAT')
            result.extend(int(t, 16) for t in tokens)
    if not found or not result:
        raise ValueError('SYMBOL-MISSING: ' + symbol)
    return bytes(result)


def selftest():
    check_good(GOOD)
    rejected = 0
    for data in (b'', GOOD[:-1], GOOD + b'\x90', bytes([GOOD[0] ^ 1]) + GOOD[1:]):
        try:
            check_good(data)
        except ValueError:
            rejected += 1
    assert rejected == 4
    fixture = '00000100 <ultra_abi_good>:\n 100:\t48 89 e0             \tmov %rsp,%rax\n 103:\t83 e0 0f             \tand $15,%eax\n 106:\tc3                   \tret\n'
    assert extract(fixture, 'ultra_abi_good') == GOOD
    try:
        extract(fixture, 'absent')
    except ValueError:
        pass
    else:
        raise AssertionError('missing symbol accepted')
    print('HEX-SELFTEST-OK: valid bytes, four mutations, extraction and missing symbol')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', nargs='?', type=Path)
    parser.add_argument('--out', type=Path)
    parser.add_argument('--selftest', action='store_true')
    args = parser.parse_args()
    if args.selftest:
        selftest()
        return
    if not args.binary or not args.out:
        parser.error('binary and --out are required')
    raw = args.binary.read_bytes()
    if len(raw) < 64 or raw[:6] != b'\x7fELF\x02\x01' or int.from_bytes(raw[18:20], 'little') != 62:
        raise ValueError('ELF-CONTRACT: requires little-endian ELF64 x86-64')
    records = []
    listings = []
    for symbol in SYMBOLS:
        listing = subprocess.check_output(
            ['objdump', '-d', '-Mintel', '--insn-width=16', '--disassemble=' + symbol,
             str(args.binary.resolve())], text=True, env={'PATH': '/usr/bin', 'LC_ALL': 'C'})
        data = extract(listing, symbol)
        if symbol == 'ultra_abi_good':
            check_good(data)
        records.append(dict(Symbol=symbol, Bytes=len(data), Hex=data.hex(' ').upper(),
                            SHA256=hashlib.sha256(data).hexdigest(),
                            ExactContract=symbol == 'ultra_abi_good'))
        listings.append(listing)
    # Publish only after all selected symbols and the exact contract pass.
    args.out.mkdir(parents=True, exist_ok=False)
    report = dict(Schema=1, BinarySHA256=hashlib.sha256(raw).hexdigest(),
                  InspectorSHA256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                  Tool=subprocess.check_output(['objdump', '--version'], text=True).splitlines()[0],
                  Symbols=records, Scope='Static inspection only; exact bytes for one leaf. No execution, UB/lifetime proof or speed claim.')
    (args.out / 'informe.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    (args.out / 'desensamblado.txt').write_text('\n'.join(listings), encoding='utf-8')
    for record in records:
        (args.out / (record['Symbol'] + '.hex')).write_text(record['Hex'] + '\n', encoding='ascii')
    print('HEX-OK: four symbols inspected; one exact byte contract passed')


if __name__ == '__main__':
    main()
