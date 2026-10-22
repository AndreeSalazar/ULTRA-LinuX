#!/usr/bin/env python3
"""Exercise real metadata writes in a disposable home, without Steam accounts."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import tempfile
from unittest.mock import patch

source = Path(__file__).resolve().parents[1]/'src/rootfs/bin/ultra-bibliotecas.py'
spec = importlib.util.spec_from_file_location('bibliotecas', source)
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
with tempfile.TemporaryDirectory() as temp:
    m.HOME = Path(temp)/'home'
    m.CATALOG = Path(temp)/'catalog.json'
    m.TARGETS = Path(temp)/'libraries'
    item = m.selection({'uuid':'1234-ABCD', 'relative':'Steam Ω', 'kind':'steam'})
    library = m.TARGETS/item['id']
    apps = library/'steamapps'
    (apps/'common/Juego').mkdir(parents=True)
    manifest = apps/'appmanifest_123.acf'
    manifest.write_text(m.write_vdf({'AppState':{'appid':'123','name':'Juego Ω','StateFlags':'4',
                                              'SizeOnDisk':'9000','installdir':'Juego'}}), encoding='utf-8')
    original_manifest = manifest.read_bytes()
    m.CATALOG.write_text(json.dumps({'folders':[{**item,'path':str(library),'status':'activa'}]}))
    path = m.HOME/'.local/share/Steam/steamapps/libraryfolders.vdf'
    path.parent.mkdir(parents=True)
    previous = {'libraryfolders':{'0':{'path':str(path.parent.parent),'apps':{},'unknown':'conservar'},
                                 '3':{'path':'/otra/biblioteca','apps':{'456':'800'}}}}
    path.write_text(m.write_vdf(previous), encoding='utf-8')
    original = path.read_bytes()
    with patch.object(m, 'user_config', return_value=[]), patch.object(m.subprocess, 'run', return_value=subprocess.CompletedProcess([],1)), contextlib.redirect_stdout(io.StringIO()) as log:
        m.register_steam()
        result = m.read_vdf(path)['libraryfolders']
        assert result['0'] == previous['libraryfolders']['0'] and result['3'] == previous['libraryfolders']['3']
        assert result['4']['path'] == str(library) and result['4']['apps'] == {'123':'9000'}
        assert path.with_name(path.name+'.ultra-anterior').read_bytes() == original
        canonical = m.HOME/'.local/share/Steam/config/libraryfolders.vdf'
        assert m.read_vdf(canonical) == m.read_vdf(path)
        assert manifest.read_bytes() == original_manifest
        first = path.read_bytes()
        m.register_steam()
        assert path.read_bytes() == first and len(m.read_vdf(path)['libraryfolders']) == 3
    assert 'Juego Ω' in log.getvalue()
    print('PASS: registra biblioteca y juegos en config y steamapps; conserva originales, otros campos y respaldo; es idempotente')
    stale = m.read_vdf(path)
    stale['libraryfolders']['4']['apps'] = {}
    stale['libraryfolders']['4']['extra'] = 'conservar'
    path.write_text(m.write_vdf(stale), encoding='utf-8')
    with patch.object(m,'user_config',return_value=[]), patch.object(m.subprocess,'run',return_value=subprocess.CompletedProcess([],1)), contextlib.redirect_stdout(io.StringIO()):
        m.register_steam()
    fixed = m.read_vdf(path)['libraryfolders']['4']
    assert fixed['apps'] == {'123':'9000'} and fixed['extra'] == 'conservar'
    assert canonical.read_bytes() == first
    first = path.read_bytes()
    print('PASS: completa indice vacio de biblioteca ya registrada sin perder campos ni duplicarla')
    with patch.object(m,'user_config',return_value=[]), patch.object(m.subprocess,'run',return_value=subprocess.CompletedProcess([],0)):
        m.register_steam()
    assert path.read_bytes() == first
    print('PASS: Steam abierto impide cambios')
    for bad in ('"libraryfolders" {', '"a" "x" "a" "y"', '"a" { "b" "c" }}', '"a" unquoted', 'x'*(m.LIMIT+1)):
        path.write_text(bad)
        try:
            m.read_vdf(path)
        except ValueError:
            pass
        else:
            raise AssertionError('VDF invalido aceptado')
    print('PASS: VDF incompleto, ambiguo, desconocido y excesivo rechazado')
print('RESULTADO: pruebas de registro Steam correctas; no acredita reconocimiento por un cliente autenticado')
