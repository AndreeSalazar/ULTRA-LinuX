#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Inventory, not permission learning. Reads the pinned tree and measured inputs."""
import argparse
import csv
import hashlib
import json
import pathlib
import re
import subprocess

EXPECTED = '32c52ec87d6245ee2d12ed5dd509e40062939b50'
# Summaries for interfaces absent from the installed man-db index, checked
# against the definitions in this exact source commit (not guessed from nr).
SUPPLEMENT = {
    'epoll_ctl_old': ('Numero historico reservado, sin punto de entrada en esta tabla', 'arch/x86/entry/syscalls/syscall_64.tbl'),
    'epoll_wait_old': ('Numero historico reservado, sin punto de entrada en esta tabla', 'arch/x86/entry/syscalls/syscall_64.tbl'),
    'io_pgetevents': ('Recoger eventos AIO con timeout y mascara de senales', 'fs/aio.c'),
    'rseq': ('Registrar o retirar el area de secuencias reiniciables del hilo', 'kernel/rseq.c'),
    'uprobe': ('Entrada desde trampolines de uprobes x86; valida el origen del trampolin', 'arch/x86/kernel/uprobes.c'),
    'process_mrelease': ('Liberar memoria de un proceso en salida identificado por pidfd', 'mm/oom_kill.c'),
    'set_mempolicy_home_node': ('Asignar nodo de referencia NUMA a la politica de un rango de memoria', 'mm/mempolicy.c'),
    'fchmodat2': ('Cambiar permisos mediante ruta relativa y flags de resolucion', 'fs/open.c'),
    'map_shadow_stack': ('Crear un mapeo para pila sombra de usuario', 'arch/x86/kernel/shstk.c'),
    'lsm_get_self_attr': ('Consultar un atributo LSM del proceso llamante', 'security/lsm_syscalls.c'),
    'lsm_set_self_attr': ('Modificar un atributo LSM del proceso llamante', 'security/lsm_syscalls.c'),
    'lsm_list_modules': ('Enumerar los identificadores de modulos LSM activos', 'security/lsm_syscalls.c'),
    'mseal': ('Sellar un rango de mapas frente a determinados cambios de memoria virtual', 'mm/mseal.c'),
    'setxattrat': ('Establecer atributo extendido mediante descriptor base y ruta', 'fs/xattr.c'),
    'getxattrat': ('Obtener atributo extendido mediante descriptor base y ruta', 'fs/xattr.c'),
    'listxattrat': ('Listar atributos extendidos mediante descriptor base y ruta', 'fs/xattr.c'),
    'removexattrat': ('Eliminar atributo extendido mediante descriptor base y ruta', 'fs/xattr.c'),
    'file_getattr': ('Consultar atributos de archivo como flags y datos especificos del filesystem', 'fs/file_attr.c'),
    'file_setattr': ('Modificar atributos de archivo como flags y datos especificos del filesystem', 'fs/file_attr.c'),
    'listns': ('Enumerar identificadores de namespaces segun filtros y permisos', 'kernel/nstree.c'),
    'rseq_slice_yield': ('Ceder la extension de turno de CPU concedida a una seccion rseq', 'kernel/rseq.c'),
    'fchroot': ('Cambiar la raiz mediante descriptor o seleccionar la raiz especial failfs', 'fs/open.c'),
}

def git(kernel, *args):
    return subprocess.check_output(['git', '-C', str(kernel), *args], text=True)

