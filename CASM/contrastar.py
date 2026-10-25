#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Contrasta semillas, katanas y emision. No confunde bootstrap con self-hosting."""
import json
import os
from pathlib import Path
import random
import subprocess
import sys
import tempfile

import bootstrap
from verify import verify

ROOT = bootstrap.ROOT
BASE = bootstrap.BASE


def run(command, env=None):
    result = subprocess.run(command, capture_output=True, text=True, timeout=180, env=env)
    if result.returncode:
        raise RuntimeError(f'{command[0]} fallo ({result.returncode}): {result.stdout[-2000:]} {result.stderr[-4000:]}')
    return result


def main():
    results = []
    binaries = []
    for compiler in ('gcc', 'clang'):
        for diagnostic in (False, True):
            variant = compiler + ('-diagnostico' if diagnostic else '-normal')
            binary = bootstrap.build(compiler, variant, diagnostic)
            binaries.append(binary)
            output = BASE / ('pruebas-'+variant)
            env = {**os.environ, 'CASM_TEST_COMPILER': str(binary), 'CASM_TEST_OUTPUT': str(output),
                   'ASAN_OPTIONS': 'detect_leaks=1:halt_on_error=1:allocator_may_return_null=1',
                   'UBSAN_OPTIONS': 'halt_on_error=1:print_stacktrace=1'}
            if diagnostic:
                env['CASM_TEST_SANITIZED'] = '1'
            else:
                env.pop('CASM_TEST_SANITIZED', None)
            test = run([sys.executable, '-B', str(ROOT/'CASM/test.py')], env)
            report = json.loads((output/'test-results.json').read_text())
            (output/'banco.log').write_text(test.stdout+test.stderr)
            results.append({'variant': variant, 'passed': report['Passed'], 'skipped': report['Skipped'],
                            'hello_sha256': report['HelloPacketSHA256'], 'binary_sha256': bootstrap.sha(binary)})
            print(f'GREEN [SEMILLA] {variant}: {report["Passed"]} casos; {len(report["Skipped"])} omisiones declaradas', flush=True)
    # Un corpus determinista de expresiones admitidas: los cuatro anfitriones deben
    # emitir exactamente el mismo HEX, IR y paquete, verificado de forma independiente.
    rng = random.Random(0xCA5A)
    with tempfile.TemporaryDirectory(prefix='diferencial-', dir=BASE) as tmp:
        work = Path(tmp)
        for index in range(128):
            a, b, c = rng.randrange(10000), rng.randrange(10000), rng.randrange(1, 100)
            source = work/'source.casm'
            source.write_text(f'void metal_main(void) {{ const int64_t n = ({a} - {b}) * {c}; metal_put_int(n); return; }}\n')
            expected = None
            for variant, binary in enumerate(binaries):
                hp, ip = work/f'{variant}.hex', work/f'{variant}.json'
                run([str(binary), str(source), str(hp), str(ip)])
                packet, report = verify(source, hp, ip)
                assert report['Integers'][0]['Value'] == str((a-b)*c)
                observed = (hp.read_bytes(), ip.read_bytes(), packet)
                if expected is None:
                    expected = observed
                assert observed == expected, f'emision distinta: caso {index}, {binary}'
    # Probar ausencia de dependencia operacional de GCC y Clang: PATH contiene solo
    # herramientas necesarias del script, y dos trampas que fallan si se los invoca.
    bootstrap.build('gcc', 'host')
    with tempfile.TemporaryDirectory(prefix='sin-compilador-', dir=BASE) as tmp:
        work = Path(tmp)
        for name in ('gcc', 'clang'):
            p = work/name
            p.write_text('#!/bin/sh\necho COMPILADOR-EXTERNO-INVOCADO >&2\nexit 99\n')
            p.chmod(0o755)
        env = {**os.environ, 'PATH': str(work)+os.pathsep+os.environ['PATH']}
        status = run([sys.executable, str(ROOT/'CASM/bootstrap.py'), 'asegurar'], env)
        assert 'compilador vigente' in status.stdout
        compiled = run(['/usr/bin/bash', str(ROOT/'CASM/compilar.sh'), '--linux'], env)
        assert 'GREEN [READY]' in compiled.stdout
        line = next(l for l in compiled.stdout.splitlines() if 'GREEN [READY]' in l)
        payload = Path(line.split(': ',1)[1]).with_name('program.elf')
        native = subprocess.run([str(payload)],capture_output=True,timeout=5)
        assert native.returncode == 0 and native.stdout == b'Hola mundo desde mi .hex!\r\n', native
        encoded = payload.with_suffix('.linux.hex').read_text()
        assert bytes.fromhex(encoded) == payload.read_bytes()
    report = {'schema': 1, 'self_hosted': False, 'target_self_host': 'Linux ELF x86-64',
              'variants': results, 'differential_cases': 128, 'emissions_verified': 512,
              'hex_ir_packet_equal': True, 'cached_compilation_without_external_compiler': True,
              'linux_elf_executed': True, 'linux_elf_hex_roundtrip': True,
              'limits': ['No demuestra ausencia total de UB ni independencia de la semilla inicial.',
                         'CASM aun no puede expresar ni compilar su propio frontend C.']}
    (BASE/'contraste.json').write_text(json.dumps(report, indent=2)+'\n')
    print('GREEN [CONTRASTE] 512 emisiones identicas; compilacion con GCC/Clang bloqueados; self_hosted=false')


if __name__ == '__main__':
    main()
