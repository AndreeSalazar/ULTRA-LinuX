#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Semillas externas y uso verificado de CASM. Todavia no es autocompilacion."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / 'build/CASM'
SOURCES = ('src/casm.h', 'src/main.c', 'src/frontend.c', 'src/emitter.c', 'bootstrap.py')


def sha(path):
    with path.open('rb') as file:
        return hashlib.file_digest(file, 'sha256').hexdigest()


def directory(variant):
    if not variant or len(variant) > 40 or any(c not in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in variant):
        raise ValueError('variante: minusculas, numeros y guion, hasta 40 caracteres')
    path = BASE / variant
    if path.resolve() != path or any(p.is_symlink() for p in (ROOT/'build', BASE, path)):
        raise ValueError('directorio de construccion redirigido')
    return path


def current(path, diagnostic=None):
    try:
        manifest = path/'toolchain.json'
        if manifest.is_symlink() or not manifest.is_file() or manifest.stat().st_size > 65536:
            return False
        record = json.loads(manifest.read_text())
        binary = path/'casmc'
        return (record['schema'] == 1 and record['self_hosted'] is False
                and binary.is_file() and not binary.is_symlink()
                and os.access(binary, os.X_OK) and sha(binary) == record['binary_sha256']
                and (diagnostic is None or record['diagnostic'] == diagnostic)
                and (path/'toolchain.txt').is_file() and not (path/'toolchain.txt').is_symlink()
                and record['sources'] == {n: sha(ROOT/'CASM'/n) for n in SOURCES})
    except (OSError, ValueError, KeyError, TypeError):
        return False


def build(compiler, variant='host', diagnostic=False):
    if compiler not in ('gcc', 'clang'):
        raise ValueError('CC debe ser gcc o clang')
    path = directory(variant)
    executable = shutil.which(compiler)
    if not executable:
        raise ValueError(f'falta la semilla {compiler}; construir una vez con GCC o Clang')
    path.mkdir(parents=True, exist_ok=True)
    version = subprocess.check_output([executable, '--version'], text=True, timeout=10).splitlines()[0]
    flags = ['-std=c11', '-Wall', '-Wextra', '-Werror', '-pedantic',
             '-O1' if diagnostic else '-O2', '-D_FORTIFY_SOURCE=3',
             '-fstack-protector-strong', '-fstack-clash-protection', '-fcf-protection',
             '-fPIE', '-pie', '-Wl,-z,relro,-z,now,-z,noexecstack']
    if compiler == 'gcc' and not diagnostic:
        flags += ['-fanalyzer']
    if diagnostic:
        flags += ['-g', '-fno-omit-frame-pointer', '-fsanitize=address,undefined', '-fno-sanitize-recover=all']
    sources = [str(ROOT/'CASM'/n) for n in SOURCES if n.endswith('.c')]
    temp = path/'casmc.nuevo'
    subprocess.run([executable, *flags, *sources, '-o', str(temp)], check=True, timeout=180)
    binary = path/'casmc'
    temp.replace(binary)
    record = {'schema': 1, 'language': 'CASM 0.2', 'self_hosted': False,
              'compiler': compiler, 'version': version, 'flags': flags,
              'diagnostic': diagnostic, 'binary_sha256': sha(binary),
              'sources': {n: sha(ROOT/'CASM'/n) for n in SOURCES}}
    temp = path/'toolchain.json.nuevo'
    temp.write_text(json.dumps(record, indent=2)+'\n')
    temp.replace(path/'toolchain.json')
    (path/'toolchain.txt').write_text(version+'\nflags: '+' '.join(flags)+'\n'
        +'self_hosted: false (semilla externa; emisor x86-64 propio)\n'
        +'\n'.join(f'{v}  CASM/{n}' for n,v in record['sources'].items())
        +f'\n{record["binary_sha256"]}  build/CASM/{variant}/casmc\n')
    return binary


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('action', choices=['construir', 'asegurar', 'estado'])
    p.add_argument('--compilador', choices=['gcc', 'clang'], default=os.environ.get('CC', 'gcc'))
    p.add_argument('--variante', default='host')
    p.add_argument('--diagnostico', action='store_true')
    a = p.parse_args()
    path = directory(a.variante)
    if a.action == 'estado':
        print(json.dumps({'vigente': current(path), 'self_hosted': False,
                          'target_actual': 'metal64/FAGRAW64', 'target_autocompilacion': 'Linux ELF x86-64'}))
    elif a.action == 'construir' or not current(path, a.diagnostico):
        build(a.compilador, a.variante, a.diagnostico)
        print(f'GREEN [BOOTSTRAP] semilla {a.compilador}, variante {a.variante}')
    else:
        print('GREEN [BOOTSTRAP] compilador vigente: no se invoco GCC ni Clang')
    return 0


if __name__ == '__main__':
    try:
        sys.exit(main())
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f'RED [BOOTSTRAP] {error}', file=sys.stderr)
        sys.exit(1)
