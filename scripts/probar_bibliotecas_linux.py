#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Component test in a private mount namespace in the disposable workshop VM.
Only mount roots are redirected to a new scratch directory. No real disks.
"""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

assert os.geteuid() == 0
assert Path('/proc/self/ns/mnt').readlink() != Path('/proc/1/ns/mnt').readlink(), 'Ejecutar con unshare --mount --fork en el taller desechable'
subprocess.run(['mount', '--make-rprivate', '/'], check=True)
spec = importlib.util.spec_from_file_location('bibliotecas', sys.argv[1])
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)
root = Path(tempfile.mkdtemp(prefix='ultra-bibliotecas-', dir='/var/tmp'))
os.chmod(root, 0o755)
source, data = root/'origen', root/'ext4'
source.mkdir(); data.mkdir()
count = 0


def passed(label):
    global count
    count += 1
    print('PASS', count, label, flush=True)


def user_write(path, text):
    subprocess.run(['setpriv', '--reuid=1000', '--regid=1000', '--clear-groups',
                    'python', '-c', 'from pathlib import Path; import sys; p=Path(sys.argv[1]); p.parent.mkdir(parents=True,exist_ok=True); p.write_text(sys.argv[2])',
                    str(path), text], check=True)


try:
    subprocess.run(['mount', '-t', 'tmpfs', '-o', 'mode=0755', 'tmpfs', str(source)], check=True)
    folder = source/'Juegos, dos Ω'
    folder.mkdir()
    (folder/'steamapps').mkdir()
    (folder/'steamapps/compatdata').mkdir()
    (folder/'steamapps/shadercache').mkdir()
    (folder/'steamapps/compatdata/windows.txt').write_text('NO USAR COMO PREFIJO LINUX')
    (folder/'steamapps/shadercache/windows.txt').write_text('CACHE WINDOWS')
    manifest = folder/'steamapps/appmanifest_22.acf'
    manifest.write_text('"AppState" { "buildid" "12345" }')
    (folder/'juego.exe').write_text('ORIGINAL')
    (source/'redirigido').symlink_to('/etc', target_is_directory=True)
    secret = source/'privado-root'
    secret.mkdir(mode=0o700)
    (secret/'hijo-publico').mkdir(mode=0o755)
    (secret/'hijo-publico/secreto.txt').write_text('NO EXPONER')
    protected_steam = source/'Steam-con-apps-privadas'
    protected_steam.mkdir(mode=0o755)
    (protected_steam/'steamapps').mkdir(mode=0o700)
    for path in [folder, *folder.rglob('*')]:
        os.chown(path, 1000, 1000)
    subprocess.run(['mount', '-o', 'remount,ro,nosuid,nodev,noexec', str(source)], check=True)
    image = root/'datos.ext4'
    # Sparse file: enough virtual space for the production 4-GiB free-space guard.
    with image.open('wb') as stream: stream.truncate(8*1024*1024*1024)
    subprocess.run(['mkfs.ext4', '-q', '-F', str(image)], check=True)
    subprocess.run(['mount', '-o', 'loop,nosuid,nodev', str(image), str(data)], check=True)
    mod.PRIVATE = data/'privado'
    mod.TARGETS = data/'bibliotecas'
    mod.LOWERS = root/'fuentes'
    for path in (mod.PRIVATE, mod.TARGETS, mod.LOWERS): mod.checked_dir(path)
    item = mod.selection({'uuid':'1111-ABCD', 'relative':folder.name, 'kind':'steam'})
    volume = {'path': str(source)}
    target = Path(mod.prepare_folder(item, volume))
    passed('FD fijado y alias permiten espacio, coma y Unicode sin inyectar opciones')
    assert not (target/'steamapps/compatdata/windows.txt').exists()
    assert not (target/'steamapps/shadercache/windows.txt').exists()
    passed('prefijos y caches Windows ocultos por directorios opacos Linux')
    user_write(target/'juego.exe', 'CAMBIO LINUX')
    user_write(target/'nuevo.txt', 'PERSISTENTE')
    assert (folder/'juego.exe').read_text() == 'ORIGINAL' and not (folder/'nuevo.txt').exists()
    passed('UID1000 escribe la capa ext4 sin modificar origen protegido')
    user_write(target/'steamapps/compatdata/linux.txt', 'PREFIJO')
    user_write(target/'steamapps/shadercache/linux.txt', 'CACHE LINUX')
    runtime_manifest='"AppState" { "appid" "1493710" "name" "Proton Experimental" "StateFlags" "4" "buildid" "123" "installdir" "Proton - Experimental" }'
    user_write(target/'steamapps/appmanifest_1493710.acf', runtime_manifest)
    user_write(target/'steamapps/common/Proton - Experimental/proton', 'RUNTIME LINUX')
    passed('Steam puede crear metadatos y prefijos sobre ext4')
    subprocess.run(['umount', str(target)], check=True)
    target = Path(mod.prepare_folder(item, volume))
    assert (target/'nuevo.txt').read_text() == 'PERSISTENTE'
    passed('remontar conserva cambios y reconoce el origen')
    bad = mod.selection({'uuid':'1111-ABCD', 'relative':'redirigido', 'kind':'folder'})
    try:
        mod.prepare_folder(bad, volume)
    except OSError:
        passed('rechaza componente simbolico antes del bind privilegiado')
    else:
        raise AssertionError('ruta simbolica admitida')
    subprocess.run(['umount', str(target)], check=True)
    subprocess.run(['mount', '-o', 'remount,rw', str(source)], check=True)
    manifest.write_text('"AppState" { "buildid" "67890" }')
    subprocess.run(['mount', '-o', 'remount,ro,nosuid,nodev,noexec', str(source)], check=True)
    target = Path(mod.prepare_folder(item, volume))
    assert not (target/'nuevo.txt').exists()
    assert (target/'juego.exe').read_text() == 'ORIGINAL'
    archives = list(mod.PRIVATE.glob(item['id']+'.anterior-*'))
    assert len(archives) == 1
    assert (archives[0]/'upper/nuevo.txt').read_text() == 'PERSISTENTE'
    assert (target/'steamapps/compatdata/linux.txt').read_text() == 'PREFIJO'
    assert (target/'steamapps/shadercache/linux.txt').read_text() == 'CACHE LINUX'
    assert not (target/'steamapps/compatdata/windows.txt').exists()
    assert not (archives[0]/'upper/steamapps/compatdata').exists()
    passed('cambio Windows conserva prefijos y cache Linux activos sin revelar datos Windows')
    assert (target/'steamapps/common/Proton - Experimental/proton').read_text() == 'RUNTIME LINUX'
    assert (target/'steamapps/appmanifest_1493710.acf').read_text() == runtime_manifest
    passed('Proton completo y su manifiesto sobreviven al cambio de generacion sin descargar otra copia')
    bad_old, bad_new = data/'old-malicioso', data/'new-seguro'
    for store in (bad_old, bad_new):
        mod.checked_dir(store, 0o700)
        for suffix in ('upper', 'upper/steamapps'):
            p=store/suffix; p.mkdir(mode=0o755); os.chown(p,1000,1000)
    old_apps=bad_old/'upper/steamapps'; new_apps=bad_new/'upper/steamapps'
    (old_apps/'compatdata').mkdir(); os.chown(old_apps/'compatdata',1000,1000)
    (old_apps/'compatdata/canary').write_text('CONSERVAR');os.chown(old_apps/'compatdata/canary',1000,1000)
    (old_apps/'appmanifest_1493710.acf').write_text(runtime_manifest);os.chown(old_apps/'appmanifest_1493710.acf',1000,1000)
    (old_apps/'common').mkdir(); os.chown(old_apps/'common',1000,1000)
    (old_apps/'common/Proton - Experimental').symlink_to('/etc',target_is_directory=True)
    try: mod.retain_linux_state(bad_old,bad_new)
    except OSError: pass
    else: raise AssertionError('runtime simbolico admitido')
    assert (old_apps/'compatdata/canary').read_text()=='CONSERVAR'
    assert not (new_apps/'compatdata').exists()
    passed('runtime simbolico se rechaza antes de mover prefijos; sin recorrido root de enlaces')
    (old_apps/'appmanifest_1493710.acf').write_text(runtime_manifest.replace('Proton - Experimental','../escape'))
    try: mod.retain_linux_state(bad_old,bad_new)
    except ValueError: pass
    else: raise AssertionError('ruta de runtime con escape admitida')
    assert (old_apps/'compatdata/canary').is_file()
    passed('manifiesto con escape se rechaza sin alterar prefijos')
    for relative in ('privado-root', 'privado-root/hijo-publico'):
        item = mod.selection({'uuid':'1111-ABCD', 'relative':relative, 'kind':'folder'})
        try:
            mod.prepare_folder(item, volume)
        except ValueError as error:
            assert 'no accesible' in str(error)
            passed('respeta permisos de lectura Unix en '+relative)
        else:
            raise AssertionError('montaje privilegiado expuso carpeta inaccesible')
    item = mod.selection({'uuid':'1111-ABCD', 'relative':protected_steam.name, 'kind':'steam'})
    try:
        mod.prepare_folder(item, volume)
    except ValueError as error:
        assert 'no accesible' in str(error)
        passed('biblioteca publica no revela steamapps privado')
    else:
        raise AssertionError('steamapps privado expuesto')
    print('RESULTADO', count, 'sin discos fisicos; descubrimiento se prueba en arranque nativo')
finally:
    for parent in (mod.TARGETS, mod.LOWERS):
        if parent.exists():
            for path in parent.iterdir():
                if mod.mounted(path): subprocess.run(['umount', str(path)], check=True)
    for path in (data, source):
        if mod.mounted(path): subprocess.run(['umount', str(path)], check=True)
    print('Banco conservado:', root)
