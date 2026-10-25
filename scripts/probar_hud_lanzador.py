#!/usr/bin/env python3
"""Real HUD adapter: launcher blacklist must not disable game metrics or limits."""
import os
from pathlib import Path
import subprocess
import tempfile

root=Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory() as tmp:
    home=Path(tmp); bin_=home/'bin'; bin_.mkdir()
    config=home/'.config/ultra/MangoHud.conf'; config.parent.mkdir(parents=True)
    config.write_text('fps\nfps_limit=60\nblacklist=steam,custom-helper\n')
    record=home/'environment'
    for name,body in [('id','echo 1000'),('heronite','printf "%s\\n" "$MANGOHUD" "$MANGOHUD_CONFIGFILE" > "$RECORD"'),
                      ('game','printf "%s\\n" "$MANGOHUD" > "$RECORD"')]:
        path=bin_/name; path.write_text('#!/usr/bin/bash\n'+body+'\n'); path.chmod(0o755)
    script=home/'hud'
    script.write_text((root/'src/rootfs/bin/ultra-hud').read_text().replace('/usr/bin/heronite-rs',str(bin_/'heronite')))
    env=dict(os.environ,HOME=str(home),PATH=str(bin_)+':'+os.environ['PATH'],RECORD=str(record))
    env.pop('ULTRA_DIAGNOSTICO_HUD',None)
    subprocess.run(['/usr/bin/bash',str(script),'ejecutar',str(bin_/'heronite')],env=env,check=True)
    text=config.read_text()
    assert 'fps_limit=60' in text and 'blacklist=steam,custom-helper,heronite-rs,heronite-epic-login' in text
    assert record.read_text().splitlines()==['1',str(config)]
    before=config.read_bytes(),config.stat().st_mtime_ns
    subprocess.run(['/usr/bin/bash',str(script),'ejecutar',str(bin_/'heronite')],env=env,check=True)
    assert before==(config.read_bytes(),config.stat().st_mtime_ns)
    subprocess.run(['/usr/bin/bash',str(script),'ejecutar',str(bin_/'game')],env=env,check=True)
    assert record.read_text().strip()=='1'
print('VERDE: launcher excluido, HUD de juegos y limite personal conservados, actualizacion idempotente')
