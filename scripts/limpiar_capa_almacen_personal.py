#!/usr/bin/env python3
"""One-off offline cleanup of the diagnosed obsolete layer, with a full backup.

No physical disks, NIC, account clients, cache or active game removal.
Requires the verified backup manifest produced before this maintenance.
"""
import base64
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import shutil
import socket
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
IMAGE = Path('C:/Espacio para Linux/ULTRA-juegos.ext4')
WORK = ROOT/'build/respaldo-limpieza-20261023'
OLD = '5f9b020e65d5dfb89025.anterior-1792755502206347986'

GUEST = r'''
import hashlib,json,os,shutil,stat,subprocess
from pathlib import Path
def run(*args):
    return subprocess.run(args,check=True,text=True,capture_output=True).stdout.strip()
assert run('blockdev','--getsize64','/dev/vdb') == '103079215104'
assert run('blkid','-p','-s','UUID','-o','value','/dev/vdb') == 'ecbc9f71-f43c-4d20-9b71-a7929f264ba8'
assert not subprocess.run(['findmnt','-rn','-S','/dev/vdb'],capture_output=True).stdout
run('e2fsck','-f','-n','/dev/vdb')
root=Path('/mnt/limpieza')
root.mkdir(exist_ok=True)
run('mount','-t','ext4','-o','rw,nosuid,nodev,noexec','/dev/vdb',str(root))
try:
    parent=root/'.ultra-bibliotecas'
    old=parent/'5f9b020e65d5dfb89025.anterior-1792755502206347986'
    active=parent/'5f9b020e65d5dfb89025'
    for p in (root,parent,old,active):
        s=p.lstat()
        assert stat.S_ISDIR(s.st_mode) and s.st_uid == 0
        assert p.resolve() == p
    assert old.parent == parent and old.name.endswith('.anterior-1792755502206347986')
    def identity(p):
        q=p/'source.json'
        assert not q.is_symlink() and q.stat().st_size < 262144
        return json.loads(q.read_text())
    a,b=identity(old),identity(active)
    assert {k:v for k,v in a.items() if k!='version'} == {k:v for k,v in b.items() if k!='version'}
    # Separately authorized: remove the Linux Cyberpunk layer and its own
    # Wine prefix (including its saves). Never visit Windows C:/D: originals.
    cyberpunk_removed=[]
    home=root/'usuarios/ultra'
    # Confirm the actual boot image maps this fixed upper/work pair only to
    # Cyberpunk. Reject extra children; never remove the broad overlay parent.
    preparer=Path('/usr/local/libexec/ultra-preparar-juegos').read_text()
    assert 'game="$game_volume/Cyberpunk 2077"' in preparer
    assert 'upperdir=/juegos/overlay/upper,workdir=/juegos/overlay/work" /juegos/Cyberpunk2077' in preparer
    overlay=root/'overlay'
    if overlay.exists():
        assert overlay.resolve() == overlay and overlay.is_dir()
        assert {p.name for p in overlay.iterdir()} <= {'upper','work'}
    targets=[overlay/'upper',overlay/'work',home/'Juegos/prefijos/cyberpunk2077',
             home/'.local/share/Steam/steamapps/compatdata/1091500']
    userdata=home/'.local/share/Steam/userdata'
    if userdata.is_dir() and not userdata.is_symlink():
        for account in list(userdata.iterdir())[:128]:
            if account.name.isdecimal() and account.is_dir() and not account.is_symlink():
                targets.append(account/'1091500')
    for target in targets:
        assert target.is_relative_to(root) and target.resolve() == target
        if target.exists():
            assert target.is_dir() and not target.is_symlink()
            assert not subprocess.run(['findmnt','-rn','-R',str(target)],capture_output=True).stdout
            shutil.rmtree(target)
            cyberpunk_removed.append(str(target.relative_to(root)))
    def fingerprint(p):
        h=hashlib.sha256(); count=0
        # Metadata only: never read session/account contents. Do not follow links.
        for base,dirs,files in os.walk(p,followlinks=False):
            for name in sorted(dirs+files):
                q=Path(base)/name; s=q.lstat(); count+=1
                row=(str(q.relative_to(p)),s.st_mode,s.st_uid,s.st_gid,s.st_size,s.st_mtime_ns)
                h.update(repr(row).encode())
        return h.hexdigest(),count
    before=[fingerprint(p) for p in (root/'usuarios',active)]
    initial=os.statvfs(root)
    assert not subprocess.run(['findmnt','-rn','-R',str(old)],capture_output=True).stdout
    shutil.rmtree(old)
    os.sync()
    assert before == [fingerprint(p) for p in (root/'usuarios',active)]
    after=os.statvfs(root)
    result={'capa_antigua_retirada':old.name,'home_y_capa_activa_metadatos_conservados':True,
            'cyberpunk_linux_retirado':cyberpunk_removed,
            'liberado_bytes':(after.f_bfree-initial.f_bfree)*after.f_frsize,
            'disponible_usuario_bytes':after.f_bavail*after.f_frsize}
    assert result['liberado_bytes'] > 55*1024**3
    print('LIMPIEZA_JSON '+json.dumps(result),flush=True)
finally:
    run('umount',str(root))
run('e2fsck','-f','-n','/dev/vdb')
print('LIMPIEZA_FSCK_OK',flush=True)
'''


