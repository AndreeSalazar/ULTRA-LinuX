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
import shutil
import stat
import subprocess
import sys
import tempfile
import time

HOME = Path('/juegos/usuarios/ultra')
CONFIG = HOME / '.config/ultra/bibliotecas.json'
CATALOG = Path('/run/ultra-bibliotecas.json')
VOLUMES = Path('/discos')
TARGETS = Path('/juegos/Bibliotecas')
PRIVATE = Path('/juegos/.ultra-bibliotecas')
LOWERS = Path('/run/ultra-bibliotecas/fuentes')
LIMIT = 262144
MAX_PREVIOUS_STORES = 3
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


def discover_selections(items, volumes):
    """Bounded search inside explicitly selected folders, as the desktop user."""
    result = {item['id']: item for item in items}
    visited = 0
    for item in items:
        if item['kind'] == 'steam':
            continue
        volume = next((v for v in volumes if v['uuid'] == item['uuid'] and v['status'] == 'lectura'), None)
        if volume is None:
            continue
        base = Path(volume['path'])
        pending = [(base/item['relative'], 0)]
        while pending and visited < 2048 and len(result) < 128:
            folder, depth = pending.pop()
            visited += 1
            try:
                if folder.is_symlink():
                    continue
                apps = folder/'steamapps'
                if apps.is_dir() and not apps.is_symlink():
                    found = selection(dict(uuid=item['uuid'], relative=folder.relative_to(base).as_posix(), kind='steam'))
                    result[found['id']] = found
                    continue
                if depth < 3:
                    with os.scandir(folder) as entries:
                        for index, entry in enumerate(entries):
                            if index >= 512:
                                break
                            if (len(pending) < 2048 and entry.is_dir(follow_symlinks=False)
                                    and not entry.name.startswith('.')):
                                pending.append((Path(entry.path), depth+1))
            except OSError:
                continue
    return list(result.values())


def rotate_changed_store(store, identity):
    """Never combine an old writable overlay with a changed Windows lower."""
    if not (store.exists() or store.is_symlink()):
        return
    checked_dir(store, 0o700)
    state = read_json(store/'source.json')
    if state == identity:
        return
    if ({k: v for k, v in state.items() if k != 'version'} !=
            {k: v for k, v in identity.items() if k != 'version'}):
        raise ValueError('identidad de origen cambio; revisar la capa anterior')
    # Keep a recoverable layer, but never accumulate unlimited generations.
    previous = list(store.parent.glob(store.name+'.anterior-*'))
    if len(previous) >= MAX_PREVIOUS_STORES:
        raise ValueError(f'limite de {MAX_PREVIOUS_STORES} capas anteriores conservadas; '
                         'archivar y revisar desde Discos antes de renovar; no se borro el juego')
    free = shutil.disk_usage(store.parent).free
    if free < 4*1024**3:
        raise ValueError('menos de 4 GiB libres; no se crea otra capa de biblioteca')
    # A refreshed lower can force another copy of the writable game files.
    # Account allocated blocks once, without following user-controlled symlinks.
    try:
        previous_bytes = int(run('/usr/bin/du', '-sx', '-B1', str(store/'upper')).split()[0])
    except (ValueError, IndexError, subprocess.SubprocessError) as error:
        raise ValueError('no se pudo medir la capa; se conserva para revision') from error
    if free < previous_bytes + 4*1024**3:
        raise ValueError('espacio insuficiente para renovar sin duplicar la biblioteca; revisar la capa existente')
    # Same parent, no recursive traversal and no deletion of saves or prefixes.
    archive = store.with_name(store.name+'.anterior-'+str(time.time_ns()))
    store.rename(archive)
    print('CAPA CONSERVADA:', archive.name, '; origen actualizado, se crea una vista limpia')
    return archive


