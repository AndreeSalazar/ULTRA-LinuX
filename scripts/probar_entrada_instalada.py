#!/usr/bin/env python3
"""Only inside the disposable QEMU compositor session, never physical devices."""
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import time

assert Path('/sys/class/dmi/id/sys_vendor').read_text().strip() == 'QEMU'
assert os.getuid() == 1000
loader=importlib.machinery.SourceFileLoader('entrada_instalada','/usr/local/bin/ultra-entrada')
spec=importlib.util.spec_from_loader(loader.name,loader);m=importlib.util.module_from_spec(spec);loader.exec_module(m)
home=Path(os.environ['HOME']);p=m.folder(home)/'entrada.json';state=Path('/tmp/entrada-banco-original.json')


def cli(action,data=None):
    r=subprocess.run(['/usr/local/bin/ultra-entrada',action],input=json.dumps(data) if data else None,
                     text=True,capture_output=True,timeout=45,check=True)
    return json.loads(r.stdout)


if len(sys.argv)>1 and sys.argv[1]=='concluir':
    original=json.loads(state.read_text());m.apply(original['actual'])
    if original['perfil'] is None:p.unlink(missing_ok=True)
    else:p.write_bytes(bytes.fromhex(original['perfil']))
    state.unlink();print('ENTRADA-BANCO-RESTAURADO',flush=True)
else:
    before=m.snapshot()
    assert not p.is_symlink()
    state.write_text(json.dumps({'actual':before,'perfil':p.read_bytes().hex() if p.exists() else None}));state.chmod(0o600)
    new={**before,'sensitivity':.35,'accel_profile':'flat','scroll_factor':1.4,
         'natural_scroll':True,'left_handed':False,'kb_layout':'us,latam',
         'kb_options':'grp:alt_shift_toggle','repeat_rate':32,'repeat_delay':450,
         'follow_mouse':2,'hide_on_key_press':True}
    result=cli('guardar',new);assert m.matches(new,result['actual'])
    assert p.stat().st_mode & 0o777 == 0o600
    m.apply(before)
    assert m.matches(new,cli('restaurar')['actual'])
    subprocess.run(['hyprctl','reload'],check=True,capture_output=True,timeout=4)
    for _ in range(25):
        if m.matches(new,m.snapshot()):break
        time.sleep(.2)
    else:raise AssertionError('El evento config.reloaded no recupero el perfil')
    for _ in range(25):
        try:
            assert cli('estado')['guardado']==new
            break
        except subprocess.CalledProcessError as error:
            if 'temporarily unavailable' not in error.stderr.lower(): raise
            time.sleep(.1)  # The config.reloaded worker may still hold its lock.
    else: raise AssertionError('El perfil siguio ocupado despues de la recarga')
    print('ENTRADA-REAL-OK: once opciones verificadas, perfil 0600, restauracion y recarga',flush=True)
