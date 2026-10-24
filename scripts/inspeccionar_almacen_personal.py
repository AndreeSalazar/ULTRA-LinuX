#!/usr/bin/env python3
"""Offline storage accounting: one read-only ext4 file, no NIC, no clients.

Never runs game tests with personal storage or opens authentication stores.
Optional engine-log inspection exports fixed categories, never raw lines.
"""
import importlib.util
import argparse
import base64
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
UUID = 'ecbc9f71-f43c-4d20-9b71-a7929f264ba8'
PORT = 15965
BUCKETS = {
    'total_archivos': '',
    'home_total': 'usuarios/ultra',
    'steam_total': 'usuarios/ultra/.local/share/Steam',
    'steam_common_Proton_runtimes': 'usuarios/ultra/.local/share/Steam/steamapps/common',
    'steam_descargas_temporales': 'usuarios/ultra/.local/share/Steam/steamapps/downloading',
    'steam_shadercache': 'usuarios/ultra/.local/share/Steam/steamapps/shadercache',
    'steam_compatdata': 'usuarios/ultra/.local/share/Steam/steamapps/compatdata',
    'cache_usuario': 'usuarios/ultra/.cache',
    'prefijos_UMU': 'usuarios/ultra/Juegos/prefijos',
    'capas_bibliotecas': '.ultra-bibliotecas',
    'biblioteca_Steam_upper': '.ultra-bibliotecas/5f9b020e65d5dfb89025/upper',
    'biblioteca_Steam_common': '.ultra-bibliotecas/5f9b020e65d5dfb89025/upper/steamapps/common',
    'biblioteca_Steam_descargando': '.ultra-bibliotecas/5f9b020e65d5dfb89025/upper/steamapps/downloading',
    'biblioteca_Steam_shadercache': '.ultra-bibliotecas/5f9b020e65d5dfb89025/upper/steamapps/shadercache',
    'biblioteca_Steam_compatdata': '.ultra-bibliotecas/5f9b020e65d5dfb89025/upper/steamapps/compatdata',
    'capa_Warframe_anterior': '.ultra-warframe',
    'capa_Cyberpunk': 'overlay',
}

WARFRAME_QUERY = r'''
from pathlib import Path
import json,os,re,stat
root=Path('/mnt/inspeccion')
home=root/'usuarios/ultra'
def bounded(path,limit):
    # Do not follow redirected paths outside this read-only mount.
    resolved=path.resolve()
    if not resolved.is_relative_to(root): return None
    try:
        fd=os.open(resolved,os.O_RDONLY|os.O_NOFOLLOW|os.O_NONBLOCK)
        with os.fdopen(fd,'rb') as f:
            if not stat.S_ISREG(os.fstat(f.fileno()).st_mode): return None
            raw=f.read(limit+1)
            return raw[:limit].decode('utf-8',errors='replace')
    except OSError: return None
folder=home/'.cache/ultra/rendimiento'
files=[]
if folder.is_dir() and folder.resolve().is_relative_to(root):
    for index,p in enumerate(folder.iterdir()):
        if index>=256: break
        if not p.is_symlink() and p.is_file() and p.suffix=='.csv':
            files.append(dict(nombre=p.name[:160],bytes=p.stat().st_size))
conf=bounded(home/'.config/ultra/MangoHud.conf',65536)
keys={}
if conf:
    for line in conf.splitlines():
        key,sep,value=line.partition('=')
        if sep and key.strip() in ('log_interval','log_duration','permit_upload','autostart_log'):
            keys[key.strip()]=value.strip()[:64]
roots=[home/'.local/share/Steam/steamapps/compatdata/230410',
       root/'.ultra-bibliotecas/5f9b020e65d5dfb89025/upper/steamapps/compatdata/230410']
logs=[]
for index,prefix in enumerate(roots):
    path=prefix/'pfx/drive_c/users/steamuser/AppData/Local/Warframe/EE.log'
    raw=bounded(path,4*1024*1024)
    if raw is None: continue
    # EE.log can contain private identifiers: export fixed categories/counts,
    # never raw lines, network addresses, command lines or account details.
    low=raw.lower()
    flags={k: bool(re.search(v,low)) for k,v in {
        'menciona_d3d11':r'\b(?:d3d11|directx ?11|direct3d ?11|dx11)\b',
        'menciona_d3d12':r'\b(?:d3d12|directx ?12|direct3d ?12|dx12)\b',
        'device_removed':r'dxgi_error_device_(?:removed|hung|reset)',
        'compilacion_shader':r'(?:compil\w*[^\n]{0,80}shader|shader[^\n]{0,80}compil)',
        'io_error':r'(?:i/o error|no space left|disk full)',
        'shader_cache':r'shader[^\n]{0,40}cache'}.items()}
    # Counts refer only to the bounded beginning of the last engine log.
    logs.append(dict(prefijo=index,bytes_log=path.resolve().stat().st_size,flags=flags,
                     alcance='primeros 4 MiB; menciones no prueban causalidad ni API activa'))
print('WARFRAME_DIAG '+json.dumps(dict(csv=files[:32],csv_listado_limitado=len(files)>32,
    hud_config=keys,engine_logs=logs,raw_logs_exportados=False)))
'''