def retain_linux_state(archive, store):
    """Move Linux-only state into a fresh generation; never carry game overlays.

    All traversal uses pinned directories and no-follow opens. Runtime manifests
    are bounded data, never commands. Called before the new view is exposed.
    """
    flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW
    opened = []
    def directory(parent, name):
        fd = os.open(name, flags, dir_fd=parent)
        opened.append(fd)
        s = os.fstat(fd)
        if s.st_uid != 1000 or s.st_mode & 0o022:
            raise ValueError('estado Linux con permisos inesperados')
        return fd
    def absent(fd, name):
        try: os.stat(name, dir_fd=fd, follow_symlinks=False)
        except FileNotFoundError: return
        raise ValueError('destino Linux ya ocupado; se conserva')
    try:
        for path in (archive, store):
            checked_dir(path, 0o700)
        old_root = os.open(archive, flags); opened.append(old_root)
        new_root = os.open(store, flags); opened.append(new_root)
        old_upper = directory(old_root, 'upper'); new_upper = directory(new_root, 'upper')
        old_apps = directory(old_upper, 'steamapps'); new_apps = directory(new_upper, 'steamapps')
        plans = []
        for name in ('compatdata', 'shadercache'):
            try: fd = directory(old_apps, name)
            except FileNotFoundError: continue
            absent(new_apps, name)
            plans.append((name, fd))
        runtimes = []
        old_common = None
        names = []
        with os.scandir(old_apps) as entries:
            for entry in entries:
                if len(names) >= 2048: raise ValueError('demasiados metadatos en la capa Linux')
                names.append(entry.name)
        for name in sorted(names):
            if not re.fullmatch(r'appmanifest_[0-9]{1,10}\.acf', name): continue
            fd = os.open(name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=old_apps)
            with os.fdopen(fd, 'rb') as stream:
                info = os.fstat(stream.fileno())
                if not stat.S_ISREG(info.st_mode) or info.st_uid != 1000:
                    raise ValueError('manifiesto Linux no regular o propietario distinto')
                raw = stream.read(LIMIT+1)
            if len(raw) > LIMIT: raise ValueError('manifiesto Linux demasiado grande')
            row = parse_vdf(raw.decode('utf-8-sig')).get('AppState', {})
            title = row.get('name', '') if isinstance(row, dict) else ''
            if not isinstance(title, str) or not title.startswith(('Proton', 'Steam Linux Runtime')): continue
            appid, folder = row.get('appid', ''), row.get('installdir', '')
            if (not isinstance(appid, str) or name != f'appmanifest_{appid}.acf'
                    or not appid.isdecimal() or not str(row.get('StateFlags', '')).isdecimal()
                    or not int(row['StateFlags']) & 4 or not str(row.get('buildid', '')).isdecimal()
                    or int(row['buildid']) == 0): continue
            if (not isinstance(folder, str) or not folder or folder in ('.', '..')
                    or any(c in folder for c in '/\\\0\n\r')):
                raise ValueError('ruta de runtime invalida')
            if old_common is None: old_common = directory(old_apps, 'common')
            fd = directory(old_common, folder)
            absent(new_apps, name)
            runtimes.append((name, folder, fd, raw))
        new_common = None
        if runtimes:
            os.mkdir('common', 0o755, dir_fd=new_apps)
            os.chown('common', 1000, 1000, dir_fd=new_apps, follow_symlinks=False)
            new_common = directory(new_apps, 'common')
        def move(name, src, dst, fd):
            absent(dst, name)
            expected = os.fstat(fd)
            current = os.stat(name, dir_fd=src, follow_symlinks=False)
            if (current.st_dev, current.st_ino) != (expected.st_dev, expected.st_ino):
                raise ValueError('estado Linux cambio durante la preparacion')
            os.rename(name, name, src_dir_fd=src, dst_dir_fd=dst)
            current = os.stat(name, dir_fd=dst, follow_symlinks=False)
            if (current.st_dev, current.st_ino) != (expected.st_dev, expected.st_ino):
                raise ValueError('estado Linux redirigido; revisar sin borrar')
        for name, fd in plans:
            os.setxattr(fd, 'trusted.overlay.opaque', b'y')
            move(name, old_apps, new_apps, fd)
        for name, folder, fd, raw in runtimes:
            move(folder, old_common, new_common, fd)
            target = os.open(name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600, dir_fd=new_apps)
            with os.fdopen(target, 'wb') as stream:
                os.fchown(stream.fileno(), 1000, 1000)
                stream.write(raw); stream.flush(); os.fsync(stream.fileno())
        os.fsync(new_apps)
        print('ESTADO LINUX CONSERVADO:', len(plans), 'directorios de prefijos/cache;', len(runtimes), 'runtimes')
    finally:
        for fd in reversed(opened): os.close(fd)


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
                for candidate in (Path('/windows'), Path('/run/ultra-juegos/volumen'), Path('/run/ultra-juegos/originales')):
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
        previous = rotate_changed_store(store, identity)
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
                if previous is not None:
                    retain_linux_state(previous, store)
                for child in ('compatdata', 'shadercache'):
                    directory = apps/child
                    if directory.exists(): continue
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
                      '/usr/local/libexec/ultra-bibliotecas.py', 'export-descubrir', input=json.dumps(volumes))
            if len(raw.encode()) > LIMIT:
                raise ValueError('seleccion demasiado grande')
            items = [selection(value) for value in json.loads(raw)]
            if len(items) > 128:
                raise ValueError('demasiadas carpetas descubiertas')
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


