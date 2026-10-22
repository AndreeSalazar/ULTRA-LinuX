#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Validation tests for untrusted folder selections; no mounts or real game files."""
import importlib.util
from pathlib import Path
import tempfile

source = Path(__file__).resolve().parents[1]/'src/rootfs/bin/ultra-bibliotecas.py'
spec = importlib.util.spec_from_file_location('bibliotecas', source)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
count = 0


def reject(relative=None, uuid='1234-ABCD', kind='steam'):
    global count
    try:
        module.selection({'uuid': uuid, 'relative': relative, 'kind': kind})
    except ValueError:
        count += 1
        print('PASS', count, 'rechazo', repr((uuid, relative, kind))[:160])
    else:
        raise AssertionError('se acepto una seleccion insegura')


for path in ('', '/', '../juego', 'games/../Windows', 'games/./foo', '/etc',
             'games//foo', 'games\x00foo', 'games\nfoo', 'a'*4097):
    reject(path)
for uuid in ('../disco', '/dev/sda', 'UUID,upperdir=/etc', '', None):
    reject('SteamLibrary', uuid=uuid)
reject('SteamLibrary', kind='shell')
one = module.selection({'uuid':'1234-ABCD', 'relative':'Juegos, dos Ω/SteamLibrary', 'kind':'steam'})
two = module.selection({'uuid':'1234-ABCD', 'relative':'Otros', 'kind':'steam', 'id':'../../etc'})
assert len(one['id']) == 20 and one['id'].isalnum() and one['id'] != two['id']
count += 1
print('PASS', count, 'rutas con espacios, coma y Unicode no se usan como opciones de montaje')
with tempfile.TemporaryDirectory() as temp:
    path = Path(temp)/'oversized.json'
    path.write_bytes(b'x'*(module.LIMIT+1))
    try:
        module.read_json(path)
    except ValueError:
        count += 1
        print('PASS', count, 'limite de entrada antes de interpretar JSON')
    else:
        raise AssertionError('entrada sin limite')
print('RESULTADO', count)
