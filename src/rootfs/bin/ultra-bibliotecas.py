#!/usr/bin/python
# SPDX-License-Identifier: GPL-2.0-only
"""Selected folders on read-only data volumes; mounts are prepared before login.

No shell evaluation, host drive letters, automatic formatting or live root IPC.
User configuration is exported by UID1000, never opened with root privileges.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys
import tempfile

HOME = Path('/juegos/usuarios/ultra')
CONFIG = HOME / '.config/ultra/bibliotecas.json'
CATALOG = Path('/run/ultra-bibliotecas.json')
VOLUMES = Path('/discos')
TARGETS = Path('/juegos/Bibliotecas')
PRIVATE = Path('/juegos/.ultra-bibliotecas')
LOWERS = Path('/run/ultra-bibliotecas/fuentes')
LIMIT = 262144
UUID = re.compile(r'[A-Za-z0-9-]{1,64}')
EXCLUDED = {'c12a7328-f81f-11d2-ba4b-00a0c93ec93b',
            'e3c9e316-0b5c-4db8-817d-f92df00215ae',
            'de94bba4-06d1-4d40-a16a-bfd50179d6ac'}


def run(*args, **kwargs):
    return subprocess.run(args, check=True, text=True, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE, timeout=30, **kwargs).stdout


def read_json(path):
    with path.open('rb') as stream:
        raw = stream.read(LIMIT + 1)
    if len(raw) > LIMIT:
        raise ValueError('configuracion demasiado grande')
    return json.loads(raw)


def selection(item):
    if not isinstance(item, dict):
        raise ValueError('seleccion invalida')
    uuid, relative, kind = item.get('uuid'), item.get('relative'), item.get('kind')
    if not isinstance(uuid, str) or not UUID.fullmatch(uuid):
        raise ValueError('UUID invalido')
    if not isinstance(relative, str) or not relative or len(relative.encode('utf-8')) > 4096:
        raise ValueError('carpeta invalida')
    if relative and (relative.startswith('/') or any(p in ('', '.', '..') for p in relative.split('/'))):
        raise ValueError('ruta absoluta o recorrido de padres rechazado')
    if any(ord(c) < 32 for c in relative) or kind not in ('steam', 'folder'):
        raise ValueError('tipo de carpeta o caracteres invalidos')
    token = hashlib.sha256((uuid+'\0'+relative+'\0'+kind).encode()).hexdigest()[:20]
    return {'uuid': uuid, 'relative': relative, 'kind': kind, 'id': token}


def user_config():
    if os.geteuid() != 1000 or Path(os.environ.get('HOME', '')) != HOME:
        raise ValueError('se requiere el usuario del escritorio y su home persistente')
    try:
        value = read_json(CONFIG)
    except FileNotFoundError:
        return []
    if not isinstance(value, list) or len(value) > 32:
        raise ValueError('se admiten hasta 32 carpetas')
    result = [selection(item) for item in value]
    if len({item['id'] for item in result}) != len(result):
        raise ValueError('selecciones duplicadas')
    return result


def atomic_json(path, value, mode=0o600):
    fd, name = tempfile.mkstemp(prefix='.bibliotecas-', dir=path.parent)
    try:
        with os.fdopen(fd, 'w', encoding='utf-8') as stream:
            os.fchmod(stream.fileno(), mode)
            json.dump(value, stream, ensure_ascii=False)
            stream.write('\n')
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(name, path)
    finally:
        if os.path.exists(name):
            os.unlink(name)


def checked_dir(path, mode=0o755):
    try:
        value = path.lstat()
    except FileNotFoundError:
        path.mkdir(mode=mode)
        value = path.lstat()
    if not stat.S_ISDIR(value.st_mode) or value.st_uid != 0 or value.st_mode & 0o022:
        raise ValueError('destino de montaje inseguro: '+str(path))
    os.chmod(path, mode)


def mounted(path):
    try:
        data = json.loads(run('/usr/bin/findmnt', '--json', '--mountpoint', str(path),
                              '-o', 'TARGET,SOURCE,FSTYPE,OPTIONS,UUID'))
        view = data['filesystems'][0]
        device = (view.get('source') or '').split('[', 1)[0]
        if not view.get('uuid') and device.startswith('/dev/') and Path(device).is_block_device():
            try:
                view['uuid'] = run('/usr/bin/blkid', '-p', '-s', 'UUID', '-o', 'value', device).strip()
            except subprocess.CalledProcessError:
                pass
        return view
    except subprocess.CalledProcessError:
        return None


def readonly_bind(source, target, fd=None):
    checked_dir(target)
    if mounted(target):
        raise ValueError('destino ya montado')
    run('/usr/bin/mount', '-i', '--no-canonicalize', '--bind', str(source), str(target),
        pass_fds=(() if fd is None else (fd,)))
    try:
        run('/usr/bin/mount', '-i', '-o', 'remount,bind,ro,nosuid,nodev,noexec', str(target))
    except Exception:
        run('/usr/bin/umount', str(target))
        raise


def enumerate_volumes():
    checked_dir(VOLUMES)
    data = json.loads(run('/usr/bin/lsblk', '-J', '-p', '-o', 'NAME,TYPE,FSTYPE,UUID,LABEL,PARTTYPE'))
    nodes = []

    def visit(items):
        for node in items:
            if node.get('type') in ('part', 'disk'):
                # Cold boots can precede udev's filesystem properties. Probe metadata
                # read-only instead of treating an empty cache as an empty disk.
                if not node.get('fstype') or not node.get('uuid'):
                    try:
                        raw = run('/usr/bin/blkid', '-p', '-o', 'export', node['name'])
                        properties = dict(line.split('=', 1) for line in raw.splitlines() if '=' in line)
                        node['fstype'] = properties.get('TYPE', node.get('fstype'))
                        node['uuid'] = properties.get('UUID', node.get('uuid'))
                        node['label'] = properties.get('LABEL', node.get('label'))
                        node['parttype'] = properties.get('PART_ENTRY_TYPE', node.get('parttype'))
                    except subprocess.CalledProcessError:
                        pass
                if node.get('fstype') in ('ntfs', 'ext4', 'exfat'):
                    nodes.append(node)
            visit(node.get('children', []))
    visit(data['blockdevices'])
    counts = {}
    for node in nodes:
        uuid = node.get('uuid')
        counts[uuid] = counts.get(uuid, 0) + 1
    result = []
    for node in nodes:
        uuid = node.get('uuid')
        if (not isinstance(uuid, str) or not UUID.fullmatch(uuid) or counts[uuid] != 1
                or (node.get('parttype') or '').lower() in EXCLUDED
                or node.get('label') in ('ULTRA-DATOS', 'ULTRA-SISTEMA-A', 'BMO')):
            continue
        target = VOLUMES / uuid
        entry = {'uuid': uuid, 'label': node.get('label') or node['name'],
                 'fstype': node['fstype'], 'path': str(target)}
        try:
            active = mounted(target)
            if active:
                if active.get('uuid') != uuid or not {'ro', 'nosuid', 'nodev', 'noexec'} <= set(active['options'].split(',')):
                    raise ValueError('vista existente no corresponde al volumen protegido')
            else:
                # Reuse C: and D: mounts; D: must stay writable privately for the ext4 image.
                source = None
                for candidate in (Path('/windows'), Path('/run/ultra-juegos/volumen')):
                    view = mounted(candidate)
                    if view and view.get('uuid') == uuid:
                        source = candidate
                        break
                if source:
                    readonly_bind(source, target)
                else:
                    try:
                        run('/usr/bin/findmnt', '-n', '-S', node['name'])
                    except subprocess.CalledProcessError:
                        pass
                    else:
                        raise ValueError('volumen ya en uso; no se modifica su montaje')
                    run('/usr/bin/blockdev', '--setro', node['name'])
                    if run('/usr/bin/blockdev', '--getro', node['name']).strip() != '1':
                        raise ValueError('no se pudo proteger el dispositivo')
                    checked_dir(target)
                    fs = node['fstype']
                    options = 'ro,nosuid,nodev,noexec'
                    if fs in ('ntfs', 'exfat'):
                        options += ',uid=1000,gid=1000,dmask=0022,fmask=0133'
                    if fs == 'ext4':
                        options += ',noload'
                    run('/usr/bin/mount', '-i', '-t', 'ntfs3' if fs == 'ntfs' else fs,
                        '-o', options, node['name'], str(target))
            entry['status'] = 'lectura'
        except (OSError, ValueError, subprocess.SubprocessError) as error:
            entry['status'] = 'no disponible: '+str(error)
        result.append(entry)
    return result


def open_folder(volume, relative):
    """Pin each component without following symlinks, including the volume root."""
    fd = os.open(volume, os.O_PATH | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        for part in relative.split('/') if relative else []:
            child = os.open(part, os.O_PATH | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=fd)
            os.close(fd)
            fd = child
        return fd
    except Exception:
        os.close(fd)
        raise


def signature(alias, kind):
    if kind != 'steam':
        return str(alias.stat().st_ino)
    apps = alias / 'steamapps'
    if apps.is_symlink() or not apps.is_dir():
        raise ValueError('no contiene steamapps como directorio normal')
    versions = []
    for path in sorted(apps.glob('appmanifest_*.acf')):
        if len(versions) >= 512:
            raise ValueError('demasiados manifiestos para esta biblioteca')
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        with os.fdopen(fd, 'rb') as stream:
            if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
                raise ValueError('manifiesto no regular')
            raw = stream.read(LIMIT + 1)
        if len(raw) > LIMIT:
            raise ValueError('manifiesto demasiado grande')
        build = re.search(rb'"buildid"\s+"([0-9]+)"', raw)
        versions.append((path.name, build[1].decode() if build else 'sin-version'))
    return hashlib.sha256(json.dumps(versions).encode()).hexdigest()


def prepare_folder(item, volume):
    token = item['id']
    target, store, alias = TARGETS/token, PRIVATE/token, LOWERS/token
    if mounted(target):
        if mounted(target)['fstype'] != 'overlay':
            raise ValueError('destino ocupado por otro sistema')
        return str(target)
    # A root mount must not turn an unreadable Unix directory into a readable
    # user-owned overlay. Check the full path (including ancestors) as UID1000.
    source = Path(volume['path'])/item['relative']
    readable = [source]
    if item['kind'] == 'steam':
        readable.append(source/'steamapps')
    for path in readable:
        for permission in ('-r', '-x'):
            try:
                run('/usr/bin/setpriv', '--reuid=1000', '--regid=1000', '--init-groups',
                    '--no-new-privs', '--bounding-set=-all', '--inh-caps=-all', '--ambient-caps=-all',
                    '/usr/bin/test', permission, str(path), stdin=subprocess.DEVNULL)
            except subprocess.CalledProcessError:
                raise ValueError('carpeta no accesible al usuario; no se elevan permisos de lectura') from None
    if mounted(alias):
        run('/usr/bin/umount', str(alias))
    checked_dir(alias)
    fd = open_folder(volume['path'], item['relative'])
    try:
        readonly_bind(f'/proc/self/fd/{fd}', alias, fd)
    finally:
        os.close(fd)
    try:
        version = signature(alias, item['kind'])
        identity = {**item, 'version': version}
        if store.exists() or store.is_symlink():
            checked_dir(store, 0o700)
            state = read_json(store/'source.json')
            if state != identity:
                raise ValueError('origen/version cambio: conservar y revisar la capa anterior')
            for child in ('upper', 'work'):
                if (store/child).is_symlink() or not (store/child).is_dir():
                    raise ValueError('capa redirigida o ausente')
        else:
            checked_dir(store, 0o700)
            (store/'upper').mkdir(mode=0o755)
            os.chown(store/'upper', 1000, 1000)
            (store/'work').mkdir(mode=0o700)
            if item['kind'] == 'steam':
                # Fresh upper only. Never walk user-modifiable children as root on later boots.
                apps = store/'upper/steamapps'
                apps.mkdir(mode=0o755)
                os.chown(apps, 1000, 1000)
                for child in ('compatdata', 'shadercache'):
                    directory = apps/child
                    directory.mkdir(mode=0o700)
                    os.chown(directory, 1000, 1000)
                    os.setxattr(directory, 'trusted.overlay.opaque', b'y')
            atomic_json(store/'source.json', identity)
        checked_dir(target)
        # All option paths are root-owned ASCII aliases, never user-provided option strings.
        options = f'nosuid,nodev,lowerdir={alias},upperdir={store}/upper,workdir={store}/work'
        run('/usr/bin/mount', '-i', '-t', 'overlay', 'overlay', '-o', options, str(target))
        return str(target)
    except Exception:
        if mounted(alias):
            run('/usr/bin/umount', str(alias))
        raise


def prepare():
    if os.geteuid() != 0:
        raise ValueError('preparacion reservada a root antes del escritorio')
    volumes = enumerate_volumes()
    catalog = {'volumes': volumes, 'folders': []}
    if mounted('/juegos') and mounted('/juegos')['fstype'] == 'ext4':
        checked_dir(Path('/juegos'))
        checked_dir(PRIVATE, 0o700)
        checked_dir(TARGETS)
        checked_dir(Path('/run/ultra-bibliotecas'), 0o700)
        checked_dir(LOWERS, 0o700)
        try:
            raw = run('/usr/bin/setpriv', '--reuid=1000', '--regid=1000', '--init-groups',
                      '--no-new-privs', '--bounding-set=-all', '--inh-caps=-all', '--ambient-caps=-all',
                      '/usr/bin/env', f'HOME={HOME}', '/usr/bin/python', '-I',
                      '/usr/local/libexec/ultra-bibliotecas.py', 'export', stdin=subprocess.DEVNULL)
            if len(raw.encode()) > LIMIT:
                raise ValueError('seleccion demasiado grande')
            items = [selection(value) for value in json.loads(raw)]
            for item in items:
                record = {**item, 'status': 'pendiente'}
                try:
                    volume = next(v for v in volumes if v['uuid'] == item['uuid'] and v['status'] == 'lectura')
                    record['path'] = prepare_folder(item, volume)
                    record['status'] = 'activa'
                except (StopIteration, OSError, ValueError, subprocess.SubprocessError) as error:
                    record['status'] = 'no disponible: '+(str(error) or 'disco ausente')
                catalog['folders'].append(record)
        except (OSError, ValueError, subprocess.SubprocessError) as error:
            catalog['error'] = str(error)
    else:
        catalog['error'] = 'falta el almacenamiento ext4 persistente'
    atomic_json(CATALOG, catalog, 0o644)
    print('BIBLIOTECAS:', json.dumps(catalog, ensure_ascii=True))


def choose(title, rows):
    labels = [f'{i+1}. {label.replace(chr(10), " ")}' for i, (label, _) in enumerate(rows)]
    process = subprocess.run(['fuzzel', '--dmenu', '--prompt='+title+' / ', '--lines=12'],
                             input='\n'.join(labels), text=True, stdout=subprocess.PIPE, check=False)
    if process.returncode:
        return None
    value = process.stdout.rstrip('\n')
    return rows[labels.index(value)][1] if value in labels else None


def pick_folder(catalog):
    rows = [(v['label']+' ['+v['uuid']+']', v) for v in catalog.get('volumes', []) if v['status'] == 'lectura']
    volume = choose('Disco', rows)
    if volume is None:
        return None
    relative = Path()
    while True:
        folder = Path(volume['path'])/relative
        children = sorted((p for p in folder.iterdir() if not p.is_symlink() and p.is_dir()
                           and not any(ord(c) < 32 for c in p.name)), key=lambda p: p.name.casefold())[:500]
        rows = []
        if relative != Path():
            rows.extend([('Elegir esta carpeta: '+str(relative), 'select'), ('Subir una carpeta', 'up')])
        rows.extend((p.name, p) for p in children)
        result = choose('Carpeta', rows)
        if result is None:
            return None
        if result == 'up':
            relative = relative.parent
        elif result == 'select':
            # Accept the Steam root or steamapps itself. Do not guess from EXE filenames.
            if folder.name == 'steamapps' and relative.parent != Path():
                folder, relative = folder.parent, relative.parent
            kind = 'steam' if (folder/'steamapps').is_dir() and not (folder/'steamapps').is_symlink() else 'folder'
            return selection({'uuid': volume['uuid'], 'relative': '' if relative == Path() else relative.as_posix(), 'kind': kind})
        else:
            relative = result.relative_to(Path(volume['path']))


def status():
    catalog = read_json(CATALOG)
    print('Carpetas elegidas (se aplican al volver a abrir el escritorio):')
    for item in user_config():
        active = next((r for r in catalog.get('folders', []) if r['id'] == item['id']), {})
        print(item['uuid'], item['relative'] or '/', item['kind'], active.get('status', 'pendiente'))
        print('  Steam/Heroic/archivos:', active.get('path', str(TARGETS/item['id'])))
    print('Las vistas /discos son solo lectura. Cambios y caches usan el ext4 de /juegos.')
    print('Una actualizacion puede copiar archivos grandes; comprueba el espacio libre.')
    if catalog.get('error'):
        print('AVISO:', catalog['error'])


def ui():
    config = user_config()
    catalog = read_json(CATALOG)
    action = choose('Bibliotecas', [('Agregar carpeta de cualquier disco', 'add'),
                                  ('Ver carpetas y estado', 'status'),
                                  ('Abrir carpeta preparada', 'open'),
                                  ('Jugar EXE con UMU / Proton', 'play'),
                                  ('Quitar seleccion (conservar archivos)', 'remove')])
    if action == 'add':
        item = pick_folder(catalog)
        if item is None or item['id'] in {v['id'] for v in config}:
            return
        if len(config) >= 32:
            raise ValueError('limite de 32 carpetas alcanzado')
        CONFIG.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        atomic_json(CONFIG, config+[item])
        run('/usr/bin/notify-send', 'Carpeta registrada', 'Cierra el escritorio y vuelve a abrirlo desde e para activar la carpeta. Luego agregala en Steam > Almacenamiento.')
    elif action == 'status':
        subprocess.Popen(['foot', '--hold', '/usr/local/bin/ultra-bibliotecas', 'estado'])
    elif action in ('open', 'play'):
        active = [r for r in catalog.get('folders', []) if r['status'] == 'activa']
        item = choose('Carpetas activas', [(r['relative'] or r['uuid'], r) for r in active])
        if item is None:
            return
        if action == 'open':
            subprocess.Popen(['thunar', item['path']])
        else:
            if item['kind'] == 'steam':
                raise ValueError('usa Steam para iniciar los juegos de esa biblioteca con su AppID y Proton')
            play(item)
    elif action == 'remove':
        item = choose('Quitar seleccion', [(r['relative'] or r['uuid'], r) for r in config])
        if item is not None:
            atomic_json(CONFIG, [r for r in config if r['id'] != item['id']])


def play(item):
    root = Path(item['path'])
    folder = root
    while True:
        rows = [] if folder == root else [('Subir una carpeta', 'up')]
        rows.extend((p.name, p) for p in sorted(folder.iterdir()) if not p.is_symlink()
                    and (p.is_dir() or p.suffix.lower() == '.exe'))
        value = choose('Ejecutable Windows', rows)
        if value is None:
            return
        if value == 'up':
            folder = folder.parent
            continue
        if value.is_dir():
            folder = value
            continue
        cache = HOME/'.cache/ultra'
        cache.mkdir(mode=0o700, parents=True, exist_ok=True)
        prefix = HOME/'Juegos/prefijos'/('carpeta-'+item['id'])
        env = dict(os.environ, WINEPREFIX=str(prefix), GAMEID='umu-0', PROTONPATH='GE-Proton')
        with (cache/('carpeta-'+item['id']+'.log')).open('a') as log:
            subprocess.Popen(['umu-run', str(value)], cwd=value.parent, env=env,
                             stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
        return


def main():
    action = sys.argv[1] if len(sys.argv) > 1 else 'menu'
    if action == 'preparar':
        prepare()
    elif action == 'export':
        print(json.dumps(user_config(), ensure_ascii=True))
    elif action == 'estado':
        user_config()
        status()
    elif action == 'menu':
        ui()
    else:
        raise ValueError('accion desconocida')


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print('BIBLIOTECAS:', error, file=sys.stderr)
        if os.geteuid() == 1000 and os.environ.get('WAYLAND_DISPLAY'):
            subprocess.run(['notify-send', 'Bibliotecas: revisar', str(error)], check=False)
        sys.exit(1)
