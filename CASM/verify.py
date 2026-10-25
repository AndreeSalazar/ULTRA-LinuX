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


# CASM 0.2: exact integers. The verifier recomputes every value from its postfix form with
# unbounded Python integers, applying the same rules as the compiler, and must agree.
INT_TYPES = {'uint8_t': (8, False), 'uint16_t': (16, False), 'uint32_t': (32, False), 'uint64_t': (64, False),
             'int8_t': (8, True), 'int16_t': (16, True), 'int32_t': (32, True), 'int64_t': (64, True)}
NAME = r'[A-Za-z][A-Za-z0-9_]{0,62}'


def fits(value, type_name):
    bits, signed = INT_TYPES[type_name]
    low, high = (-(1 << (bits - 1)), (1 << (bits - 1)) - 1) if signed else (0, (1 << bits) - 1)
    return low <= value <= high


def evaluate(rpn, type_name, known):
    """Exact value of a postfix expression in one declared type; every step must fit."""
    require(isinstance(rpn, str), 'INT-RPN')
    tokens = rpn.split()
    require(1 <= len(tokens) <= 256, 'INT-RPN')
    stack = []
    for token in tokens:
        if re.fullmatch(r'-?(?:0|[1-9][0-9]*)', token):
            require(abs(int(token)) <= (1 << 64) - 1, 'INT-RPN')
            stack.append(int(token))
        elif token.startswith('@'):
            require(token[1:] in known, 'INT-RPN')
            stack.append(known[token[1:]])
        elif token == 'neg':
            require(len(stack) >= 1, 'INT-RPN')
            stack.append(-stack.pop())
        elif token in ('+', '-', '*', '/', '%'):
            require(len(stack) >= 2, 'INT-RPN')
            b, a = stack.pop(), stack.pop()
            if token in ('/', '%'):
                require(b != 0, 'INT-DIV-ZERO')
                quotient = abs(a) // abs(b) * (1 if (a < 0) == (b < 0) else -1)  # truncates toward zero
                stack.append(quotient if token == '/' else a - b * quotient)
            else:
                stack.append(a + b if token == '+' else a - b if token == '-' else a * b)
        else:
            raise ValueError('INT-RPN')
        require(fits(stack[-1], type_name), 'INT-OVERFLOW')
    require(len(stack) == 1, 'INT-RPN')
    return stack[0]


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
    require(ir['Schema'] == 2 and ir['Target'] == 'x86_64' and ir['ABI'] == 'metal64', 'IR-CONTRACT')
    require(1 <= len(body) <= 4072 and ir['BodyBytes'] == len(body), 'BODY-LIMIT')
    constants, calls, integers = ir['Constants'], ir['Calls'], ir['Integers']
    require(isinstance(constants, list) and isinstance(calls, list) and isinstance(integers, list), 'IR-LISTS')
    require(len(constants) <= 128 and len(calls) <= 128 and len(integers) <= 128, 'IR-LIMIT')
    expected = bytearray(11 * len(calls) + 1)
    expected[-1] = 0xC3
    names = set()
    line_count = source.count(b'\n') + 1
    mapping = []

    def line_number(value):
        require(type(value) is int and 1 <= value <= line_count, 'SOURCE-LINE')
        return value

    values, integer_lines = {}, {}
    for integer in integers:
        name = integer['Name']
        require(isinstance(name, str) and re.fullmatch(NAME, name), 'IR-NAME')
        require(name not in names, 'IR-DUPLICATE')
        names.add(name)
        require(integer['Type'] in INT_TYPES, 'INT-TYPE')
        integer_lines[name] = line_number(integer['Line'])
        value = evaluate(integer['Rpn'], integer['Type'], values)
        written = integer['Value']
        require(isinstance(written, str) and re.fullmatch(r'-?(?:0|[1-9][0-9]*)', written) and int(written) == value, 'INT-VALUE')
        values[name] = value
    for constant in constants:
        name, kind = constant['Name'], constant.get('Kind')
        if kind == 'decimal':
            # The decimal rendering of one integer: '#' + its name, its line and exactly its digits.
            owner = constant.get('Integer')
            require(isinstance(name, str) and owner in values and name == '#' + owner and constant['Line'] == integer_lines[owner], 'DATA-DECIMAL')
            require(constant['Hex'] == (str(values[owner]).encode('ascii') + b'\0').hex().upper(), 'DATA-DECIMAL')
        else:
            require(kind == 'ascii', 'IR-CONTRACT')
            require(isinstance(name, str) and re.fullmatch(NAME, name), 'IR-NAME')
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
        mapping.append(dict(Line=constant['Line'], Offset=constant['Offset'], Bytes=len(data), Kind='immutable-ascii-z' if kind == 'ascii' else 'decimal-ascii-z', Name=name))
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
                  Integers=[dict(Name=n['Name'], Type=n['Type'], Line=n['Line'], Value=n['Value']) for n in integers],
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
        # KeyError: a required IR field is missing; it is named in the message.
        raise SystemExit(f'RED [VERIFY] {error}') from error
    print(f'GREEN [VERIFY] {len(packet)} bytes packaged; emission contract passed.')


if __name__ == '__main__':
    main()
