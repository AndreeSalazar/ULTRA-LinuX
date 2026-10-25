"""Exercise source rejection, byte verification and the existing metal64 ABI."""
import copy
import hashlib
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
        }
        for name, (source, rule) in invalid.items():
            process, paths = compile_case(work/name, source)
            assert process.returncode == 1 and f'RED [{rule}]' in process.stderr, (name,process.returncode,process.stderr)
            assert not paths[1].exists() and not paths[2].exists(), name
            RESULTS.append(dict(Case=name,Passed=True,Kind='source-rejected',Rule=rule))
        original_ir = json.loads(hello_paths[2].read_text())
        original_hex = hello_paths[1].read_text()
        mutations = {
            'opcode': lambda h,ir: (h.replace('48 8D 35','48 89 35',1),ir),
            'displacement': lambda h,ir: (h.replace('35 05','35 06',1),ir),
            'return-opcode': lambda h,ir: (h.replace('10 C3','10 90',1),ir),
            'extra-byte': lambda h,ir: (h+'90\n',ir),
            'bad-hex': lambda h,ir: (h+'ZZ\n',ir),
            'wrong-abi': lambda h,ir: (h,dict(ir,ABI='windows')),
            'code-offset': lambda h,ir: (h,dict(ir,Calls=[dict(ir['Calls'][0],Offset=1)])),
            'call-target': lambda h,ir: (h,dict(ir,Calls=[dict(ir['Calls'][0],Constant=99)])),
            'data-offset': lambda h,ir: (h,dict(ir,Constants=[dict(ir['Constants'][0],Offset=0)])),
            'early-nul': lambda h,ir: (h,dict(ir,Constants=[dict(ir['Constants'][0],Hex='00'+ir['Constants'][0]['Hex'][2:])])),
        }
        for name, mutate in mutations.items():
            folder=work/('mutated-'+name); folder.mkdir()
            hex_text, ir = mutate(original_hex,copy.deepcopy(original_ir))
            hp,ip=folder/'body.hex',folder/'body.ir.json'
            hp.write_text(hex_text); ip.write_text(json.dumps(ir))
            try: verify(hello_paths[0],hp,ip)
            except ValueError: pass
            else: raise AssertionError(f'Mutation accepted: {name}')
            # The CLI also must refuse publication after verification fails.
            published=folder/'INICIO.X86'
            process=subprocess.run([sys.executable,str(ROOT/'CASM/verify.py'),str(hello_paths[0]),str(hp),str(ip),'--packet',str(published),'--report',str(folder/'verified.json')],capture_output=True,text=True,timeout=10)
            assert process.returncode != 0 and not published.exists(),name
            RESULTS.append(dict(Case=name,Passed=True,Kind='emission-rejected'))
    report=dict(Passed=len(RESULTS),Total=len(RESULTS),Cases=RESULTS,
                HelloPacketSHA256=hashlib.sha256((OUTPUT/'hello.x86').read_bytes()).hexdigest(),
                PhysicalInstallationChanged=False)
    (OUTPUT/'test-results.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
    print(f'GREEN [TEST] {len(RESULTS)}/{len(RESULTS)} cases passed.')


if __name__ == '__main__': main()
