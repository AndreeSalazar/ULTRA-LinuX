#!/usr/bin/env python3
"""Real compositor captures. Only execute inside the disposable QEMU desktop."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
from PIL import Image


assert os.getuid()==1000 and os.environ.get('WAYLAND_DISPLAY')
errors=json.loads(subprocess.check_output(['hyprctl','-j','configerrors'],text=True,timeout=5))
assert isinstance(errors,list) and all(isinstance(e,str) and not e.strip() for e in errors),repr(errors)
with tempfile.TemporaryDirectory(prefix='captura-instalada-') as tmp:
    env=dict(os.environ,HOME=tmp)
    folder=Path(tmp)/'Imágenes/Capturas'
    def run(mode):
        result=subprocess.run(['/usr/local/bin/ultra-captura',mode],env=env,capture_output=True,text=True,timeout=30)
        assert result.returncode==0,(mode,result.stderr)
        path=Path(result.stdout.strip())
        with Image.open(path) as image:
            assert image.format=='PNG' and min(image.size)>0
            image.verify()
        assert path.stat().st_mode & 0o777==0o600
        return path
    first=run('pantalla');second=run('ventana')
    assert first!=second and first.is_file()
    with Image.open(first).convert('RGB') as image:
        assert sum(g>r+40 and b>r+40 for r,g,b in image.resize((160,100)).getdata())>20,'captura sin el acento cyan del escritorio de prueba'
    pasted=subprocess.run(['wl-paste','--type','image/png'],capture_output=True,timeout=5)
    assert pasted.returncode==0 and pasted.stdout.startswith(b'\x89PNG\r\n\x1a\n')
    region=subprocess.Popen(['/usr/local/bin/ultra-captura','region'],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
    print('CAPTURA-ESPERA-REGION',flush=True)
    out,err=region.communicate(timeout=30)
    assert region.returncode==0,err
    with Image.open(out.strip()) as image:
        assert image.width>10 and image.height>10;image.verify()
    previous=set(folder.glob('*.png'))
    cancel=subprocess.Popen(['/usr/local/bin/ultra-captura','region'],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
    print('CAPTURA-ESPERA-CANCELAR',flush=True)
    out,err=cancel.communicate(timeout=30)
    assert cancel.returncode==0 and not out and previous==set(folder.glob('*.png')),(out,err)
    print('CAPTURA-REAL-OK: PNG pantalla/ventana/region, clipboard y Esc; sin datos personales',flush=True)
