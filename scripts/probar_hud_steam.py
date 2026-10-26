#!/usr/bin/env python3
"""Steam boundary after Heronite's intentional environment sanitization."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory() as tmp:
    home = Path(tmp)
    tools = home/'bin'
    tools.mkdir()
    config = home/'.config/ultra/MangoHud.conf'
    config.parent.mkdir(parents=True)
    config.write_text('fps\nframetime\nfps_limit=60\nblacklist=steam,steamwebhelper\n')
    (tools/'id').write_text('#!/bin/sh\necho 1000\n')
    (tools/'id').chmod(0o755)
    target = tools/'native-steam'
    target.write_text('#!/usr/bin/python3\nimport os,sys,json\nfrom pathlib import Path\n'
                      'Path(os.environ["RECORD"]).write_text(json.dumps({'
                      '"hud":os.environ.get("MANGOHUD"),'
                      '"config":os.environ.get("MANGOHUD_CONFIGFILE"),"args":sys.argv[1:]}))\n')
    target.chmod(0o755)
    hud = tools/'ultra-hud'
    hud.write_text((root/'src/rootfs/bin/ultra-hud').read_text())
    hud.chmod(0o755)
    wrapper = tools/'steam'
    source = (root/'src/rootfs/bin/steam').read_text()
    assert '/usr/bin/steam "$@"' in source
    wrapper.write_text(source.replace('/usr/local/bin/ultra-hud', str(hud)).replace('/usr/bin/steam', str(target)))
    wrapper.chmod(0o755)
    env = {k:v for k,v in os.environ.items() if not k.startswith('MANGOHUD')}
    env.update(HOME=str(home), PATH=str(tools)+':/usr/bin', RECORD=str(home/'result'))
    env.pop('ULTRA_DIAGNOSTICO_HUD', None)
    args = ['-applaunch', '230410', 'two words', '$(touch never-execute)']
    before = config.read_bytes()
    subprocess.run([str(wrapper), *args], env=env, check=True)
    enabled = json.loads((home/'result').read_text())
    assert enabled == {'hud':'1', 'config':str(config), 'args':args}, enabled
    assert before == config.read_bytes()
    (config.parent/'hud-desactivado').touch()
    subprocess.run([str(wrapper), *args], env=env, check=True)
    disabled = json.loads((home/'result').read_text())
    assert disabled == {'hud':'0', 'config':None, 'args':args}, disabled
    assert not (home/'never-execute').exists()
print('VERDE: Steam restaura HUD tras entorno saneado; desactivar, argumentos y limites conservados')
