#!/usr/bin/python
"""Disposable QEMU probe only: fake SteamAppId and actual software OpenGL CSV."""
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

loader=importlib.machinery.SourceFileLoader('diag','/usr/local/bin/ultra-diagnostico-juego')
spec=importlib.util.spec_from_loader(loader.name,loader)
diag=importlib.util.module_from_spec(spec); loader.exec_module(diag)

assert os.geteuid()==1000
status=Path('/proc/self/status').read_text()
assert 'NoNewPrivs:\t1' in status
for key in ('CapEff','CapBnd','CapPrm','CapInh','CapAmb'):
    assert int(next(line.split()[1] for line in status.splitlines() if line.startswith(key+':')),16)==0

if sys.argv[1]=='resources':
    kernel=json.loads(Path('/run/ultra-kernel/eventos.json').read_text())
    assert kernel['status']=='escuchando'
    provider=Path('/proc')/str(kernel['pid'])
    priv=(provider/'status').read_text()
    assert provider.stat().st_uid==0
    for key in ('CapEff','CapBnd'):
        assert int(next(line.split()[1] for line in priv.splitlines() if line.startswith(key+':')),16)==1<<34
    print('DIAG-KERNEL-SYSLOG-42',flush=True)
    opts=subprocess.check_output(['/usr/bin/findmnt','-n','-o','FSTYPE,OPTIONS','--mountpoint','/run/ultra-cuadros'],text=True)
    assert 'tmpfs' in opts and all(v in opts for v in ('nosuid','nodev','noexec','size=65536k'))
    print('DIAG-RAM-64M-42',flush=True)
    worker=subprocess.Popen(['/usr/bin/python','-c','import time; end=time.monotonic()+7\nwhile time.monotonic()<end: pass'],
                            env=dict(os.environ,SteamAppId='230410',PASSWORD='SECRET-NEVER-EXPORT'))
    try:
        diag.record(diag.load_live(),duration=4)
        session,data=diag.latest_session()
        rows=[json.loads(row) for row in (session/'recursos.jsonl').read_text().splitlines()]
        assert len(rows)>=2 and not data['running']
        assert any((r.get('game') or {}).get('cpu_pct',0)>0 for r in rows[1:] if (r.get('game') or {}).get('cpu_pct') is not None)
        assert any(r['threads']['observed']>0 for r in rows)
        assert 'SECRET-NEVER-EXPORT' not in json.dumps(rows)
        assert all('network_global' in r and 'disks_global' in r and 'pressure_global' in r for r in rows)
        print('DIAG-MEDICION '+json.dumps(dict(scope='proceso ficticio en QEMU, no Warframe',
              samples=len(rows),cpu_peak_pct=max((r.get('game') or {}).get('cpu_pct') or 0 for r in rows),
              threads_peak=max(r['threads']['observed'] for r in rows))),flush=True)
        print('DIAG-RECURSOS-UID1000-42',flush=True)
    finally:
        worker.terminate(); worker.wait(timeout=3)
elif sys.argv[1]=='frames':
    files=[p for p in diag.FRAMES.glob('glxgears*.csv') if not p.name.endswith('_summary.csv')]
    assert files,'CSV OpenGL no encontrado'
    actual=diag.frame_summary(files[-1])
    assert actual['rows']>10 and actual['frametime_ms']['maximum']>0
    print('DIAG-CUADROS '+json.dumps(dict(scope='glxgears software, no RTX ni Warframe',
          rows=actual['rows'],frametime_ms=actual['frametime_ms'])),flush=True)
    print('DIAG-CUADROS-OPENGL-42',flush=True)
    # Rename only the software benchmark fixture to exercise the production
    # collector allowlist. These frames are not Warframe or RTX measurements.
    shutil.copyfile(files[-1],diag.FRAMES/'Warframe-fixture.csv')
    session,data=diag.latest_session(); frames=diag.collect_frames(session)
    assert frames['files'] and frames['files'][0]['rows']>10
    diag.atomic(session/'resumen.json',dict(data,frames=frames))
    print('DIAG-CSV-ARCHIVADO-42',flush=True)
else:
    raise ValueError('fase desconocida')
