#!/usr/bin/env python3
"""Counter export: regular bounded JSON only, no following links or blocking FIFO."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

helper=Path(__file__).resolve().parents[1]/'src/rootfs/bin/ultra-puntero-red'
assert os.getuid()==1000
with tempfile.TemporaryDirectory() as tmp:
    root=Path(tmp); state=root/'ultra-puntero-red.json'
    env=dict(os.environ,XDG_RUNTIME_DIR=str(root))
    def run():return subprocess.run([sys.executable,str(helper),'estado'],env=env,capture_output=True,text=True,timeout=3)
    state.write_text(json.dumps({'reparaciones_transparentes':2,'monotonic_s':1.5,'unrelated':'never-export-this'}))
    result=run();assert result.returncode==0,result.stderr
    assert json.loads(result.stdout)=={'reparaciones_transparentes':2,'monotonic_s':1.5}
    state.unlink();secret=root/'other';secret.write_text('never-export-this');state.symlink_to(secret)
    result=run();assert result.returncode!=0 and not result.stdout
    state.unlink();os.mkfifo(state)
    result=run();assert result.returncode!=0 and not result.stdout
    state.unlink();state.write_text('{"reparaciones_transparentes":-1,"monotonic_s":0}')
    result=run();assert result.returncode!=0 and not result.stdout
print('VERDE: exportacion de puntero filtra campos, rechaza enlaces/FIFO y contadores invalidos')