def main():
    p = argparse.ArgumentParser()
    p.add_argument('--kernel', type=pathlib.Path, required=True)
    p.add_argument('--politica', type=pathlib.Path, required=True)
    p.add_argument('--system-map', type=pathlib.Path)
    p.add_argument('--trace', type=pathlib.Path, action='append', default=[])
    p.add_argument('--out', type=pathlib.Path, required=True)
    p.add_argument('--header', type=pathlib.Path, required=True)
    a = p.parse_args()
    commit = git(a.kernel, 'rev-parse', 'HEAD').strip()
    if commit != EXPECTED:
        raise SystemExit('ROJO: commit del catalogo distinto del kernel fijado')
    table = git(a.kernel, 'show', f'{commit}:arch/x86/entry/syscalls/syscall_64.tbl')
    grants = re.findall(r'X\((\w+),\s*"([^"]+)"\)', a.politica.read_text())
    rules = dict(grants)
    if len(grants) != len(rules):
        raise SystemExit('ROJO: declaracion duplicada')
    if not rules:
        raise SystemExit('ROJO: declaracion vacia')
    symbols = set()
    if a.system_map:
        symbols = {line.split()[-1] for line in a.system_map.read_text().splitlines() if line.strip()}
    observed = set()
    for trace in a.trace:
        for line in trace.read_text().splitlines():
            m = re.match(r'(?:\[pid\s+\d+\]\s+|\d+\s+)?([a-zA-Z0-9_]+)\(', line)
            if m:
                observed.add(m[1])
    rows = []
    for line in table.splitlines():
        parts = line.split('#', 1)[0].split()
        if not parts:
            continue
        nr, abi, name = parts[:3]
        entry = parts[3] if len(parts) > 3 else ''
        symbol = '__x64_' + entry if entry.startswith('sys_') else ''
        status = 'no_medido' if not a.system_map else ('simbolo_presente' if symbol in symbols else 'sin_simbolo_identificado')
        summary = subprocess.run(['whatis', '-s', '2', name], capture_output=True, text=True)
        purpose = summary.stdout.strip().replace('\n', '; ') if summary.returncode == 0 else 'pendiente: sin resumen man(2)'
        purpose_source = 'man-db seccion 2' if summary.returncode == 0 else ''
        if summary.returncode != 0 and name in SUPPLEMENT:
            purpose, path = SUPPLEMENT[name]
            git(a.kernel, 'cat-file', '-e', f'{commit}:{path}')
            purpose_source = f'{commit}:{path}'
        rows.append(dict(numero=int(nr), abi=abi, nombre=name, entrada=entry,
                         simbolo=symbol, system_map=status,
                         politica=('permitida_condicional' if name in ('mmap', 'mprotect', 'setuid', 'setgid', 'ioctl', 'fcntl', 'prctl', 'arch_prctl') else 'permitida') if abi != 'x32' and name in rules else 'denegada',
                         motivo=rules.get(name, '') if abi != 'x32' else 'ABI x32 rechazada',
                         observada='si' if name in observed and abi != 'x32' else ('no_observada' if a.trace else 'no_medido'),
                         proposito=purpose, fuente_proposito=purpose_source))
    native = [r for r in rows if r['abi'] in ('common', '64')]
    if len({r['numero'] for r in native}) != len(native):
        raise SystemExit('ROJO: numeros nativos duplicados')
    unknown_rules = set(rules) - {r['nombre'] for r in native}
    if unknown_rules:
        raise SystemExit(f'ROJO: declaraciones sin syscall nativa: {unknown_rules}')
    a.out.mkdir(parents=True, exist_ok=True)
    with (a.out / 'catalogo.csv').open('w', newline='', encoding='utf-8') as f:
        writer = csv.DictWriter(f, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)
    inputs = [a.politica, *a.trace] + ([a.system_map] if a.system_map else [])
    meta = dict(commit=commit, tabla_sha256=hashlib.sha256(table.encode()).hexdigest(),
                nativas=len(native), x32=len(rows)-len(native), declaradas=len(rules),
                resumenes_pendientes=sum(r['proposito'].startswith('pendiente:') for r in rows),
                inputs={str(f): hashlib.sha256(f.read_bytes()).hexdigest() for f in inputs},
                advertencia='Simbolo no prueba alcanzabilidad; no observado no prueba innecesario. No se conceden permisos automaticamente.')
    (a.out / 'metadatos.json').write_text(json.dumps(meta, indent=2, ensure_ascii=False) + '\n')
    header = ['/* SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note */',
              f'/* Generated from pinned kernel {commit}; do not hand-edit. */',
              '#ifndef CAJA_SYSCALLS_X86_64_H', '#define CAJA_SYSCALLS_X86_64_H',
              'static const struct { int nr; const char *name; } caja_syscall_names[] = {']
    header += [f'    {{{r["numero"]}, "{r["nombre"]}"}},' for r in native]
    header += ['};', '#endif', '']
    a.header.write_text('\n'.join(header), encoding='utf-8')
    print(f'CATALOGO: nativas={len(native)} x32={len(rows)-len(native)} declaradas={len(rules)}; resumenes pendientes={meta["resumenes_pendientes"]}')

if __name__ == '__main__':
    main()
