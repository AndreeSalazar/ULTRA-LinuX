"""Exercise source rejection, byte verification and the existing metal64 ABI."""
import copy
import hashlib
import resource
import json
from pathlib import Path
import struct
import subprocess
import sys
import tempfile

from verify import verify

ROOT = Path(__file__).resolve().parents[1]
COMPILER = ROOT / 'build/CASM/host/casmc'
OUTPUT = ROOT / 'build/CASM'
RESULTS = []
HELLO = (ROOT / 'CASM/examples/hello.casm').read_bytes()
ENTEROS = (ROOT / 'CASM/examples/enteros.casm').read_bytes()
# Bordes exactos de la aritmetica de CASM: cada valor, tal cual debe salir en el .hex.
BORDES = (b'void metal_main(void) {'
          b' const uint64_t max = 18446744073709551615; metal_put_int(max);'
          b' const int64_t min = -9223372036854775808; metal_put_int(min);'
          b' const uint8_t hex = 0xFF; metal_put_int(hex);'
          b' const int64_t r0 = -9223372036854775808 % -1; metal_put_int(r0);'
          b' const int32_t q = -7 / 2; metal_put_int(q);'
          b' const int32_t r1 = -7 % 2; metal_put_int(r1);'
          b' const int32_t r2 = 7 % -2; metal_put_int(r2);'
          b' const int16_t p = (2 + 3) * -(4 - 10); metal_put_int(p);'
          b' return; }')
BORDES_VALORES = ['18446744073709551615', '-9223372036854775808', '255', '0', '-3', '-1', '1', '30']


def compile_case(folder, source):
    folder.mkdir()
    paths = [folder / name for name in ('source.casm', 'program.hex', 'program.ir.json')]
    paths[0].write_bytes(source)
    process = subprocess.run([str(COMPILER), *map(str, paths)], capture_output=True, text=True, timeout=10)
    return process, paths