def read_vdf(path):
    """Bounded KeyValues reader; reject ambiguous input instead of losing fields."""
    with path.open('r', encoding='utf-8-sig') as stream:
        raw = stream.read(LIMIT+1)
    if len(raw.encode()) > LIMIT:
        raise ValueError('metadatos Steam demasiado grandes')
    return parse_vdf(raw)


def parse_vdf(raw):
    if len(raw.encode()) > LIMIT:
        raise ValueError('metadatos Steam demasiado grandes')
    tokens, pos = [], 0
    pattern = re.compile(r'\s+|//[^\n]*|"((?:\\.|[^"\\])*)"|([{}])')
    while pos < len(raw):
        match = pattern.match(raw, pos)
        if not match:
            raise ValueError('formato Steam desconocido; se conserva el original')
        pos = match.end()
        if match.group(1) is not None:
            tokens.append(('text', re.sub(r'\\([\\"])', r'\1', match.group(1))))
        elif match.group(2):
            tokens.append((match.group(2), ''))
    index = 0
    def obj(depth=0):
        nonlocal index
        if depth > 16:
            raise ValueError('metadatos Steam demasiado anidados')
        result = {}
        while index < len(tokens) and tokens[index][0] != '}':
            token, key = tokens[index]; index += 1
            if token != 'text' or key in result or index >= len(tokens):
                raise ValueError('metadatos Steam ambiguos')
            token, value = tokens[index]; index += 1
            if token == '{':
                value = obj(depth+1)
                if index >= len(tokens) or tokens[index][0] != '}':
                    raise ValueError('metadatos Steam incompletos')
                index += 1
            elif token != 'text':
                raise ValueError('metadatos Steam invalidos')
            result[key] = value
        return result
    value = obj()
    if index != len(tokens):
        raise ValueError('metadatos Steam con cierre inesperado')
    return value


def write_vdf(value, depth=0):
    def quote(text):
        return '"'+text.replace('\\', '\\\\').replace('"', '\\"')+'"'
    lines = []
    for key, item in value.items():
        prefix = '\t'*depth+quote(key)
        if isinstance(item, dict):
            lines.extend([prefix, '\t'*depth+'{', write_vdf(item, depth+1).rstrip('\n'), '\t'*depth+'}'])
        else:
            lines.append(prefix+'\t\t'+quote(item))
    return '\n'.join(lines)+'\n'


def find_steam_app(appid):
    user_config()
    if not re.fullmatch(r'[0-9]{1,10}', appid):
        raise ValueError('AppID invalido')
    libraries = []
    for item in read_json(CATALOG).get('folders', []):
        if item.get('kind') == 'steam' and item.get('status') == 'activa':
            expected = selection(item)
            library = TARGETS/expected['id']
            if item.get('path') == str(library):
                libraries.append(library)
    libraries.append(HOME/'Juegos/SteamLibrary')
    for library in libraries:
        manifest = library/'steamapps'/f'appmanifest_{appid}.acf'
        if not manifest.is_file() or manifest.is_symlink():
            continue
        state = read_vdf(manifest).get('AppState', {})
        if not isinstance(state, dict) or state.get('appid') != appid:
            continue
        install = state.get('installdir', '')
        if (not isinstance(install, str) or not install or '/' in install or '\\' in install
                or install in ('.', '..')):
            continue
        if (library/'steamapps/common'/install).is_dir():
            print(str(library))
            return
    raise ValueError('juego sin manifiesto y carpeta en las bibliotecas activas; revisa el selector de discos')