def public_sizes(output, path):
    known, other = {}, 0
    for line in output.splitlines():
        match = re.match(r'^(\d+)\s+(.*)$', line)
        if not match:
            continue
        target = match[2]
        if target.startswith("'") and target.endswith("'"):
            target = target[1:-1]
        if not target.startswith(path+'/'):
            continue
        name = target[len(path)+1:]
        if '/' in name:
            continue
        if (name.startswith(('Proton','SteamLinuxRuntime','ubuntu12_','steamrt'))
                or name in ('Warframe','Left 4 Dead 2','package','steamapps','compatibilitytools.d','config','userdata','logs')):
            known[name] = int(match[1])
        else:
            other += int(match[1])
    return {'directorios_publicos_bytes':known, 'otros_directorios_bytes':other}


def main():
    global IMAGE
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--imagen', type=Path, default=IMAGE,
                        choices=[IMAGE, Path('D:/ULTRA-juegos.ext4')])
    parser.add_argument('--warframe', action='store_true', help='Solo inventario CSV y categorias sanitizadas de EE.log')
    options = parser.parse_args()
    IMAGE = options.imagen
    assert IMAGE.is_file() and not IMAGE.is_symlink()
    before = IMAGE.stat()
    sys.argv = [sys.argv[0], 'generico', '--bmo']
    spec = importlib.util.spec_from_file_location('base', ROOT/'scripts/probar_arranque.py')
    base = importlib.util.module_from_spec(spec); spec.loader.exec_module(base)
    base.check_kernel()
    work = ROOT/('build/inspeccion-sololectura-'+IMAGE.drive[0].lower())
    work.mkdir(parents=True, exist_ok=True)
    variables = work/'vars.fd'
    shutil.copyfile(base.QEMU_SHARE/'edk2-i386-vars.fd', variables)
    args = [str(base.QEMU), '-name', 'ULTRA almacenamiento solo lectura', '-machine', 'pc',
            '-accel', 'whpx', '-cpu', 'host', '-smp', '2', '-m', '2048',
            '-drive', f'if=pflash,format=raw,readonly=on,file={base.QEMU_SHARE/"edk2-x86_64-code.fd"}',
            '-drive', f'if=pflash,format=raw,file={variables}',
            '-drive', f'file={base.IMG},format=raw,if=virtio,snapshot=on',
            '-drive', f'file={IMAGE},format=raw,if=virtio,readonly=on',
            '-nic', 'none', '-display', 'none', '-monitor', 'none',
            '-serial', f'tcp:127.0.0.1:{PORT},server=on,wait=off', '-no-reboot']
    log, connection = '', None
    with (work/'qemu.txt').open('w') as errors:
        process = subprocess.Popen(args, stdout=errors, stderr=errors,
                                   creationflags=subprocess.CREATE_NO_WINDOW)
        try:
            deadline = time.monotonic()+30
            while time.monotonic() < deadline:
                try:
                    connection = socket.create_connection(('127.0.0.1', PORT), timeout=2)
                    break
                except OSError:
                    time.sleep(.3)
            assert connection is not None, 'consola de mantenimiento no disponible'
            log += base.read_until(connection, 'ultra:', 70)
            def command(text, marker, timeout=60):
                nonlocal log
                connection.sendall((text+'\n').encode())
                output = base.read_until(connection, marker, timeout)
                log += output
                return output
            output = command('ultra-entrar /usr/bin/blockdev --getro /dev/vdb; '
                             'ultra-entrar /usr/bin/blkid -p -s UUID -o value /dev/vdb; '
                             'echo INSPECCION-IDENTIDAD-$((40+2))', 'INSPECCION-IDENTIDAD-42')
            assert re.search(r'(?m)^1\r?$', output) and UUID in output
            # A read-only block device plus ro,noload prevents journal replay.
            output = command('ultra-entrar /usr/bin/mkdir -p /mnt/inspeccion; '
                             'ultra-entrar /usr/bin/mount -t ext4 -o ro,noload,nosuid,nodev,noexec /dev/vdb /mnt/inspeccion; '
                             'ultra-entrar /usr/bin/findmnt -n -o OPTIONS --mountpoint /mnt/inspeccion; '
                             'echo INSPECCION-MONTAJE-$((40+2))', 'INSPECCION-MONTAJE-42')
            assert re.search(r'(?m)^ro,.*(?:noload|norecovery)', output), output
            space = command('ultra-entrar /usr/bin/stat -f -c "ESPACIO %b %f %a %S" /mnt/inspeccion; '
                            'echo INSPECCION-ESPACIO-$((40+2))', 'INSPECCION-ESPACIO-42')
            match = re.search(r'ESPACIO (\d+) (\d+) (\d+) (\d+)', space)
            assert match, space
            blocks, free, available, unit = map(int, match.groups())
            if options.warframe:
                command(': > /arch/tmp/warframe-query.b64; echo CARGA-$((40+2))','CARGA-42')
                encoded=base64.b64encode(WARFRAME_QUERY.encode()).decode()
                for offset in range(0,len(encoded),192):
                    command(f"printf '%s' '{encoded[offset:offset+192]}' >> /arch/tmp/warframe-query.b64; echo CARGA-$((40+2))",'CARGA-42')
                command('busybox base64 -d /arch/tmp/warframe-query.b64 > /arch/tmp/warframe-query.py; echo CARGA-$((40+2))','CARGA-42')
                output=command('ultra-entrar /usr/bin/python /tmp/warframe-query.py; echo RESULTADO-$?; echo FINAL-$((40+2))','FINAL-42')
                assert re.search(r'(?m)^RESULTADO-0\r?$',output),output
                result=json.loads(re.search(r'(?m)^WARFRAME_DIAG (.+)\r?$',output)[1])
                command('ultra-entrar /usr/bin/umount /mnt/inspeccion; echo SALIDA-$?','SALIDA-0')
                connection.sendall(b'poweroff\n'); log+=base.read_until(connection,'APAGADO',40)
                process.wait(timeout=20)
                after=IMAGE.stat()
                assert (before.st_size,before.st_mtime_ns)==(after.st_size,after.st_mtime_ns)
                result.update(solo_lectura=True,red=False,imagen_conservada=True,
                              disponible_usuario_bytes=available*unit)
                (work/'warframe.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
                print(json.dumps(result,indent=2),flush=True)
                return
            sizes = {}
            for label, relative in BUCKETS.items():
                # These are source-code constants, not filenames read from disk.
                path = '/mnt/inspeccion/'+relative
                output = command(f'ultra-entrar /usr/bin/du -sx -B1 "{path}"; '
                                 'echo INSPECCION-DU-$((40+2))', 'INSPECCION-DU-42', 90)
                match = re.search(r'(?m)^(\d+)\s+/mnt/inspeccion', output)
                sizes[label] = int(match[1]) if match else None
            software = {}
            output = command('ultra-entrar /usr/bin/du -x -B1 --max-depth=2 /mnt/inspeccion/.ultra-bibliotecas; '
                             'echo INSPECCION-CAPAS-$((40+2))', 'INSPECCION-CAPAS-42', 120)
            layers = {}
            for count, suffix in re.findall(r'(?m)^(\d+)\s+/mnt/inspeccion/\.ultra-bibliotecas/([a-zA-Z0-9_.\-/]+)\r?$', output):
                layers[suffix] = int(count)
            for label, relative in [('biblioteca_common', BUCKETS['biblioteca_Steam_common']),
                                    ('cliente_Steam', BUCKETS['steam_total'])]:
                path = '/mnt/inspeccion/'+relative
                output = command(f'ultra-entrar /usr/bin/du -x -B1 --max-depth=1 "{path}"; '
                                 'echo INSPECCION-SOFTWARE-$((40+2))', 'INSPECCION-SOFTWARE-42', 90)
                software[label] = public_sizes(output, path)
            output = command('ultra-entrar /usr/bin/umount /mnt/inspeccion; echo INSPECCION-SALIDA-$?', 'INSPECCION-SALIDA-0')
            connection.sendall(b'poweroff\n'); log += base.read_until(connection, 'APAGADO', 40)
            process.wait(timeout=20)
            after = IMAGE.stat()
            assert (before.st_size, before.st_mtime_ns) == (after.st_size, after.st_mtime_ns)
            result = {'solo_lectura_QEMU':True, 'montaje_ro_noload':True, 'red':False,
                      'archivo':str(IMAGE), 'capacidad_bytes':blocks*unit,
                      'usado_bytes':(blocks-free)*unit, 'libre_bytes':free*unit,
                      'disponible_usuario_bytes':available*unit,
                      'clientes_ejecutados':False, 'imagen_tamano_mtime_conservados':True,
                      'bloques_asignados_bytes':sizes,
                      'capas_bytes':layers,
                      'software':software,
                      'nota':'Totales contienen subtotales; no sumar todas las filas. du no sigue enlaces.'}
            (work/'tamanos.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
            print(json.dumps(result,indent=2), flush=True)
        finally:
            if connection is not None:
                connection.close()
            if process.poll() is None:
                process.kill(); process.wait(timeout=10)
            (work/'consola.txt').write_text(log,encoding='utf-8')


if __name__ == '__main__':
    main()
