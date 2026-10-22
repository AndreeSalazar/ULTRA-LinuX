#!/usr/bin/env python3
"""Grow only the backed-up 96 GiB candidate in build; no NIC or game clients.

The original D: image is never attached. Publication is a separate host action.
"""
import base64
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
WORK = ROOT/'build/almacen-ampliado'
IMAGE = WORK/'ULTRA-juegos.ext4'
UUID = 'ecbc9f71-f43c-4d20-9b71-a7929f264ba8'


def main():
    assert IMAGE.is_file() and not IMAGE.is_symlink() and IMAGE.stat().st_size == 96*1024**3
    backup = json.loads((WORK/'respaldo.json').read_text(encoding='utf-8-sig'))
    assert backup['bytes'] == 16*1024**3 and len(backup['sha256']) == 64
    assert (WORK/'ULTRA-juegos-respaldo.ext4').stat().st_size == backup['bytes']
    sys.argv = [sys.argv[0], 'generico', '--bmo']
    spec = importlib.util.spec_from_file_location('base', ROOT/'scripts/probar_arranque.py')
    base = importlib.util.module_from_spec(spec); spec.loader.exec_module(base)
    base.check_kernel()
    variables = WORK/'vars.fd'
    shutil.copyfile(base.QEMU_SHARE/'edk2-i386-vars.fd', variables)
    script = '''set -euo pipefail
[[ $(blockdev --getro /dev/vdb) == 0 ]]
[[ $(blockdev --getsize64 /dev/vdb) == 103079215104 ]]
[[ $(blkid -p -s TYPE -o value /dev/vdb) == ext4 ]]
[[ $(blkid -p -s UUID -o value /dev/vdb) == EXPECTED_UUID ]]
if findmnt -rn -S /dev/vdb | grep -q .; then exit 20; fi
rc=0; e2fsck -f -p /dev/vdb || rc=$?
((rc <= 1)) || exit "$rc"
resize2fs /dev/vdb
e2fsck -f -n /dev/vdb
[[ $(blkid -p -s UUID -o value /dev/vdb) == EXPECTED_UUID ]]
mkdir -p /mnt/ampliacion
mount -t ext4 -o ro,noload,nosuid,nodev,noexec /dev/vdb /mnt/ampliacion
stat -f -c 'FS blocks=%b available=%a blocksize=%S' /mnt/ampliacion
df -B1 /mnt/ampliacion
umount /mnt/ampliacion
sync
'''.replace('EXPECTED_UUID', UUID)
    args = [str(base.QEMU), '-name', 'ULTRA copia mantenimiento sin red', '-machine', 'pc',
            '-accel', 'whpx', '-cpu', 'host', '-smp', '2', '-m', '2048',
            '-drive', f'if=pflash,format=raw,readonly=on,file={base.QEMU_SHARE/"edk2-x86_64-code.fd"}',
            '-drive', f'if=pflash,format=raw,file={variables}',
            '-drive', f'file={base.IMG},format=raw,if=virtio,snapshot=on',
            '-drive', f'file={IMAGE},format=raw,if=virtio', '-nic', 'none',
            '-display', 'none', '-monitor', 'none',
            '-serial', 'tcp:127.0.0.1:15965,server=on,wait=off', '-no-reboot']
    log, connection = '', None
    with (WORK/'qemu.txt').open('w') as errors:
        process = subprocess.Popen(args, stdout=errors, stderr=errors,
                                   creationflags=subprocess.CREATE_NO_WINDOW)
        try:
            for _ in range(100):
                try:
                    connection = socket.create_connection(('127.0.0.1',15965),timeout=1); break
                except OSError:
                    time.sleep(.2)
            assert connection is not None
            log += base.read_until(connection, 'ultra:', 90)
            def command(text, marker, timeout=30):
                nonlocal log
                connection.sendall((text+'\n').encode())
                try:
                    output = base.read_until(connection, marker, timeout)
                except base.Incomplete as error:
                    log += str(error) + '\n' + repr(vars(error))
                    raise
                log += output
                return output
            command(': > /arch/tmp/ampliar.b64; echo CARGA-$((40+2))', 'CARGA-42')
            encoded = base64.b64encode(script.encode()).decode()
            for offset in range(0,len(encoded),256):
                command(f"printf '%s' '{encoded[offset:offset+256]}' >> /arch/tmp/ampliar.b64; echo CARGA-$((40+2))", 'CARGA-42')
            command('busybox base64 -d /arch/tmp/ampliar.b64 > /arch/tmp/ampliar.sh; echo CARGA-$((40+2))', 'CARGA-42')
            output = command('ultra-entrar /usr/bin/bash /tmp/ampliar.sh; echo RESULTADO-$?; echo FINAL-$((40+2))', 'FINAL-42', 900)
            assert re.search(r'(?m)^RESULTADO-0\r?$',output), output
            match = re.search(r'FS blocks=(\d+) available=(\d+) blocksize=(\d+)', output)
            assert match, output
            blocks, available, blocksize = map(int,match.groups())
            assert blocks*blocksize > 94*1024**3 and available*blocksize > 75*1024**3
            connection.sendall(b'poweroff\n'); log += base.read_until(connection,'APAGADO',40)
            process.wait(timeout=20)
            result = {'candidato_bytes':IMAGE.stat().st_size,'uuid':UUID,
                      'bloques_fs':blocks,'bloque_bytes':blocksize,
                      'disponible_usuario_bytes':available*blocksize,
                      'fsck_posterior':0,'red':False,'clientes_ejecutados':False,
                      'D_adjuntado':False,'respaldo_original_sha256':backup['sha256']}
            (WORK/'resultado.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
            print(json.dumps(result,indent=2))
        finally:
            if connection is not None: connection.close()
            (WORK/'consola.txt').write_text(log,encoding='utf-8')
            if process.poll() is None: process.kill(); process.wait(timeout=30)


if __name__ == '__main__': main()