def main():
    record=json.loads((WORK/'respaldo.json').read_text())
    backup=Path(record['copia'])
    assert IMAGE.resolve() == Path(record['origen']).resolve()
    assert backup.resolve().parent == WORK.resolve() and not backup.is_symlink()
    assert IMAGE.stat().st_size == backup.stat().st_size == record['bytes']
    assert IMAGE.stat().st_mtime_ns == record['mtime_ns']
    # Reverify the source immediately before modifying it. The backup was read
    # back and verified independently when the manifest was published.
    with IMAGE.open('rb') as stream:
        assert hashlib.file_digest(stream,'sha256').hexdigest() == record['sha256']
    sys.argv=[sys.argv[0],'generico','--bmo']
    spec=importlib.util.spec_from_file_location('base',ROOT/'scripts/probar_arranque.py')
    base=importlib.util.module_from_spec(spec); spec.loader.exec_module(base)
    base.check_kernel()
    variables=WORK/'vars.fd'
    shutil.copyfile(base.QEMU_SHARE/'edk2-i386-vars.fd',variables)
    args=[str(base.QEMU),'-name','ULTRA limpieza offline respaldada','-machine','pc',
          '-accel','whpx','-cpu','host','-smp','2','-m','2048',
          '-drive',f'if=pflash,format=raw,readonly=on,file={base.QEMU_SHARE/"edk2-x86_64-code.fd"}',
          '-drive',f'if=pflash,format=raw,file={variables}',
          '-drive',f'file={base.IMG},format=raw,if=virtio,snapshot=on',
          '-drive',f'file={IMAGE},format=raw,if=virtio',
          '-nic','none','-display','none','-monitor','none',
          '-serial','tcp:127.0.0.1:15965,server=on,wait=off','-no-reboot']
    log=''; connection=None
    with (WORK/'qemu-limpieza.txt').open('w') as errors:
        process=subprocess.Popen(args,stdout=errors,stderr=errors,creationflags=subprocess.CREATE_NO_WINDOW)
        try:
            for _ in range(100):
                try:
                    connection=socket.create_connection(('127.0.0.1',15965),timeout=1); break
                except OSError: time.sleep(.2)
            assert connection
            log+=base.read_until(connection,'ultra:',90)
            def command(text,marker,timeout=60):
                nonlocal log
                connection.sendall((text+'\n').encode())
                output=base.read_until(connection,marker,timeout); log+=output
                return output
            command(': > /arch/tmp/limpieza.b64; echo CARGA-$((40+2))','CARGA-42')
            data=base64.b64encode(GUEST.encode()).decode()
            for offset in range(0,len(data),192):
                command(f"printf '%s' '{data[offset:offset+192]}' >> /arch/tmp/limpieza.b64; echo CARGA-$((40+2))",'CARGA-42')
            command('busybox base64 -d /arch/tmp/limpieza.b64 > /arch/tmp/limpieza.py; echo CARGA-$((40+2))','CARGA-42')
            output=command('ultra-entrar /usr/bin/python /tmp/limpieza.py; echo RESULTADO-$?; echo FINAL-$((40+2))','FINAL-42',600)
            assert re.search(r'(?m)^RESULTADO-0\r?$',output) and 'LIMPIEZA_FSCK_OK' in output,output
            result=json.loads(re.search(r'(?m)^LIMPIEZA_JSON (.+)\r?$',output)[1])
            (WORK/'limpieza.json').write_text(json.dumps(result,indent=2)+'\n')
            print(json.dumps(result,indent=2),flush=True)
            connection.sendall(b'poweroff\n'); log+=base.read_until(connection,'APAGADO',40)
            process.wait(timeout=20)
        finally:
            if connection: connection.close()
            if process.poll() is None: process.kill(); process.wait(timeout=10)
            (WORK/'consola-limpieza.txt').write_text(log,encoding='utf-8')


if __name__=='__main__': main()
