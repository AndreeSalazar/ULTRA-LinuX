#!/usr/bin/env python3
"""Native ext4 storage tests: disposable QEMU, no NIC or personal devices."""
import base64,importlib.util,pathlib,shutil,socket,subprocess,sys,time
ROOT=pathlib.Path(__file__).resolve().parents[1]
GUEST=r'''set -euo pipefail
cp /etc/ultra/juegos.conf /tmp/juegos-original.conf
cp /tmp/native-config /etc/ultra/juegos.conf
if bash /tmp/native-helper; then exit 10; fi
! mountpoint -q /juegos
echo VERDE-NATIVA-AUSENTE-SIN-FALLBACK
truncate -s 96M /tmp/native.ext4
mkfs.ext4 -q -F -U 19a78f63-4252-45e8-b032-5b6e53f67af5 /tmp/native.ext4
loop=$(losetup --find --show /tmp/native.ext4)
bash /tmp/native-helper
[[ $(findmnt -n -o UUID --mountpoint /juegos) == 19a78f63-4252-45e8-b032-5b6e53f67af5 ]]
[[ $(findmnt -n -o FSTYPE --mountpoint /juegos) == ext4 ]]
echo VERDE-NATIVA-MONTAJE-UUID
setpriv --reuid=1000 --regid=1000 --clear-groups --no-new-privs --bounding-set=-all bash -c 'echo persistente > /juegos/usuarios/ultra/prueba'
echo VERDE-NATIVA-HOME-UID1000
bash /tmp/native-helper
echo VERDE-NATIVA-IDEMPOTENTE
umount /juegos
mount "$loop" /juegos
[[ $(cat /juegos/usuarios/ultra/prueba) == persistente ]]
python - <<'FIXTURE'
import json,os
from pathlib import Path
parent=Path('/juegos/.ultra-cyberpunk-original');parent.mkdir(mode=0o700)
base=parent/'Cyberpunk2077';(base/'bin/x64').mkdir(parents=True)
(base/'bin/x64/Cyberpunk2077.exe').write_bytes(b'MZ-fixture')
(base/'bin/x64/fixture.dat').write_text('original')
(base/'goggame-1423049311.info').write_text(json.dumps(dict(gameId='1423049311',rootGameId='1423049311',buildId='777')))
for directory,dirs,files in os.walk(base):
    os.chown(directory,1000,1000)
    for name in files:os.chown(Path(directory)/name,1000,1000)
receipt=parent/'verificado.json';receipt.write_text(json.dumps(dict(schema=1,app='1423049311',verified=True,storage_uuid='19a78f63-4252-45e8-b032-5b6e53f67af5',build_id='777',sha256_manifest='a'*64)))
receipt.chmod(0o600)
FIXTURE
umount /juegos
echo VERDE-NATIVA-PERSISTENCIA
bash /tmp/native-helper
[[ $(findmnt -n -o FSTYPE --mountpoint /juegos/Cyberpunk2077) == overlay ]]
echo VERDE-CYBERPUNK-SIN-DISCO-PERSONAL
setpriv --reuid=1000 --regid=1000 --clear-groups --no-new-privs --bounding-set=-all bash -c 'printf cambiado > /juegos/Cyberpunk2077/bin/x64/fixture.dat; test ! -r /juegos/.ultra-cyberpunk-original/verificado.json'
[[ $(cat /juegos/.ultra-cyberpunk-original/Cyberpunk2077/bin/x64/fixture.dat) == original ]]
[[ $(cat /juegos/Cyberpunk2077/bin/x64/fixture.dat) == cambiado ]]
echo VERDE-CYBERPUNK-ESCRITURAS-SEPARADAS
umount /juegos/Cyberpunk2077
chmod 0666 /juegos/.ultra-cyberpunk-original/verificado.json
umount /juegos
bash /tmp/native-helper
! mountpoint -q /juegos/Cyberpunk2077
echo VERDE-CYBERPUNK-RECIBO-NO-PROTEGIDO-RECHAZADO
umount /juegos
cp /tmp/native.ext4 /tmp/duplicada.ext4
other=$(losetup --find --show /tmp/duplicada.ext4)
if bash /tmp/native-helper; then exit 11; fi
! mountpoint -q /juegos
echo VERDE-NATIVA-UUID-DUPLICADO-RECHAZADO
losetup -d "$other" "$loop"
cp /tmp/juegos-original.conf /etc/ultra/juegos.conf
echo NATIVA-FIN-$((40+2))
'''
def main():
    sys.argv=[sys.argv[0],'generico','--bmo']
    s=importlib.util.spec_from_file_location('base',ROOT/'scripts/probar_arranque.py');b=importlib.util.module_from_spec(s);s.loader.exec_module(b);b.check_kernel()
    work=ROOT/'build/prueba-particion-juegos';work.mkdir(exist_ok=True)
    variables=work/'vars.fd';shutil.copyfile(b.QEMU_SHARE/'edk2-i386-vars.fd',variables)
    args=[str(b.QEMU),'-name','ULTRA native storage fixtures only','-machine','pc','-accel','whpx','-cpu','host','-m','2048',
      '-drive',f'if=pflash,format=raw,readonly=on,file={b.QEMU_SHARE/"edk2-x86_64-code.fd"}',
      '-drive',f'if=pflash,format=raw,file={variables}','-drive',f'file={b.IMG},format=raw,if=virtio,snapshot=on',
      '-nic','none','-display','none','-monitor','none','-serial','tcp:127.0.0.1:15967,server=on,wait=off','-no-reboot']
    conn=None
    with (work/'qemu.txt').open('w') as error:
        process=subprocess.Popen(args,stdout=error,stderr=error,creationflags=subprocess.CREATE_NO_WINDOW)
        try:
            for _ in range(100):
                try:conn=socket.create_connection(('127.0.0.1',15967),timeout=1);break
                except OSError:time.sleep(.2)
            if conn is None:raise RuntimeError('Sin consola QEMU')
            b.read_until(conn,'ultra:',90)
            def command(text,marker,timeout=30):
                conn.sendall((text+'\n').encode());return b.read_until(conn,marker,timeout)
            for name,raw in [('native-helper',(ROOT/'src/rootfs/bin/ultra-preparar-juegos').read_bytes()),('native-config',(ROOT/'src/rootfs/desktop/juegos.conf').read_bytes()),('native-test',GUEST.encode())]:
                command(f': > /arch/tmp/{name}.b64; echo CARGA-$((40+2))','CARGA-42')
                encoded=base64.b64encode(raw).decode()
                for offset in range(0,len(encoded),256):
                    command(f"printf '%s' '{encoded[offset:offset+256]}' >> /arch/tmp/{name}.b64; echo CARGA-$((40+2))",'CARGA-42')
                command(f'busybox base64 -d /arch/tmp/{name}.b64 > /arch/tmp/{name}; echo CARGA-$((40+2))','CARGA-42')
            output=command('ultra-entrar /usr/bin/bash /tmp/native-test; echo RESULTADO-$?; echo TERMINADO-$((40+2))','TERMINADO-42',80)
            (work/'resultado.txt').write_text(output,encoding='utf-8',newline='\n');print(output)
            if 'NATIVA-FIN-42' not in output or 'RESULTADO-0' not in output:raise RuntimeError('Banco nativo fallo')
            conn.sendall(b'poweroff\n');b.read_until(conn,'APAGADO',40);process.wait(timeout=20)
        finally:
            if conn:conn.close()
            if process.poll() is None:process.terminate();process.wait(timeout=10)
if __name__=='__main__':main()