def main():
    OUTPUT.mkdir(exist_ok=True, parents=True)
    with tempfile.TemporaryDirectory(prefix='tests-', dir=OUTPUT) as temporary:
        work = Path(temporary)
        valid = {
            'hello': HELLO,
            'empty-entry': b'void metal_main(void) { return; }',
            'multiple-calls': b'void metal_main(void) { const char a[]="A"; metal_puts(a); const char b[]="B"; metal_puts(b); metal_puts(a); return; }',
            'unused-warning': b'void metal_main(void) { const char a[]="A"; return; }',
            'comments-crlf': b'/* header */\r\nvoid metal_main(void) { // comment\r\n const char a[]="\\\"\\\\\\n\\r"; metal_puts(a); return; }',
            'packet-4096': b'void metal_main(void) { const char a[]="'+ b'A'*4059 + b'"; metal_puts(a); return; }',
            'integers': ENTEROS,
            'integer-edges': BORDES,
        }
        hello_paths = None
        for name, source in valid.items():
            process, paths = compile_case(work/name, source)
            assert process.returncode == 0, (name, process.stderr)
            packet, report = verify(*paths)
            size, entry, checksum, flags = struct.unpack_from('<IIII', packet, 8)
            assert packet[:8] == b'FAGRAW64' and size == len(packet)-24 and entry == flags == 0
            assert checksum == sum(packet[24:])
            if name == 'hello':
                hello_paths = paths
                # Golden byte contract from the hand-written program, not from the emitter.
                body = bytes.fromhex('48 8D 35 05 00 00 00 41 FF 57 10 C3') + b'Hola mundo desde mi .hex!\r\n\0'
                assert packet[24:] == body and len(packet) == 64
                (OUTPUT/'hello.x86').write_bytes(packet)
                (OUTPUT/'hello.hex').write_bytes(paths[1].read_bytes())
                (OUTPUT/'hello.ir.json').write_bytes(paths[2].read_bytes())
                (OUTPUT/'hello.verified.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
            if name == 'integers':
                ints_paths = paths
                assert [n['Value'] for n in report['Integers']] == ['16000000000', '-128', '-14'], report['Integers']
                assert b'16000000000\0' in packet and b'-128\0' in packet and b'-14\0' in packet
            if name == 'integer-edges':
                assert [n['Value'] for n in report['Integers']] == BORDES_VALORES, report['Integers']
                for v in BORDES_VALORES: assert v.encode() + b'\0' in packet, v
            if name == 'unused-warning': assert 'YELLOW [UNUSED-CONSTANT]' in process.stdout
            if name == 'packet-4096': assert len(packet) == 4096 and 'YELLOW [BODY-MARGIN]' in process.stdout
            RESULTS.append(dict(Case=name, Passed=True, Kind='accepted-and-verified'))
        invalid = {
            'unknown-operation': (b'void metal_main(void){ screen_clear(); return; }','C-SUBSET'),
            'undefined-name': (b'void metal_main(void){ metal_puts(a); return; }','NAME-UNDEFINED'),
            'duplicate-name': (b'void metal_main(void){ const char a[]="A"; const char a[]="B"; return; }','NAME-DUPLICATE'),
            'wrong-type': (b'void metal_main(void){ const char a[]=1; return; }','TYPE-STRING'),
            'keyword-name': (b'void metal_main(void){ const char while[]="A"; return; }','NAME-RESERVED'),
            'runtime-name': (b'void metal_main(void){ const char metal_puts[]="A"; return; }','NAME-RESERVED'),
            'raw-pointer': (b'void metal_main(void){ const char *a="A"; return; }','TOKEN-UNSUPPORTED'),
            'write-to-constant': (b'void metal_main(void){ const char a[]="A"; a[0]=1; return; }','C-SUBSET'),
            'missing-return': (b'void metal_main(void){}','RETURN-REQUIRED'),
            'after-return': (b'void metal_main(void){return; metal_puts(a);}','AFTER-RETURN'),
            'trailing-source': (b'void metal_main(void){return;} void other(void){}','TRAILING-SOURCE'),
            'bad-escape': (b'void metal_main(void){const char a[]="\\0";return;}','STRING-ESCAPE'),
            'newline-in-string': (b'void metal_main(void){const char a[]="A\nB";return;}','STRING-ASCII'),
            'unterminated-string': (b'void metal_main(void){const char a[]="A','STRING-END'),
            'unterminated-comment': (b'/* missing end','COMMENT-END'),
            'preprocessor': (b'#include <stdio.h>\nvoid metal_main(void){return;}','TOKEN-UNSUPPORTED'),
            'non-ascii': (b'//\xff\nvoid metal_main(void){return;}','SOURCE-ASCII'),
            'nul-source': (b'void metal_main(void){return;}\0','SOURCE-ASCII'),
            'long-identifier': (b'void metal_main(void){const char '+b'a'*64+b'[]="A";return;}','NAME-SIZE'),
            'body-too-large': (b'void metal_main(void){const char a[]="'+b'A'*4060+b'";metal_puts(a);return;}','BODY-LIMIT'),
            'source-too-large': (b' '*65537,'SOURCE-SIZE'),
            'call-limit': (b'void metal_main(void){const char a[]="A";'+b'metal_puts(a);'*129+b'return;}','CALL-LIMIT'),
            'old-proposal': (b'target x86_64; program hello {return;}','C-SUBSET'),
            'constant-limit': (b'void metal_main(void){'+b''.join(b'const char c%d[]="A";' % i for i in range(129))+b'return;}','CONSTANT-LIMIT'),
            'decl-name': (b'void metal_main(void){ const char []="A"; return; }','DECL-NAME'),
            'syntax': (b'void metal_main(void){ return }','SYNTAX'),
            'string-size': (b'void metal_main(void){const char a[]="'+b'A'*4100+b'";return;}','STRING-SIZE'),
            # CASM 0.2: enteros exactos. Nada da la vuelta, nada es octal, nada se divide por cero.
            'int-sum-overflow': (b'void metal_main(void){const uint8_t a=255+1;return;}','INT-OVERFLOW'),
            'int-difference-underflow': (b'void metal_main(void){const uint32_t a=0-1;return;}','INT-OVERFLOW'),
            'int-product-overflow': (b'void metal_main(void){const int64_t a=4294967296*4294967296;return;}','INT-OVERFLOW'),
            'int-quotient-overflow': (b'void metal_main(void){const int64_t a=-9223372036854775808/-1;return;}','INT-OVERFLOW'),
            'int-negation-overflow': (b'void metal_main(void){const int8_t a=-(-128);return;}','INT-OVERFLOW'),
            'int-literal-range': (b'void metal_main(void){const int8_t a=128;return;}','INT-OVERFLOW'),
            'int-negative-unsigned': (b'void metal_main(void){const uint16_t a=-1;return;}','INT-OVERFLOW'),
            'int-reference-range': (b'void metal_main(void){const int64_t a=300;const uint8_t b=a;return;}','INT-OVERFLOW'),
            'int-division-by-zero': (b'void metal_main(void){const uint16_t a=7/0;return;}','INT-DIV-ZERO'),
            'int-remainder-by-zero': (b'void metal_main(void){const int32_t a=7%(3-3);return;}','INT-DIV-ZERO'),
            'int-octal-literal': (b'void metal_main(void){const uint8_t a=010;return;}','INT-LITERAL'),
            'int-literal-too-big': (b'void metal_main(void){const uint64_t a=18446744073709551616;return;}','INT-LITERAL'),
            'int-literal-suffix': (b'void metal_main(void){const uint64_t a=10u;return;}','INT-LITERAL'),
            'int-hex-without-digits': (b'void metal_main(void){const uint8_t a=0x;return;}','INT-LITERAL'),
            'int-literal-too-long': (b'void metal_main(void){const uint8_t a='+b'1'*4100+b';return;}','INT-LITERAL'),
            'int-string-operand': (b'void metal_main(void){const char s[]="x";const uint8_t a=s;return;}','TYPE-INT'),
            'int-string-initializer': (b'void metal_main(void){const uint8_t a="x";return;}','TYPE-INT'),
            'int-put-string': (b'void metal_main(void){const char s[]="x";metal_put_int(s);return;}','TYPE-INT'),
            'int-put-literal': (b'void metal_main(void){metal_put_int(5);return;}','TYPE-INT'),
            'int-puts-integer': (b'void metal_main(void){const uint8_t a=1;metal_puts(a);return;}','TYPE-STRING'),
            'int-undefined': (b'void metal_main(void){const uint8_t a=b+1;return;}','NAME-UNDEFINED'),
            'int-put-undefined': (b'void metal_main(void){metal_put_int(b);return;}','NAME-UNDEFINED'),
            'int-duplicate': (b'void metal_main(void){const char a[]="x";const uint8_t a=1;return;}','NAME-DUPLICATE'),
            'int-reserved-name': (b'void metal_main(void){const uint8_t int64_t=1;return;}','NAME-RESERVED'),
            'int-decl-name': (b'void metal_main(void){const uint8_t =1;return;}','DECL-NAME'),
            'int-missing-operand': (b'void metal_main(void){const uint8_t a=1+;return;}','SYNTAX'),
            'int-raw-pointer': (b'void metal_main(void){const char *p="x";return;}','TOKEN-UNSUPPORTED'),
            'int-expression-depth': (b'void metal_main(void){const uint8_t a='+b'('*40+b'1'+b')'*40+b';return;}','EXPR-DEPTH'),
            'int-negation-depth': (b'void metal_main(void){const int8_t a='+b'-'*40+b'1;return;}','EXPR-DEPTH'),
            'int-expression-size': (b'void metal_main(void){const uint64_t a=0'+b'+1'*300+b';return;}','EXPR-SIZE'),
            'int-integer-limit': (b'void metal_main(void){'+b''.join(b'const uint8_t n%d=1;' % i for i in range(129))+b'return;}','INTEGER-LIMIT'),
            'int-decimal-constant-limit': (b'void metal_main(void){'+b''.join(b'const char c%d[]="A";' % i for i in range(128))+b'const uint8_t n=1;metal_put_int(n);return;}','CONSTANT-LIMIT'),
        }
        for name, (source, rule) in invalid.items():
            process, paths = compile_case(work/name, source)
            assert process.returncode == 1 and f'RED [{rule}]' in process.stderr, (name,process.returncode,process.stderr)
            assert not paths[1].exists() and not paths[2].exists(), name
            RESULTS.append(dict(Case=name,Passed=True,Kind='source-rejected',Rule=rule))
        # Anfitrion: leer, memoria y escribir. Cada caso nombra su katana.
        def sin_memoria():
            # 256 KiB de datos: el programa arranca, pero no cabe el CasmProgram (~0,5 MiB).
            resource.setrlimit(resource.RLIMIT_DATA, (256 * 1024, 256 * 1024))
        ok = work/'host-ok.casm'; ok.write_bytes(HELLO)
        (work/'host-dir.hex').mkdir()
        host = {
            'host-read': ('HOST-READ', [work/'no-existe.casm', work/'r.hex', work/'r.ir.json'], None),
            'host-memory': ('HOST-MEMORY', [ok, work/'m.hex', work/'m.ir.json'], sin_memoria),
            'host-write': ('HOST-WRITE', [ok, work/'host-dir.hex', work/'w.ir.json'], None),
        }
        for name, (rule, paths, limite) in host.items():
            process = subprocess.run([str(COMPILER), *map(str, paths)], capture_output=True, text=True, timeout=10, preexec_fn=limite)
            assert process.returncode == 1 and f'RED [{rule}]' in process.stderr, (name, process.returncode, process.stderr)
            assert not paths[2].exists(), name
            RESULTS.append(dict(Case=name,Passed=True,Kind='host-rejected',Rule=rule))
        # Verificador: se altera UNA cosa de una emision buena y debe cortar con SU katana.
        original_ir = json.loads(hello_paths[2].read_text())
        original_hex = hello_paths[1].read_text()
        original_src = hello_paths[0].read_bytes()
        c0 = lambda ir, **k: dict(ir, Constants=[dict(ir['Constants'][0], **k)])
        mutations = {
            'verify-source-ascii': ('SOURCE-ASCII', lambda h,ir,s: (h, ir, s + b'\xff')),
            'verify-artifact-size': ('ARTIFACT-SIZE', lambda h,ir,s: (h + ';' * 33000 + '\n', ir, s)),
            'verify-hex-token': ('HEX-TOKEN', lambda h,ir,s: (h+'ZZ\n', ir, s)),
            'verify-ir-contract': ('IR-CONTRACT', lambda h,ir,s: (h, dict(ir,ABI='windows'), s)),
            'verify-body-limit': ('BODY-LIMIT', lambda h,ir,s: (h+'90\n', ir, s)),
            'verify-ir-lists': ('IR-LISTS', lambda h,ir,s: (h, dict(ir,Constants={}), s)),
            'verify-ir-limit': ('IR-LIMIT', lambda h,ir,s: (h, dict(ir,Calls=ir['Calls']*129), s)),
            'verify-ir-name': ('IR-NAME', lambda h,ir,s: (h, c0(ir, Name='1greeting'), s)),
            'verify-ir-duplicate': ('IR-DUPLICATE', lambda h,ir,s: (h, dict(ir,Constants=ir['Constants']*2), s)),
            'verify-source-line': ('SOURCE-LINE', lambda h,ir,s: (h, c0(ir, Line=99), s)),
            'verify-data-offset': ('DATA-OFFSET', lambda h,ir,s: (h, c0(ir, Offset=0), s)),
            'verify-data-hex': ('DATA-HEX', lambda h,ir,s: (h, c0(ir, Hex=ir['Constants'][0]['Hex'].lower()), s)),
            'verify-string-end': ('STRING-END', lambda h,ir,s: (h, c0(ir, Hex=ir['Constants'][0]['Hex'][:-2]), s)),
            'verify-early-nul': ('STRING-ASCII', lambda h,ir,s: (h, c0(ir, Hex='00'+ir['Constants'][0]['Hex'][2:]), s)),
            'verify-region-size': ('REGION-SIZE', lambda h,ir,s: (h+'90\n', dict(ir,BodyBytes=41), s)),
            'verify-code-offset': ('CODE-OFFSET', lambda h,ir,s: (h, dict(ir,Calls=[dict(ir['Calls'][0],Offset=1)]), s)),
            'verify-call-target': ('CALL-TARGET', lambda h,ir,s: (h, dict(ir,Calls=[dict(ir['Calls'][0],Constant=99)]), s)),
            'verify-declaration-order': ('DECLARATION-ORDER', lambda h,ir,s: (h, c0(ir, Line=5), s)),
            'verify-return-offset': ('RETURN-OFFSET', lambda h,ir,s: (h, dict(ir,Return=dict(ir['Return'],Offset=12)), s)),
            'verify-return-order': ('RETURN-ORDER', lambda h,ir,s: (h, dict(ir,Return=dict(ir['Return'],Line=2)), s)),
            'verify-opcode': ('EMISSION-MISMATCH', lambda h,ir,s: (h.replace('48 8D 35','48 89 35',1), ir, s)),
            'verify-displacement': ('EMISSION-MISMATCH', lambda h,ir,s: (h.replace('35 05','35 06',1), ir, s)),
            'verify-return-opcode': ('EMISSION-MISMATCH', lambda h,ir,s: (h.replace('10 C3','10 90',1), ir, s)),
        }
        n0 = lambda ir, **k: dict(ir, Integers=[dict(ir['Integers'][0], **k)] + ir['Integers'][1:])
        k1 = lambda ir, **k: dict(ir, Constants=[ir['Constants'][0], dict(ir['Constants'][1], **k)] + ir['Constants'][2:])
        enteros = {
            'verify-int-value': ('INT-VALUE', lambda h,ir,s: (h, n0(ir, Value='16000000001'), s)),
            'verify-int-value-format': ('INT-VALUE', lambda h,ir,s: (h, n0(ir, Value='016000000000'), s)),
            'verify-int-type': ('INT-TYPE', lambda h,ir,s: (h, n0(ir, Type='uint128_t'), s)),
            'verify-int-overflow': ('INT-OVERFLOW', lambda h,ir,s: (h, n0(ir, Type='uint32_t'), s)),
            'verify-int-div-zero': ('INT-DIV-ZERO', lambda h,ir,s: (h, n0(ir, Rpn='4000000000 0 /'), s)),
            'verify-int-rpn-stack': ('INT-RPN', lambda h,ir,s: (h, n0(ir, Rpn='4000000000 4 * +'), s)),
            'verify-int-rpn-token': ('INT-RPN', lambda h,ir,s: (h, n0(ir, Rpn='4000000000 4 shl'), s)),
            'verify-int-rpn-forward': ('INT-RPN', lambda h,ir,s: (h, n0(ir, Rpn='@mezcla'), s)),
            'verify-int-name': ('IR-NAME', lambda h,ir,s: (h, n0(ir, Name='_grande'), s)),
            'verify-int-duplicate': ('IR-DUPLICATE', lambda h,ir,s: (h, dict(ir, Integers=ir['Integers'][:1]*2+ir['Integers'][1:]), s)),
            'verify-data-decimal-digits': ('DATA-DECIMAL', lambda h,ir,s: (h, k1(ir, Hex='313630303030303030303100'), s)),
            'verify-data-decimal-owner': ('DATA-DECIMAL', lambda h,ir,s: (h, k1(ir, Integer='bajo'), s)),
            'verify-data-kind': ('IR-CONTRACT', lambda h,ir,s: (h, k1(ir, Kind='binary'), s)),
        }
        ints_ir = json.loads(ints_paths[2].read_text())
        ints_hex = ints_paths[1].read_text()
        ints_src = ints_paths[0].read_bytes()
        casos = [(n, r, m, original_hex, original_ir, original_src) for n, (r, m) in mutations.items()]
        casos += [(n, r, m, ints_hex, ints_ir, ints_src) for n, (r, m) in enteros.items()]
        for name, rule, mutate, base_hex, base_ir, base_src in casos:
            folder=work/('mutated-'+name); folder.mkdir()
            hex_text, ir, src = mutate(base_hex,copy.deepcopy(base_ir),base_src)
            sp,hp,ip=folder/'source.casm',folder/'body.hex',folder/'body.ir.json'
            sp.write_bytes(src); hp.write_text(hex_text); ip.write_text(json.dumps(ir))
            try: verify(sp,hp,ip)
            except ValueError as error: assert str(error).split(':')[0] == rule, (name, rule, str(error))
            else: raise AssertionError(f'Mutation accepted: {name}')
            # The CLI also must refuse publication after verification fails, naming the katana.
            published=folder/'INICIO.X86'
            process=subprocess.run([sys.executable,str(ROOT/'CASM/verify.py'),str(sp),str(hp),str(ip),'--packet',str(published),'--report',str(folder/'verified.json')],capture_output=True,text=True,timeout=10)
            assert process.returncode != 0 and not published.exists() and f'RED [VERIFY] {rule}' in process.stderr, (name, process.stderr)
            RESULTS.append(dict(Case=name,Passed=True,Kind='emission-rejected',Rule=rule))
        # El verificador como programa: sus propias katanas.
        folder=work/'verify-cli'; folder.mkdir()
        cli = {
            'verify-output-alias': ('OUTPUT-ALIAS', ['--packet', str(folder/'mismo'), '--report', str(folder/'mismo')], hello_paths[1]),
            'verify-missing-input': ('VERIFY', ['--packet', str(folder/'p.x86'), '--report', str(folder/'r.json')], folder/'no-existe.hex'),
        }
        for name, (rule, salida, hex_path) in cli.items():
            process=subprocess.run([sys.executable,str(ROOT/'CASM/verify.py'),str(hello_paths[0]),str(hex_path),str(hello_paths[2]),*salida],capture_output=True,text=True,timeout=10)
            assert process.returncode != 0 and f'RED [VERIFY]' in process.stderr and (rule == 'VERIFY' or rule in process.stderr), (name, process.stderr)
            assert not any(folder.glob('p.x86')) and not (folder/'mismo').exists(), name
            RESULTS.append(dict(Case=name,Passed=True,Kind='verifier-cli-rejected',Rule=rule))
    report=dict(Passed=len(RESULTS),Total=len(RESULTS),Cases=RESULTS,
                HelloPacketSHA256=hashlib.sha256((OUTPUT/'hello.x86').read_bytes()).hexdigest(),
                PhysicalInstallationChanged=False)
    (OUTPUT/'test-results.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
    print(f'GREEN [TEST] {len(RESULTS)}/{len(RESULTS)} cases passed.')


if __name__ == '__main__': main()