class SteamRegistryBusy(ValueError):
    """Otra tarea puede estar escribiendo VDF; no lanzar un cliente nuevo aun."""


def register_steam():
    import fcntl
    user_config()
    # El lanzador no debe esperar al registro de otra tarea si Steam ya existe.
    # La comprobacion se repite bajo el bloqueo para conservar la proteccion
    # frente a un Steam que arranque entre ambas operaciones.
    if steam_is_running():
        print('STEAM: ya abierto; se conservan sus bibliotecas sin esperar al registro')
        return False
    lockdir = HOME / '.cache/ultra'
    lockdir.mkdir(mode=0o700, parents=True, exist_ok=True)
    with (lockdir/'registro-steam.lock').open('a') as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise SteamRegistryBusy('registro Steam ocupado por otra revision; espera a que termine y vuelve a abrir') from None
        return _register_steam()


def steam_is_running():
    active = subprocess.run(['pgrep', '-u', '1000', '-x', 'steam'], stdout=subprocess.DEVNULL)
    if active.returncode not in (0, 1):
        raise ValueError('no se pudo comprobar si Steam esta cerrado')
    return active.returncode == 0


def _register_steam():
    if steam_is_running():
        print('STEAM: ya abierto; cierra Steam completamente para registrar nuevas bibliotecas')
        return False
    catalog = read_json(CATALOG)
    unavailable = sum(item.get('kind') == 'steam' and item.get('status') != 'activa'
                      for item in catalog.get('folders', []))
    if unavailable:
        print(f'STEAM: {unavailable} bibliotecas seleccionadas no estan montadas; '
              'Instalar no demuestra que se borraron los juegos. Revisa Discos y carpetas.', file=sys.stderr)
        if os.geteuid() == 1000 and os.environ.get('WAYLAND_DISPLAY'):
            try:
                subprocess.run(['notify-send', 'Steam: biblioteca no disponible',
                                'Hay una biblioteca seleccionada sin montar. Revisa Discos y carpetas antes de reinstalar.'],
                               timeout=3, check=False)
            except (OSError, subprocess.SubprocessError):
                pass
    root = HOME/'.local/share/Steam'
    root.mkdir(parents=True, exist_ok=True)
    canonical, legacy = root/'config/libraryfolders.vdf', root/'steamapps/libraryfolders.vdf'
    # Current clients keep a config copy as well as the steamapps copy. Updating
    # only one can be undone by Steam at startup. Preserve each existing record.
    for path, fallback in ((canonical, legacy), (legacy, canonical)):
        path.parent.mkdir(exist_ok=True)
        register_steam_file(path, fallback, root, catalog)
    return True


