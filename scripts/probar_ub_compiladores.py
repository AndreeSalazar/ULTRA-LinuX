#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Casos sinteticos C/C++ con GCC y Clang. Ejecutar solo en el taller QEMU."""
import hashlib
import json
import os
from pathlib import Path
import resource
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT/'build/ub-compiladores'
HEADER = '#include <stdio.h>\n#include <stdlib.h>\n#include <limits.h>\n'
CASES = [
    ('desborde-signado', 'volatile int n=INT_MAX; printf("%d\\n",n+1);',
     'volatile int n=INT_MAX-1; printf("%d\\n",n+1);', 'runtime error:', None),
    ('division-cero', 'volatile int n=0; printf("%d\\n",42/n);',
     'volatile int n=2; printf("%d\\n",42/n);', 'runtime error:', None),
    ('desplazamiento-ancho', 'volatile unsigned n=sizeof(unsigned)*CHAR_BIT; printf("%u\\n",1u<<n);',
     'volatile unsigned n=1; printf("%u\\n",1u<<n);', 'runtime error:', None),
    ('vida-util', 'int *p=(int*)malloc(sizeof(int)); if(!p)return 2; *p=7; free(p); printf("%d\\n",*p);',
     'int *p=(int*)malloc(sizeof(int)); if(!p)return 2; *p=7; printf("%d\\n",*p); free(p);', 'AddressSanitizer: heap-use-after-free', None),
    ('limite-memoria', 'int *p=(int*)calloc(2,sizeof(int)); if(!p)return 2; volatile int i=2; printf("%d\\n",p[i]); free(p);',
     'int *p=(int*)calloc(2,sizeof(int)); if(!p)return 2; volatile int i=1; printf("%d\\n",p[i]); free(p);', ('AddressSanitizer: heap-buffer-overflow', 'insufficient space for an object'), None),
    ('liberacion-cpp', 'int *p=new int[2]; printf("%p\\n",(void*)p); delete p;', 'int *p=new int[2]; printf("%p\\n",(void*)p); delete[] p;',
     'AddressSanitizer: alloc-dealloc-mismatch', 'cpp'),
]


def limits():
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    resource.setrlimit(resource.RLIMIT_CPU, (5, 5))
    resource.setrlimit(resource.RLIMIT_FSIZE, (1048576, 1048576))


def main():
    if os.getuid() == 0:
        raise SystemExit('Ejecutar sin root en el taller QEMU')
    OUT.mkdir(parents=True, exist_ok=True)
    if OUT.resolve() != OUT:
        raise SystemExit('Directorio de banco redirigido')
    rows = []
    for compiler, language, standard in [('gcc','c','c17'),('g++','cpp','c++20'),('clang','c','c17'),('clang++','cpp','c++20')]:
        version = subprocess.check_output([compiler, '--version'], text=True, timeout=10).splitlines()[0]
        for name, bad, good, marker, only in CASES:
            if only and only != language:
                continue
            for kind, body in [('mal',bad),('bien',good)]:
                base = OUT/f'{compiler}-{name}-{kind}'
                source = base.with_suffix('.'+language)
                binary = base.with_suffix('.elf')
                source.write_text(HEADER+'int main(void) { '+body+' return 0; }\n')
                flags = ['-std='+standard, '-O1', '-g', '-fno-omit-frame-pointer',
                         '-fsanitize=address,undefined', '-fno-sanitize-recover=all']
                build = subprocess.run([compiler, *flags, str(source), '-o', str(binary)], capture_output=True, text=True, timeout=60)
                assert build.returncode == 0, (compiler,name,build.stderr)
                env = {**os.environ, 'ASAN_OPTIONS':'detect_leaks=1:halt_on_error=1:alloc_dealloc_mismatch=1',
                       'UBSAN_OPTIONS':'halt_on_error=1:print_stacktrace=1'}
                run = subprocess.run([str(binary)], capture_output=True, text=True, timeout=10, env=env, preexec_fn=limits)
                diagnostic = run.stdout+run.stderr
                markers = (marker,) if isinstance(marker,str) else marker
                matched = next((m for m in markers if m in diagnostic),None)
                if kind == 'bien':
                    passed = run.returncode == 0 and 'runtime error:' not in diagnostic and 'AddressSanitizer' not in diagnostic
                else:
                    passed = run.returncode != 0 and matched is not None
                (base.with_suffix('.log')).write_text(build.stderr+diagnostic)
                rows.append({'compiler':compiler,'version':version,'language':standard,'case':name,'kind':kind,
                             'flags':flags,'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),
                             'exit':run.returncode,'expected':marker if kind=='mal' else 'salida limpia',
                             'diagnostic_match':matched if kind=='mal' else None,'passed':passed})
                print(('GREEN' if passed else 'RED')+f' [UB] {compiler} {name} {kind}', flush=True)
    report = {'schema':1,'passed':sum(r['passed'] for r in rows),'total':len(rows),'cases':rows,
              'limits':['Solo cubre estos casos y sus caminos ejecutados; no prueba todos los UB de C/C++.',
                        'Clang es el frontend de C/C++ de LLVM; no se modifica el compilador ni la produccion.']}
    (OUT/'resultados.json').write_text(json.dumps(report,indent=2)+'\n')
    return 0 if report['passed']==report['total'] else 1


if __name__ == '__main__':
    sys.exit(main())
