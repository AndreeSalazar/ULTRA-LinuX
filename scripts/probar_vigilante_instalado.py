#!/usr/bin/env python3
"""Installed VIGILANTE, own synthetic process only: no real game or credentials."""
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time


def load(name):
    loader=importlib.machinery.SourceFileLoader(name, '/usr/local/bin/'+name)
    spec=importlib.util.spec_from_loader(loader.name,loader)
    module=importlib.util.module_from_spec(spec);loader.exec_module(module)
    return module


if '--child' not in sys.argv:
    env=dict(os.environ,SteamAppId='424242',MANGOHUD='1',VIGILANTE_SECRET='DO-NOT-EXPORT')
    subprocess.run([sys.executable,__file__,'--child'],env=env,check=True,timeout=20)
else:
    assert os.getuid()==1000
    app=load('ultra-juegos-vivo'); guard=app.vigilante
    one=app.sample(measure_pss=False);time.sleep(.1);two=app.sample(one,measure_pss=False)
    row=next(p for p in two['vigilante']['processes'] if p['pid']==os.getpid())
    assert row['game_id']=='steam:424242' and row['hud']['requested'] is True
    assert row['hud']['mapped'] is False and row['deltas']['major_faults'] is not None
    assert two['vigilante']['automatic_tuning'] is False and two['vigilante']['fps'] is None
    assert 'DO-NOT-EXPORT' not in json.dumps(two)
    with tempfile.TemporaryDirectory(prefix='vigilante-instalado-') as tmp:
        p=Path(tmp)/'.cache/ultra/vigilante.json';p.parent.mkdir(parents=True)
        p.write_text(json.dumps(guard.history_document([two['vigilante']])))
        out=subprocess.check_output(['/usr/local/bin/ultra-vigilante','informe','0'],env=dict(os.environ,HOME=tmp),text=True,timeout=3)
        assert json.loads(out)['samples'][0]['role']=='VIGILANTE'
        assert 'DO-NOT-EXPORT' not in out
        p.write_text(json.dumps(guard.history_document([dict(two['vigilante'],monotonic_s=0)])))
        stale=subprocess.run(['/usr/local/bin/ultra-vigilante','informe','1'],env=dict(os.environ,HOME=tmp),capture_output=True,timeout=3)
        assert stale.returncode!=0 and not stale.stdout
    print('VIGILANTE-INSTALADO-OK: UID1000, deltas reales, HUD observado, privacidad y sesion anterior rechazada')
