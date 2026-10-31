#!/usr/bin/env python3
"""Real Wayland clipboard, exclusively on a disposable QEMU desktop."""
import io
import json
import os
from pathlib import Path
import subprocess
import time
from PIL import Image

assert Path('/sys/class/dmi/id/sys_vendor').read_text().strip()=='QEMU'
assert os.getuid()==1000
BIN='/usr/local/bin/ultra-portapapeles'


def cli(action, data=None):
    r=subprocess.run([BIN,action],input=json.dumps(data).encode() if data is not None else None,
                     capture_output=True,timeout=15,check=True)
    return json.loads(r.stdout)


def wait_for(predicate):
    for _ in range(40):
        result=cli('listar')
        if predicate(result):return result
        time.sleep(.15)
    raise AssertionError('El observador no registro la copia Wayland')


def copy(data,mime):
    subprocess.run(['wl-copy','--type',mime],input=data,stdout=subprocess.DEVNULL,
                   stderr=subprocess.DEVNULL,check=True,timeout=5)


cli('pausa',{'paused':False});cli('borrar')
text=b'ULTRA: pin persistente de prueba'
copy(text,'text/plain;charset=utf-8')
result=wait_for(lambda d:any(r['preview']==text.decode() for r in d['entries']))
entry=next(r['id'] for r in result['entries'] if r['preview']==text.decode())
cli('pin',{'id':entry})
out=io.BytesIO();Image.new('RGBA',(72,48),(0,210,240,255)).save(out,format='PNG');png=out.getvalue()
copy(png,'image/png')
result=wait_for(lambda d:any(r['mime']=='image/png' and r['bytes']==len(png) for r in d['entries']))
picture=next(r['id'] for r in result['entries'] if r['mime']=='image/png' and r['bytes']==len(png))
cli('pin',{'id':picture});cli('borrar')
result=cli('listar');assert result['pins']==2 and result['total']==2
cli('copiar',{'id':entry})
assert subprocess.check_output(['wl-paste','--no-newline'],timeout=5)==text
cli('copiar',{'id':picture})
assert subprocess.check_output(['wl-paste','--type','image/png'],timeout=5)==png
cli('pausa',{'paused':True})
copy(b'ULTRA-no-guardar-pausa','text/plain')
time.sleep(.5)
assert all('no-guardar-pausa' not in r['preview'] for r in cli('listar')['entries'])
cli('pausa',{'paused':False})
cli('presupuesto',{'mib':256})
assert cli('listar')['pins']==2
path=Path(os.environ['HOME'])/'.local/share/ultra/portapapeles/historial.sqlite3'
assert path.stat().st_mode & 0o777==0o600
print('PORTAPAPELES-REAL-OK: observador Wayland, texto/PNG exactos, pines tras limpiar y reabrir, pausa, permisos 0600',flush=True)
