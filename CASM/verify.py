"""Validate the bounded CASM emission contract, then publish a FAGRAW64 packet."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import struct


def require(condition, rule):
    if not condition:
        raise ValueError(rule)


def verify(source_path, hex_path, ir_path):
    source = source_path.read_bytes()
    require(len(source) <= 65536 and all(0 < b < 128 for b in source), 'SOURCE-ASCII')
    require(hex_path.stat().st_size <= 32768 and ir_path.stat().st_size <= 131072, 'ARTIFACT-SIZE')
    tokens = []
    for line in hex_path.read_text(encoding='ascii').splitlines():
        tokens.extend(line.split(';', 1)[0].split())
    require(all(re.fullmatch(r'[0-9A-Fa-f]{2}', t) for t in tokens), 'HEX-TOKEN')
    body = bytes(int(t, 16) for t in tokens)
    ir = json.loads(ir_path.read_text(encoding='ascii'))
    require(ir['Schema'] == 1 and ir['Target'] == 'x86_64' and ir['ABI'] == 'metal64', 'IR-CONTRACT')
    require(1 <= len(body) <= 4072 and ir['BodyBytes'] == len(body), 'BODY-LIMIT')
    constants, calls = ir['Constants'], ir['Calls']
    require(isinstance(constants, list) and isinstance(calls, list), 'IR-LISTS')
    require(len(constants) <= 128 and len(calls) <= 128, 'IR-LIMIT')
    expected = bytearray(11 * len(calls) + 1)
    expected[-1] = 0xC3
    names = set()
    line_count = source.count(b'\n') + 1
    mapping = []

    def line_number(value):
        require(type(value) is int and 1 <= value <= line_count, 'SOURCE-LINE')
        return value

    for constant in constants:
        name = constant['Name']
        require(isinstance(name, str) and re.fullmatch(r'[A-Za-z][A-Za-z0-9_]{0,62}', name), 'IR-NAME')
        require(name not in names, 'IR-DUPLICATE')
        names.add(name)
        line_number(constant['Line'])
        require(type(constant['Offset']) is int and constant['Offset'] == len(expected), 'DATA-OFFSET')
        raw_hex = constant['Hex']
        require(isinstance(raw_hex, str) and re.fullmatch(r'(?:[0-9A-F]{2})+', raw_hex), 'DATA-HEX')
        data = bytes.fromhex(raw_hex)
        require(1 <= len(data) <= 4096 and data[-1] == 0, 'STRING-END')
        require(all(b in (10, 13) or 32 <= b <= 126 for b in data[:-1]), 'STRING-ASCII')
        expected.extend(data)
        mapping.append(dict(Line=constant['Line'], Offset=constant['Offset'], Bytes=len(data), Kind='immutable-ascii-z', Name=name))
    require(len(expected) == len(body), 'REGION-SIZE')
    for index, call in enumerate(calls):
        at = 11 * index
        require(type(call['Offset']) is int and call['Offset'] == at, 'CODE-OFFSET')
        target = call['Constant']
        require(type(target) is int and 0 <= target < len(constants), 'CALL-TARGET')
        line_number(call['Line'])
        require(constants[target]['Line'] <= call['Line'], 'DECLARATION-ORDER')
        offset = constants[target]['Offset']
        displacement = offset - (at + 7)
        instruction = b'\x48\x8d\x35' + struct.pack('<i', displacement) + b'\x41\xff\x57\x10'
        expected[at:at + 11] = instruction
        mapping.append(dict(Line=call['Line'], Offset=at, Bytes=11, Kind='metal_puts', Constant=constants[target]['Name'], Hex=instruction.hex().upper()))
    ret = ir['Return']
    require(type(ret['Offset']) is int and ret['Offset'] == 11 * len(calls), 'RETURN-OFFSET')
    line_number(ret['Line'])
    require(all(c['Line'] <= ret['Line'] for c in constants + calls), 'RETURN-ORDER')
    require(body == expected, 'EMISSION-MISMATCH: bytes do not match the allowed IR templates')
    mapping.append(dict(Line=ret['Line'], Offset=ret['Offset'], Bytes=1, Kind='return', Hex='C3'))
    packet = b'FAGRAW64' + struct.pack('<IIII', len(body), 0, sum(body), 0) + body
    report = dict(Schema=1, Verified=True, Target='x86_64', ABI='metal64',
                  SourceSHA256=hashlib.sha256(source).hexdigest(),
                  HexSHA256=hashlib.sha256(hex_path.read_bytes()).hexdigest(),
                  IRSHA256=hashlib.sha256(ir_path.read_bytes()).hexdigest(),
                  PacketSHA256=hashlib.sha256(packet).hexdigest(), PacketBytes=len(packet),
                  BodyBytes=len(body), Mapping=sorted(mapping, key=lambda row: row['Offset']),
                  Scope='Emission structure verified against compiler IR; source semantics rely on the C frontend. No memory-safety proof.')
    return packet, report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('hex', type=Path)
    parser.add_argument('ir', type=Path)
    parser.add_argument('--packet', required=True, type=Path)
    parser.add_argument('--report', required=True, type=Path)
    args = parser.parse_args()
    try:
        outputs = [args.packet.resolve(), args.report.resolve()]
        inputs = [p.resolve() for p in (args.source, args.hex, args.ir)]
        require(len(set(outputs)) == 2 and not any(p in inputs for p in outputs), 'OUTPUT-ALIAS')
        packet, report = verify(args.source, args.hex, args.ir)
        args.packet.write_bytes(packet)
        args.report.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    except (ValueError, KeyError, IndexError, TypeError, OSError, struct.error) as error:
        raise SystemExit(f'RED [VERIFY] {error}') from error
    print(f'GREEN [VERIFY] {len(packet)} bytes packaged; emission contract passed.')


if __name__ == '__main__':
    main()