def register_steam_file(path, fallback, root, catalog):
    try:
        with path.open('rb') as stream:
            original = stream.read(LIMIT+1)
        if len(original) > LIMIT:
            raise ValueError('registro Steam demasiado grande; se conserva')
        data = read_vdf(path)
    except FileNotFoundError:
        original = None
        if fallback.exists():
            data = read_vdf(fallback)
        else:
            data = {'libraryfolders': {'0': {'path': str(root), 'label': '', 'apps': {}}}}
    libraries = data.get('libraryfolders')
    if not isinstance(libraries, dict) or any(not k.isdecimal() or not isinstance(v, dict)
                                              or not isinstance(v.get('path'), str)
                                              for k, v in libraries.items()):
        raise ValueError('registro Steam desconocido; no se modifica')
    added, changed = 0, False
    for item in catalog.get('folders', []):
        if item.get('kind') != 'steam' or item.get('status') != 'activa':
            continue
        expected = selection(item)
        library = TARGETS/expected['id']
        if item.get('path') != str(library) or not (library/'steamapps').is_dir():
            raise ValueError('biblioteca preparada no valida')
        apps, names = {}, []
        manifests = sorted((library/'steamapps').glob('appmanifest_*.acf'))
        if len(manifests) > 512:
            raise ValueError('demasiados manifiestos Steam')
        for manifest in manifests:
            if manifest.is_symlink():
                continue
            state = read_vdf(manifest).get('AppState', {})
            if not isinstance(state, dict):
                raise ValueError('manifiesto Steam invalido')
            appid, size = state.get('appid', ''), state.get('SizeOnDisk', '')
            if (not isinstance(appid, str) or not isinstance(size, str) or not appid.isdecimal()
                    or manifest.name != f'appmanifest_{appid}.acf' or not size.isdecimal()
                    or not str(state.get('StateFlags', '')).isdecimal()
                    or not (int(state['StateFlags']) & 4)):
                continue
            install = state.get('installdir', '')
            if not isinstance(install, str) or not install or '/' in install or '\\' in install or install in ('.', '..'):
                continue
            if not (library/'steamapps/common'/install).is_dir():
                continue
            apps[appid] = size
            names.append(str(state.get('name', appid)))
        existing = next((v for v in libraries.values() if v['path'] == str(library)), None)
        if existing is None:
            index = str(max((int(k) for k in libraries), default=-1)+1)
            libraries[index] = {'path': str(library), 'label': 'ULTRA / '+Path(item['relative']).name,
                                'contentid': str(int(item['id'][:16], 16) & ((1 << 63)-1)),
                                'totalsize': '0', 'update_clean_bytes_tally': '0',
                                'time_last_update_verified': '0', 'apps': apps}
            added += 1
            changed = True
        else:
            installed = existing.setdefault('apps', {})
            if not isinstance(installed, dict):
                raise ValueError('indice de juegos Steam desconocido; se conserva')
            for appid, size in apps.items():
                if appid not in installed:
                    installed[appid] = size
                    changed = True
        print('STEAM: biblioteca '+item['relative']+'; juegos con manifiesto instalado: '+str(len(apps)))
        for name in names:
            print('  '+name.replace('\n', ' '))
    if changed or (original is None and libraries):
        # Only Linux library metadata. Never import Windows accounts or sessions.
        if original is not None:
            backup = path.with_name('libraryfolders.vdf.ultra-anterior')
            backup.write_bytes(original)
            backup.chmod(0o600)
        fd, temp = tempfile.mkstemp(prefix='.ultra-', dir=path.parent)
        try:
            with os.fdopen(fd, 'w', encoding='utf-8') as stream:
                stream.write(write_vdf(data)); stream.flush(); os.fsync(stream.fileno())
            os.replace(temp, path)
        finally:
            if os.path.exists(temp):
                os.unlink(temp)
    print(f'STEAM: {added} bibliotecas nuevas registradas; Steam verificara los archivos y la compatibilidad')


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
    cursor = CONFIG.with_name('navegacion.json')
    try:
        saved = selection(read_json(cursor))
    except (OSError, ValueError, TypeError):
        saved = None
    resume = next((v for _, v in rows if saved and v['uuid']==saved['uuid']
                   and (Path(v['path'])/saved['relative']).is_dir()), None)
    if resume:
        rows.insert(0, ('Continuar en '+saved['relative'], dict(resume, resume=saved['relative'])))
    volume = choose('Disco', rows)
    if volume is None:
        return None
    relative = Path(volume.get('resume', ''))
    while True:
        folder = Path(volume['path'])/relative
        if relative != Path():
            CONFIG.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
            atomic_json(cursor, selection(dict(uuid=volume['uuid'], relative=relative.as_posix(), kind='folder')))
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
        print('  Carpeta preparada:', active.get('path', str(TARGETS/item['id'])))
        if item['kind'] == 'steam':
            print('  Abre Steam desde Super+J para registrar esta biblioteca automaticamente (Steam debe estar cerrado).')
    selected_ids = {item['id'] for item in user_config()}
    for item in catalog.get('folders', []):
        if item['id'] not in selected_ids:
            print('Descubierta dentro de tu seleccion:', item['relative'], item['status'])
            print('  Carpeta preparada:', item.get('path', 'no disponible'))
    print('GOG: usa Importar juegos GOG; en Heroic elige el juego y su carpeta preparada.')
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
        run('/usr/bin/notify-send', 'Seleccion guardada / pendiente de activar', 'Tu carpeta ya se recuerda. Guarda tu trabajo, cierra el escritorio y vuelve a entrar desde e. Al regresar se preparan los montajes y se revisa Steam automaticamente. Consulta Super+Espacio > Discos y bibliotecas.')
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
        env = dict(os.environ, WINEPREFIX=str(prefix), GAMEID='umu-0', PROTONPATH='GE-Proton',
                   ULTRA_GAME_ID='carpeta:'+item['id'], ULTRA_GAME_NAME=item['relative'])
        with (cache/('carpeta-'+item['id']+'.log')).open('a') as log:
            subprocess.Popen(['/usr/local/bin/ultra-hud', 'opengl', 'umu-run', str(value)], cwd=value.parent, env=env,
                             stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
        return


def import_gog():
    user_config()
    active = [r for r in read_json(CATALOG).get('folders', [])
              if r.get('status') == 'activa' and r.get('kind') == 'folder']
    cyberpunk = Path('/juegos/Cyberpunk2077')
    ready = (cyberpunk/'goggame-1423049311.info').is_file() and (cyberpunk/'bin/x64/Cyberpunk2077.exe').is_file()
    if not active and not ready:
        raise ValueError('Elige primero la carpeta GOG en Discos y bibliotecas y vuelve a entrar al escritorio.')
    rows = [('Cyberpunk 2077 / GOG / disco Personal', 'cyberpunk-gog')] if ready else []
    rows.extend((r['relative'], r) for r in active)
    item = choose('Carpeta para importar en Heronite', rows)
    if item is None:
        return
    if item == 'cyberpunk-gog':
        subprocess.Popen(['/usr/local/bin/ultra-juegos', 'gog'])
        return
    expected = TARGETS/selection(item)['id']
    if item.get('path') != str(expected) or not expected.is_dir():
        raise ValueError('carpeta preparada no valida')
    # Heroic owns its import format/account catalogue; do not invent installs.
    subprocess.Popen(['thunar', str(expected)])
    subprocess.run(['notify-send', 'GOG / importar archivos existentes',
                    'En Heronite: selecciona el juego de tu cuenta e importa la carpeta abierta: '+str(expected)], check=False)
    subprocess.Popen(['/usr/local/bin/ultra-heronite', 'biblioteca'],
                     stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def main():
    action = sys.argv[1] if len(sys.argv) > 1 else 'menu'
    if action == 'preparar':
        prepare()
    elif action == 'export':
        print(json.dumps(user_config(), ensure_ascii=True))
    elif action == 'export-descubrir':
        items = user_config()
        # Root supplies read-only volume metadata, not executable instructions.
        raw = sys.stdin.read(LIMIT+1)
        if len(raw.encode()) > LIMIT:
            raise ValueError('catalogo demasiado grande')
        volumes = json.loads(raw) if raw.strip() else []
        if not isinstance(volumes, list) or len(volumes) > 128:
            raise ValueError('catalogo invalido')
        volumes = [v for v in volumes if isinstance(v, dict) and isinstance(v.get('uuid'), str)
                   and UUID.fullmatch(v['uuid']) and v.get('path') == str(VOLUMES/v['uuid'])]
        print(json.dumps(discover_selections(items, volumes), ensure_ascii=True))
    elif action == 'estado':
        user_config()
        status()
    elif action == 'menu':
        ui()
    elif action == 'registrar-steam':
        register_steam()
    elif action == 'importar-gog':
        import_gog()
    elif action == 'buscar-steam' and len(sys.argv) == 3:
        find_steam_app(sys.argv[2])
    else:
        raise ValueError('accion desconocida')


if __name__ == '__main__':
    try:
        main()
    except SteamRegistryBusy as error:
        print('BIBLIOTECAS:', error, file=sys.stderr)
        sys.exit(75)  # EX_TEMPFAIL: el lanzador debe evitar una carrera con VDF.
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print('BIBLIOTECAS:', error, file=sys.stderr)
        if os.geteuid() == 1000 and os.environ.get('WAYLAND_DISPLAY'):
            subprocess.run(['notify-send', 'Bibliotecas: revisar', str(error)], check=False)
        sys.exit(1)
