#!/usr/bin/python
# SPDX-License-Identifier: GPL-2.0-only
"""QEMU integration: theme assets, real read-only hardware and migration backup."""
import json
import os
from pathlib import Path
import struct
import subprocess
import tempfile

assert os.geteuid()==1000
theme=Path('/usr/share/icons/ULTRA-Nucleo')
for name in ('left_ptr','hand2','xterm','crosshair','watch','not-allowed','move',
             'sb_h_double_arrow','sb_v_double_arrow','size_fdiag','size_bdiag'):
    raw=(theme/'cursors'/name).read_bytes()
    assert raw[:4]==b'Xcur'
    magic,header,version,count=struct.unpack_from('<4I',raw)
    assert count==5
    for i in range(count):
        kind,size,offset=struct.unpack_from('<3I',raw,header+12*i)
        assert kind==0xfffd0002 and size in (24,28,32,48,64)
        _,_,_,_,w,h,x,y,delay=struct.unpack_from('<9I',raw,offset)
        assert w==h==size and 0<=x<w and 0<=y<h
assert (theme/'cursors/pointer').resolve()==theme/'cursors/hand2'
print('NUCLEO-CURSORES-OK',flush=True)
for name in ('folder','computer','utilities-terminal','preferences-system','applications-games'):
    assert (theme/'scalable'/(name+'.svg')).read_text().startswith('<svg')
assert 'Inherits=Papirus-Dark,Adwaita' in (theme/'index.theme').read_text()
print('NUCLEO-ICONOS-OK',flush=True)
data=json.loads(subprocess.check_output(['/usr/local/bin/ultra-equipo','json'],text=True))
assert data['threads']>0 and data['ram_total_GiB']>0
assert all(key in data for key in ('cpu','usb','disks','sensors','nvidia','linux_free_GiB'))
bar=json.loads(subprocess.check_output(['/usr/local/bin/ultra-equipo','barra'],text=True))
assert 'PC' in bar['text'] and 'RAM:' in bar['tooltip']
print('NUCLEO-EQUIPO-OK',flush=True)
settings=Path.home()/'.config/gtk-3.0/settings.ini'
assert 'gtk-cursor-theme-name=ULTRA-Nucleo' in settings.read_text()
assert 'gtk-icon-theme-name=ULTRA-Nucleo' in settings.read_text()
print('NUCLEO-SESION-OK',flush=True)
env=dict(os.environ,XDG_RUNTIME_DIR='/run/user/1000')
result=subprocess.check_output(['/usr/bin/hyprctl','-i','0','setcursor','ULTRA-Nucleo','28'],env=env,text=True)
assert result.strip()=='ok'
print('NUCLEO-CURSOR-APLICADO',flush=True)
with tempfile.TemporaryDirectory(prefix='nucleo-qa-') as temp:
    home=Path(temp); config=home/'.config/foot'; config.mkdir(parents=True)
    foot=config/'foot.ini'; foot.write_text('PERSONAL-ANTERIOR\n')
    env=dict(os.environ,HOME=temp)
    subprocess.run(['/usr/local/bin/ultra-tema'],env=env,check=True)
    backup=home/'.config/ultra/tema-respaldo-neon-1/foot--foot.ini.bak'
    assert backup.read_text()=='PERSONAL-ANTERIOR\n'
    assert foot.read_bytes()==Path('/etc/ultra/foot.ini').read_bytes()
    foot.write_text('PERSONAL-POSTERIOR\n')
    subprocess.run(['/usr/local/bin/ultra-tema'],env=env,check=True)
    assert foot.read_text()=='PERSONAL-POSTERIOR\n'
    assert backup.read_text()=='PERSONAL-ANTERIOR\n'
print('NUCLEO-RESPALDO-OK',flush=True)
