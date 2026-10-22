#!/usr/bin/env python3
"""Boot ULTRA with USB audio recorded to WAV and a disposable NTFS game fixture.
No host drives, user accounts or actual game binaries are exposed to QEMU.
"""
import hashlib
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
import wave
import struct


def main():
    profile = sys.argv[1]
    preflight = '--prevalidar' in sys.argv
    storage_c = '--almacen-c' in sys.argv
    dependencies = '--deps-steam' in sys.argv
    assert not dependencies or preflight
    reuse = '--reanudar' in sys.argv or '--diagnostico' in sys.argv
    diagnose = '--diagnostico' in sys.argv
    warframe_test = '--warframe' in sys.argv
    libraries_test = '--bibliotecas' in sys.argv
    library_client_test = '--cliente-steam' in sys.argv
    warm_udev = '--udev-temprano' in sys.argv
    assert not warframe_test or not (preflight or dependencies), 'La prueba Warframe exige ayudantes nativos, sin inyeccion'
    assert not libraries_test or not (preflight or dependencies or warframe_test)
    windows_test = '--windows' in sys.argv or warframe_test or libraries_test
    graphics_bank = '--banco-grafica' in sys.argv
    sys.argv = [sys.argv[0], profile, '--bmo']
    spec = importlib.util.spec_from_file_location('arranque', Path(__file__).with_name('probar_arranque.py'))
    base = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(base)
    serial_port = 15755 if profile == 'generico' else 15855
    base.check_kernel()
    lab = base.ROOT / ('build/juegos-grafica' if graphics_bank else 'build/juegos')
    dependencies_digest = None
    if dependencies:
        with (base.ROOT/'vm/public/deps-steam.tar').open('rb') as stream:
            dependencies_digest = hashlib.file_digest(stream, 'sha256').hexdigest()
    config = json.loads((lab/'ntfs-lab.json').read_text())
    windows_original = base.ROOT / ('build/warframe/windows.img' if warframe_test or libraries_test else 'build/juegos/windows.img')
    windows_copy = base.BUILD / 'windows-prueba.img'
    windows_before = None
    if windows_test:
        with windows_original.open('rb') as stream:
            windows_before = hashlib.file_digest(stream, 'sha256').hexdigest()
        shutil.copyfile(windows_original, windows_copy)
    assert re.fullmatch(r'[0-9A-Fa-f]{16}', config['uuid'])
    image = base.BUILD/'audio-juegos-bmo.img'
    games = base.BUILD/'audio-juegos-ntfs.img'
    variables = base.BUILD/'audio-juegos-vars.fd'
    capture = base.BUILD/'audio-usb.wav'
    shutil.copyfile(base.IMG, image)
    if not reuse or not games.exists(): shutil.copyfile(lab/'ntfs-lab.img', games)
    shutil.copyfile(base.QEMU_SHARE/'edk2-i386-vars.fd', variables)
    capture.unlink(missing_ok=True)
    transcript = ''
    results = []
    counter = 0
    process = None
    console = None
    monitor = None

    def start():
        nonlocal process, console, monitor, transcript
        process = subprocess.Popen([str(base.QEMU), '-machine', 'pc', '-accel', 'whpx', '-cpu', 'host',
            '-smp', '4', '-m', '4096', '-display', 'none', '-no-reboot',
            '-drive', f'if=pflash,format=raw,readonly=on,file={base.QEMU_SHARE/"edk2-x86_64-code.fd"}',
            '-drive', f'if=pflash,format=raw,file={variables}',
            '-drive', f'file={image},format=raw,if=virtio',
            '-drive', f'file={games},format=raw,if=virtio',
            *(['-drive', f'file={windows_copy},format=raw,if=none,id=windows',
               '-device', 'nvme,drive=windows,serial=ULTRA-WINDOWS-LAB'] if windows_test else []),
            '-netdev', 'user,id=net0', '-device', 'virtio-net-pci,netdev=net0',
            '-device', 'virtio-vga', '-device', 'qemu-xhci,id=usb0',
            '-audiodev', f'wav,id=sound0,path={capture}',
            '-device', 'usb-audio,id=usbaudio0,bus=usb0.0,audiodev=sound0',
            '-serial', f'tcp:127.0.0.1:{serial_port},server=on,wait=off',
            '-monitor', f'tcp:127.0.0.1:{serial_port+1},server=on,wait=off'],
            stdout=subprocess.DEVNULL, stderr=(base.BUILD/'audio-juegos-qemu-error.txt').open('w'))
        for _ in range(80):
            try:
                console = socket.create_connection(('127.0.0.1', serial_port), timeout=1)
                monitor = socket.create_connection(('127.0.0.1', serial_port+1), timeout=1)
                break
            except OSError:
                if process.poll() is not None: raise RuntimeError('QEMU no arranco')
                time.sleep(.2)
        else: raise ConnectionError('Sin consola QEMU')
        console.settimeout(.5); monitor.settimeout(.5)
        transcript += base.read_until(console, 'ultra:', 120)

    def command(text, timeout=30):
        nonlocal counter, transcript
        counter += 1
        marker = f'AUDIO-JUEGOS-FIN-{200+counter}'
        separator = ' ' if text.rstrip().endswith('&') else '; '
        console.sendall((text+separator+f'echo AUDIO-JUEGOS-FIN-$((200+{counter}))\n').encode())
        try: output = base.read_until(console, marker, timeout)
        except base.Incomplete as error:
            transcript += error.partial
            raise
        transcript += output
        return output

    def check(ok, label):
        print(('VERDE' if ok else 'ROJO'), label, flush=True)
        if not ok: raise AssertionError(label)
        results.append(label)

    def setup():
        if preflight:
            for name in ('ultra-preparar-juegos', 'ultra-sesion', 'ultra-preparar-red'):
                source=base.ROOT/'src/rootfs/bin'/name
                encoded=base64.b64encode(source.read_bytes()).decode()
                command(': > /arch/tmp/juegos-helper.b64')
                for i in range(0,len(encoded),256):
                    command(f"printf '%s' '{encoded[i:i+256]}' >> /arch/tmp/juegos-helper.b64")
                command(f'busybox base64 -d /arch/tmp/juegos-helper.b64 > /arch/usr/local/libexec/{name}')
        if dependencies:
            command('ultra-entrar /usr/bin/bash /usr/local/libexec/ultra-preparar-red', 45)
            out=command('ultra-entrar /usr/bin/curl --fail --silent --max-time 30 http://10.0.2.2:18765/deps-steam.tar -o /tmp/deps-steam.tar; ultra-entrar /usr/bin/sha256sum /tmp/deps-steam.tar')
            assert dependencies_digest in out
            command('busybox tar -xf /arch/tmp/deps-steam.tar -C /arch')
        if storage_c:
            command("ultra-entrar /usr/bin/mkdir -p /mnt/almacen-c; ultra-entrar /usr/bin/mount -t ntfs3 -o rw /dev/vdb /mnt/almacen-c")
            command("ultra-entrar /usr/bin/mkdir -p '/mnt/almacen-c/Espacio para Linux'; ultra-entrar /usr/bin/bash -c '[[ ! -f /mnt/almacen-c/ULTRA-juegos.ext4 ]] || mv /mnt/almacen-c/ULTRA-juegos.ext4 \"/mnt/almacen-c/Espacio para Linux/ULTRA-juegos.ext4\"'; ultra-entrar /usr/bin/umount /mnt/almacen-c")
            config['image_path'] = 'Espacio para Linux/ULTRA-juegos.ext4'
            command(f"printf 'uuid={config['uuid']}\\n' > /arch/etc/ultra/windows.conf")
        content = ''.join(f'{k}={v}\\n' for k, v in config.items())
        command(f"printf '{content}' > /arch/etc/ultra/juegos.conf")
        out = command('ultra-entrar /usr/bin/bash /usr/local/libexec/ultra-preparar-juegos; echo MONTAJE-RC-$?', 40)
        check('MONTAJE-RC-0' in out, 'NTFS identificado y ext4 montado sin formatear')
        if storage_c:
            out = command('ultra-entrar /usr/bin/bash /usr/local/libexec/ultra-preparar-windows; echo WINDOWS-C-RC-$?')
            check('WINDOWS-C-RC-0' in out, 'almacen en subcarpeta de C: y vista Windows preparada')
            out = command(user+"/usr/bin/bash -c 'if touch /windows/prueba-prohibida; then exit 1; fi; echo C-PUBLICO-DENEGADO'")
            check('C-PUBLICO-DENEGADO' in out and 'Read-only file system' in out, 'vista C: rechaza escritura con contenedor ext4 activo')
            out = command(user+"/usr/bin/bash -c 'echo C-EXT4-ESCRITURA > /juegos/usuarios/ultra/c-ext4.txt; cat /juegos/usuarios/ultra/c-ext4.txt'")
            check('C-EXT4-ESCRITURA' in out, 'archivo ext4 sigue escribible sin desproteger la vista Windows')
        if windows_test:
            out = command('ultra-entrar /usr/bin/bash /usr/local/libexec/ultra-preparar-windows; echo WINDOWS-RC-$?')
            check('WINDOWS-RC-0' in out, 'NVMe Windows ficticio identificado por UUID y montado')
            out = command(user+'/usr/bin/cat /windows/prueba.txt; ultra-entrar /usr/bin/blockdev --getro /dev/nvme0n1')
            check('WINDOWS-LECTURA-LAB' in out and re.search(r'(?m)^1\r?$', out),
                  'Windows legible por UID1000 con dispositivo NVMe de solo lectura')
            out = command(user+"/usr/bin/bash -c 'if echo CAMBIO > /windows/prueba.txt; then exit 1; fi; echo ESCRITURA-DENEGADA'")
            check('ESCRITURA-DENEGADA' in out and 'Read-only file system' in out,
                  'Windows rechaza escrituras de aplicaciones')
            out = command("ultra-entrar /usr/bin/bash -c 'if printf X | dd of=/dev/nvme0n1 bs=1 count=1 conv=notrunc; then exit 1; fi; echo DISPOSITIVO-DENEGADO'")
            check('DISPOSITIVO-DENEGADO' in out and '0 bytes copied' in out
                  and any(message in out for message in ('Read-only file system', 'Operation not permitted')),
                  'NVMe ficticio rechaza escritura directa incluso del preparador root')

    user = ('ultra-entrar /usr/bin/setpriv --reuid=1000 --regid=1000 --init-groups '
            '--no-new-privs --bounding-set=-all /usr/bin/env XDG_RUNTIME_DIR=/run/user/1000 '
            'HOME=/juegos/usuarios/ultra ')
    ipc = 'ultra-entrar /usr/bin/env XDG_RUNTIME_DIR=/run/user/1000 /usr/bin/hyprctl -i 0 '

    def clients():
        out = command(ipc+'-j clients')
        m = re.search(r'(?m)^\s*\[\s*(?:\{|\])', out)
        return json.JSONDecoder().raw_decode(out[m.start():].lstrip())[0] if m else []

    def wait_app(name, timeout=120):
        deadline = time.monotonic()+timeout
        while time.monotonic()<deadline:
            if any(name in a.get('class','').lower() for a in clients()): return True
            time.sleep(1)
        return False

    def wait_steam_login(timeout=600):
        deadline = time.monotonic()+timeout
        report_at = time.monotonic()+30
        paint_deadline = None
        while time.monotonic()<deadline:
            for app in clients():
                title = app.get('title','').lower()
                if 'transport error' in title:
                    return False
                if 'steam' in app.get('class','').lower() and any(s in title for s in ('sign in', 'iniciar sesi', 'steam login')):
                    # A CEF helper distinguishes the actual client from its updater.
                    out = command('pidof steamwebhelper || true')
                    if re.search(r'(?m)^\d+(?: \d+)*$', out.replace('\r', '')):
                        if paint_deadline is None: paint_deadline=time.monotonic()+60
                        from PIL import Image
                        snapshot('steam-login.png')
                        x,y=app['at']; w,h=app['size']
                        with Image.open(base.BUILD/'steam-login.png') as screen:
                            pixels=list(screen.convert('RGB').crop((x+12,y+12,x+w-12,y+h-12)).resize((40,25)).getdata())
                        if sum(max(pixel)>35 for pixel in pixels)>100 and len(set(pixels))>30:
                            return True
                        if time.monotonic()>paint_deadline: return False
            if time.monotonic() >= report_at:
                out = command('ultra-entrar /usr/bin/tail -n 3 /juegos/usuarios/ultra/.local/share/Steam/logs/console-linux.txt')
                print('STEAM:', out.replace('\r', '').strip()[-500:], flush=True)
                if 'No space left on device' in out:
                    return False
                report_at = time.monotonic()+30
            time.sleep(2)
        return False

    def snapshot(name):
        from PIL import Image
        path = base.BUILD/name
        path.unlink(missing_ok=True)
        monitor.sendall(f'screendump "{path.as_posix()}" -f png\n'.encode())
        for _ in range(80):
            try:
                with Image.open(path) as im: im.verify()
                return
            except (OSError, SyntaxError): time.sleep(.1)
        raise ValueError('Captura incompleta')

    try:
        start()
        if diagnose:
            command('ultra-entrar /usr/bin/mkdir -p /run/diagnostico-ntfs /run/diagnostico-ext4; ultra-entrar /usr/bin/mount -t ntfs3 -o ro /dev/vdb /run/diagnostico-ntfs; ultra-entrar /usr/bin/mount -t ext4 -o loop,ro,noload /run/diagnostico-ntfs/ULTRA-juegos.ext4 /run/diagnostico-ext4')
            command('ultra-entrar /usr/bin/bash -c \'ls /run/diagnostico-ext4/usuarios/ultra/.local/share/Steam/logs; tail -n 100 /run/diagnostico-ext4/usuarios/ultra/.cache/ultra/steam.log /run/diagnostico-ext4/usuarios/ultra/.local/share/Steam/logs/cef_log.txt /run/diagnostico-ext4/usuarios/ultra/.local/share/Steam/logs/webhelper-linux.txt /run/diagnostico-ext4/usuarios/ultra/.local/share/Steam/logs/console-linux.txt\'',60)
            console.sendall(b'poweroff\n');transcript+=base.read_until(console,'APAGADO',30);process.wait(timeout=20)
            return
        setup()
        if storage_c:
            assert not (preflight or dependencies or libraries_test or warframe_test or windows_test)
            hashes = {}
            for name in ('ultra-preparar-juegos', 'ultra-preparar-windows', 'ultra-bibliotecas.py', 'ultra-brave', 'ultra-juegos'):
                expected = hashlib.sha256((base.ROOT/'src/rootfs/bin'/name).read_bytes()).hexdigest()
                subdir = 'libexec' if name in ('ultra-preparar-juegos','ultra-preparar-windows','ultra-bibliotecas.py') else 'bin'
                out = command(f'ultra-entrar /usr/bin/sha256sum /usr/local/{subdir}/{name}')
                check(expected in out, 'fuente nativa coincide: '+name)
                hashes[name] = expected
            command('ultra-entrar /usr/bin/mkdir -p /run/brave-espacio; ultra-entrar /usr/bin/mount -t tmpfs -o size=128M,mode=0700,uid=1000,gid=1000 tmpfs /run/brave-espacio')
            out = command(user+'/usr/bin/env HOME=/run/brave-espacio WAYLAND_DISPLAY=prueba /usr/local/bin/ultra-brave; echo BRAVE-ESPACIO-RC-$?')
            check('ROJO [BRAVE]: quedan menos de 256 MiB' in out and 'BRAVE-ESPACIO-RC-1' in out, 'Brave detecta falta de espacio antes de abrir su perfil')
            command('ultra-entrar /usr/bin/umount /run/brave-espacio')
            command(user+"/usr/bin/bash -c 'echo PERSISTENCIA-C-NVME > \"$HOME/prueba-c.txt\"'")
            command('ultra-escritorio --hyprland --software > /tmp/almacen-c-sesion.log 2>&1 &')
            check(wait_app('foot',60), 'escritorio inicia con home del contenedor en C:')
            command(ipc+'eval \'hl.exec_cmd("ultra-brave")\'')
            check(wait_app('brave',90), 'Brave abre en Wayland con perfil persistente y espacio suficiente')
            snapshot('almacen-c-brave.png')
            console.sendall(b'reboot\n'); transcript += base.read_until(console,'Restarting system',40)
            process.wait(timeout=20); console.close(); monitor.close()
            check('VERDE [JUEGOS]: /arch/juegos' in transcript and 'VERDE [JUEGOS]: /arch/run/ultra-juegos/volumen' in transcript,
                  'PID1 desmonta ext4 y volumen NTFS del contenedor al reiniciar')
            start(); setup()
            out = command(user+'/usr/bin/cat /juegos/usuarios/ultra/prueba-c.txt')
            check('PERSISTENCIA-C-NVME' in out, 'datos del contenedor en C: sobreviven a un reinicio real')
            console.sendall(b'poweroff\n'); transcript += base.read_until(console,'APAGADO',40); process.wait(timeout=20)
            verified={Path(line.split()[1]).name:line.split()[0] for line in (base.BUILD/'SHA256SUMS').read_text().splitlines() if line.strip()}
            result={'perfil':profile,'pruebas':results,'disco_real_expuesto':False,'helpers_inyectados':False,
                    'imagen_sha256':verified[base.IMG.name],'fuentes_sha256':hashes,'almacen_c_ficticio':True}
            (base.BUILD/'almacen-c-resultados.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
            return
        if libraries_test:
            if warm_udev:
                command('ultra-entrar /usr/lib/systemd/systemd-udevd --daemon; ultra-entrar /usr/bin/udevadm trigger --action=add --type=devices; ultra-entrar /usr/bin/udevadm settle --timeout=15',30)
            def prepare_libraries():
                output = command('ultra-entrar /usr/bin/bash /usr/local/libexec/ultra-preparar-bibliotecas; echo BIBLIOTECAS-RC-$?', 60)
                check('BIBLIOTECAS-RC-0' in output, 'preparador general nativo termina sin inyectar ayudantes')
                match = re.search(r'(?m)^BIBLIOTECAS: (\{.*\})\r?$', output)
                assert match, output
                return json.loads(match[1])
            catalog = prepare_libraries()
            by_uuid = {v['uuid']:v for v in catalog['volumes'] if v['status']=='lectura'}
            c_uuid = 'D8BAB58CBAB567A0'
            if c_uuid not in by_uuid or config['uuid'] not in by_uuid:
                print(command('ultra-entrar /usr/bin/lsblk -J -p -o NAME,TYPE,FSTYPE,UUID,LABEL,PARTTYPE; ultra-entrar /usr/bin/blkid',30),flush=True)
            check(c_uuid in by_uuid and config['uuid'] in by_uuid, 'dos discos NTFS distintos disponibles en vistas de solo lectura')
            initial = [{'uuid':c_uuid,'relative':'Program Files (x86)/Steam','kind':'steam'}]
            encoded = base64.b64encode(json.dumps(initial).encode()).decode()
            code = ('from pathlib import Path;import base64;p=Path.home()/".config/ultra/bibliotecas.json";'
                    'p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(base64.b64decode("'+encoded+'"))')
            command(user+"/usr/bin/python -c '"+code+"'")
            hook = base64.b64encode(b'from pathlib import Path\np=Path.home()/".cache/ultra/site-bibliotecas-probe"\np.parent.mkdir(parents=True,exist_ok=True)\np.write_text("USER-SITE-RAN")\n').decode()
            hook_setup = ('from pathlib import Path;import site,base64;'
                          'p=Path(site.getusersitepackages());p.mkdir(parents=True,exist_ok=True);'
                          'f=p/"sitecustomize.py";b=base64.b64decode("'+hook+'");'
                          'assert not f.exists() or f.read_bytes()==b;'
                          'm=Path.home()/".cache/ultra/site-bibliotecas-probe";m.unlink(missing_ok=True);f.write_bytes(b)')
            command(user+"/usr/bin/python -I -c '"+hook_setup+"'")
            catalog = prepare_libraries()
            out = command('test ! -e /arch/juegos/usuarios/ultra/.cache/ultra/site-bibliotecas-probe && echo PYTHON-AISLADO-$((6*7))')
            check('PYTHON-AISLADO-42' in out, 'exportador privilegiado ignora modulos Python controlados por el usuario')
            command(user+"/usr/bin/python -I -c 'from pathlib import Path;import site;(Path(site.getusersitepackages())/\"sitecustomize.py\").unlink()'")
            steam_library = next(r for r in catalog['folders'] if r['kind']=='steam')
            check(steam_library['status']=='activa', 'biblioteca Steam seleccionada por UUID y carpeta; sin AppID fijo')
            steam_path = steam_library['path']
            # An installed synthetic app, not a real game or account. The older
            # Warframe fixture intentionally lacks installdir and is not indexed.
            record = '"AppState" { "appid" "123" "name" "ULTRA fixture" "StateFlags" "4" "SizeOnDisk" "0" "installdir" "Warframe" }'
            code = ('from pathlib import Path;p=Path('+json.dumps(steam_path)+')/"steamapps";'
                    '(p/"appmanifest_123.acf").write_text('+json.dumps(record)+')')
            # Base64 avoids shell interpretation of the fixture text.
            encoded = base64.b64encode(code.encode()).decode()
            command(user+"/usr/bin/python -I -c 'import base64;exec(base64.b64decode(\""+encoded+"\"))'")
            out = command(user+'/usr/local/bin/ultra-bibliotecas registrar-steam; echo REGISTRO-RC-$?')
            check('REGISTRO-RC-0' in out and 'biblioteca Program Files (x86)/Steam' in out,
                  'registro Steam nativo sin seleccionar rutas en su interfaz')
            code = ('import json;from pathlib import Path;'
                    'p=Path.home()/".local/share/Steam/steamapps/libraryfolders.vdf";'
                    's=p.read_text();assert '+json.dumps(steam_path)+' in s;print("REGISTRO-PRESENTE-42")')
            out = command(user+"/usr/bin/python -I -c '"+code+"'")
            check('REGISTRO-PRESENTE-42' in out, 'configuracion Linux de Steam contiene la biblioteca activa')
            out = command(user+'/usr/local/bin/ultra-bibliotecas registrar-steam')
            check('0 bibliotecas nuevas registradas' in out and 'ULTRA fixture' in out,
                  'registro idempotente encuentra juego sintetico por manifiesto y carpeta')
            code = ('import runpy;from pathlib import Path;'
                    'm=runpy.run_path("/usr/local/libexec/ultra-bibliotecas.py");'
                    'root=Path.home()/".local/share/Steam";'
                    'a=m["read_vdf"](root/"config/libraryfolders.vdf");'
                    'b=m["read_vdf"](root/"steamapps/libraryfolders.vdf");'
                    'assert all(any(v["path"]=='+json.dumps(steam_path)+' and "123" in v.get("apps",{}) '
                    'for v in d["libraryfolders"].values()) for d in (a,b));print("DOS-REGISTROS-42")')
            encoded = base64.b64encode(code.encode()).decode()
            out = command(user+"/usr/bin/python -I -c 'import base64;exec(base64.b64decode(\""+encoded+"\"))'")
            check('DOS-REGISTROS-42' in out, 'config y steamapps conservan el indice de la instalacion existente')
            out = command(user+f'/usr/bin/bash -c \'echo LINUX-PREFIJO > "{steam_path}/steamapps/compatdata/prueba.txt"; echo CACHE-LINUX > "{steam_path}/steamapps/shadercache/prueba.txt"; cat "{steam_path}/steamapps/compatdata/prueba.txt"\'')
            check('LINUX-PREFIJO' in out and 'Permission denied' not in out, 'Steam escribe prefijos y caches Linux en ext4')
            for volume in (by_uuid[c_uuid], by_uuid[config['uuid']]):
                out=command(user+f'/usr/bin/bash -c \'if echo X > "{volume["path"]}/no-escribir.txt"; then exit 1; fi; echo VISTA-PROTEGIDA-$((6*7))\'')
                check('VISTA-PROTEGIDA-42' in out and 'Read-only file system' in out, 'vista del disco '+volume['uuid']+' rechaza escrituras')
            out = command('ultra-entrar /usr/bin/blockdev --getro /dev/vdb')
            check(bool(re.search(r'(?m)^0\r?$', out)), 'D: privado conserva escritura para el ext4, su vista publica permanece solo lectura')
            command('ultra-escritorio --hyprland --software > /tmp/bibliotecas-sesion.log 2>&1 &')
            check(wait_app('foot', 60), 'escritorio con home persistente para el selector general')
            monitor.sendall(b'sendkey meta_l-j\n'); time.sleep(.8)
            snapshot('bibliotecas-juegos.png')
            monitor.sendall(b'sendkey down\n'); time.sleep(.2)
            monitor.sendall(b'sendkey ret\n')
            time.sleep(1)
            out=command('pidof fuzzel || true')
            check(bool(re.search(r'(?m)^\d+(?: \d+)*\r?$', out)), 'gestor de carpetas abre como usuario del escritorio')
            snapshot('bibliotecas-menu.png')
            monitor.sendall(b'sendkey ret\n'); time.sleep(.7)
            snapshot('bibliotecas-discos.png')
            visible=[v for v in catalog['volumes'] if v['status']=='lectura']
            index=next(i for i,v in enumerate(visible) if v['uuid']==config['uuid'])
            for _ in range(index):
                monitor.sendall(b'sendkey down\n'); time.sleep(.15)
            monitor.sendall(b'sendkey ret\n'); time.sleep(.7)
            # The disposable D: fixture has exactly one directory, Cyberpunk 2077.
            monitor.sendall(b'sendkey ret\n'); time.sleep(.7)
            snapshot('bibliotecas-carpeta.png')
            monitor.sendall(b'sendkey ret\n'); time.sleep(1)
            out=command(user+'/usr/local/bin/ultra-bibliotecas export')
            match=re.search(r'(?m)^(\[.*\])\r?$',out)
            selected=json.loads(match[1]) if match else []
            check(any(r['uuid']==config['uuid'] and r['relative']=='Cyberpunk 2077' and r['kind']=='folder' for r in selected),
                  'teclado elige carpeta en el segundo disco y guarda seleccion persistente')
            out=command('ultra-entrar /usr/bin/cat /run/ultra-bibliotecas.json')
            check('Cyberpunk 2077' not in out, 'seleccion nueva queda pendiente; no se monta en caliente desde la interfaz')
            if library_client_test:
                command(ipc+'eval \'hl.exec_cmd("ultra-juegos steam")\'')
                check(wait_steam_login(180), 'cliente Steam abre desde el integrador sin cuenta autenticada')
                command(ipc+'eval \'hl.exec_cmd("steam -shutdown")\'')
                deadline = time.monotonic()+30
                while time.monotonic() < deadline:
                    out = command(user+'/usr/bin/pgrep -x steam || true')
                    if not re.search(r'(?m)^\d+\r?$',out):
                        break
                    time.sleep(1)
                check(not re.search(r'(?m)^\d+\r?$',out), 'Steam cierra sin forzar terminacion')
                code = ('import runpy;from pathlib import Path;'
                        'm=runpy.run_path("/usr/local/libexec/ultra-bibliotecas.py");'
                        'p=Path.home()/".local/share/Steam/config/libraryfolders.vdf";'
                        'd=m["read_vdf"](p);assert any(v["path"]=='+json.dumps(steam_path)+' '
                        'for v in d["libraryfolders"].values());print("CLIENTE-CONSERVA-42")')
                encoded = base64.b64encode(code.encode()).decode()
                out = command(user+"/usr/bin/python -I -c 'import base64;exec(base64.b64decode(\""+encoded+"\"))'")
                check('CLIENTE-CONSERVA-42' in out, 'cliente real conserva biblioteca preparada tras abrir y cerrar')
            monitor.sendall(b'sendkey meta_l-shift-m\n');time.sleep(4)
            console.sendall(b'reboot\n')
            transcript += base.read_until(console,'Restarting system',40)
            process.wait(timeout=20);console.close();monitor.close()
            check('VERDE [BIBLIOTECAS]: /arch/juegos/Bibliotecas/' in transcript and 'BIBLIOTECAS-DESMONTAJE' not in transcript,
                  'PID1 desmonta las bibliotecas dinamicas antes del almacenamiento')
            start();setup();catalog=prepare_libraries()
            active=[r for r in catalog['folders'] if r['status']=='activa']
            check(len(active)==2 and {r['uuid'] for r in active}=={c_uuid,config['uuid']},
                  'tras reinicio se activan carpetas de ambos discos elegidos')
            game=next(r for r in active if r['kind']=='folder')
            out=command(user+f'/usr/bin/bash -c \'echo CARPETA-LINUX > "{game["path"]}/prueba-general.txt"; cat "{steam_path}/steamapps/compatdata/prueba.txt"\'')
            check('LINUX-PREFIJO' in out and 'Permission denied' not in out, 'prefijo Steam persiste y carpeta independiente admite escrituras aisladas')
            out=command('test ! -e "/arch/run/ultra-juegos/volumen/Cyberpunk 2077/prueba-general.txt" && cat "/arch/run/ultra-juegos/volumen/Cyberpunk 2077/bin/x64/Cyberpunk2077.exe"')
            check('ULTRA-LAB-GAME-ORIGINAL' in out, 'carpeta original del segundo disco permanece intacta')
            console.sendall(b'poweroff\n');transcript+=base.read_until(console,'APAGADO',30);process.wait(timeout=20)
            with windows_copy.open('rb') as stream: windows_after=hashlib.file_digest(stream,'sha256').hexdigest()
            check(windows_after==windows_before,'SHA256 completo del primer disco ficticio sin cambios')
            with base.IMG.open('rb') as stream:image_hash=hashlib.file_digest(stream,'sha256').hexdigest()
            result={'perfil':profile,'pruebas':results,'disco_real_expuesto':False,'helpers_inyectados':False,
                    'imagen_sha256':image_hash,'windows_ficticio_sha256_antes_despues':windows_after,
                    'seleccion_teclado':True,'juegos_reales_ejecutados':False,'udev_previamente_iniciado':warm_udev,
                    'almacen_sintetico_reutilizado':reuse}
            result['cliente_steam_sin_cuenta'] = library_client_test
            (base.BUILD/'bibliotecas-resultados.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
            return
        if warframe_test:
            # Focused native-image test: no injected helpers, real game or account.
            out = command('ultra-entrar /usr/bin/bash /usr/local/libexec/ultra-preparar-warframe; echo CAPA-RC-$?')
            check('CAPA-RC-0' in out, 'kernel ULTRA monta overlay Warframe sobre NTFS protegido')
            out = command(user+'/usr/local/bin/ultra-steam-warframe; echo BIBLIOTECA-RC-$?')
            check('BIBLIOTECA-RC-0' in out, 'ayudante nativo prepara biblioteca Steam como UID1000')
            out = command(user+"/usr/bin/bash -c 'set -e; echo CAMBIO-WARFRAME > /juegos/Warframe/Warframe.x64.exe; echo PERSISTE-WARFRAME > /juegos/Warframe/prueba.txt; echo CAPA-ESCRITA-$((6*7))'")
            check('CAPA-ESCRITA-42' in out, 'escrituras de Warframe se admiten en la capa ext4')
            original = '/windows/Program Files (x86)/Steam/steamapps/common/Warframe/Warframe.x64.exe'
            out = command(user+f'/usr/bin/cat "{original}"')
            check('WARFRAME-SINTETICO' in out, 'ejecutable original permanece intacto')
            out = command(user+"/usr/bin/bash -c 'readlink \"$HOME/Juegos/SteamLibrary/steamapps/common/Warframe\"; cat \"$HOME/Juegos/SteamLibrary/steamapps/appmanifest_230410.acf\"'")
            check('/juegos/Warframe' in out and '9876543210' in out and 'CUENTA-SINTETICA' not in out,
                  'metadatos publicos y enlace importados sin cuenta Windows')
            out = command(user+'/usr/local/bin/ultra-juegos estado', 45)
            check('[RAM:' in out and 'Mem:' in out and '[VRAM:' in out and '/juegos/Warframe' in out
                  and 'command not found' not in out,
                  'diagnostico nativo distingue discos, RAM, VRAM y biblioteca')
            command('ultra-entrar /usr/bin/install -m 0755 /usr/bin/sleep /tmp/steam')
            command(user+'/tmp/steam 60 &')
            out = command(user+'/usr/local/bin/ultra-steam-warframe; echo STEAM-ABIERTO-RC-$?')
            check('STEAM-ABIERTO-RC-1' in out and 'cierra Steam' in out,
                  'imagen nativa rechaza modificar la biblioteca con proceso Steam activo')
            command('ultra-entrar /usr/bin/pkill -u 1000 -x steam')
            console.sendall(b'reboot\n')
            transcript += base.read_until(console, 'Restarting system', 40)
            process.wait(timeout=20); console.close(); monitor.close()
            game_unmount = transcript.rfind('VERDE [JUEGOS]: Warframe desmontado')
            windows_unmount = transcript.rfind('VERDE [WINDOWS]: NVMe desmontado')
            check(0 <= game_unmount < windows_unmount, 'PID1 desmonta Warframe antes de Windows al reiniciar')
            start(); setup()
            out = command('ultra-entrar /usr/bin/bash /usr/local/libexec/ultra-preparar-warframe; '+user+'/usr/bin/cat /juegos/Warframe/prueba.txt /juegos/Warframe/Warframe.x64.exe')
            check('PERSISTE-WARFRAME' in out and 'CAMBIO-WARFRAME' in out, 'capa de Warframe persiste tras reinicio real')
            console.sendall(b'poweroff\n')
            transcript += base.read_until(console, 'APAGADO', 30)
            process.wait(timeout=20)
            with windows_copy.open('rb') as stream:
                windows_after = hashlib.file_digest(stream, 'sha256').hexdigest()
            check(windows_after == windows_before, 'SHA256 del NTFS sintetico completo inalterado en ambos arranques')
            with base.IMG.open('rb') as stream:
                image_hash = hashlib.file_digest(stream, 'sha256').hexdigest()
            result = {'perfil': profile, 'pruebas': results, 'disco_real_expuesto': False,
                      'imagen_sha256': image_hash, 'windows_ficticio_sha256_antes_despues': windows_after,
                      'juego_real_ejecutado': False, 'helpers_inyectados': False,
                      'almacen_sintetico_reutilizado': reuse}
            (base.BUILD/'warframe-boot-resultados.json').write_text(json.dumps(result, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
            return
        out = command(user+"/usr/bin/bash -c 'set -e; echo PERSISTENCIA-BMO > /juegos/usuarios/ultra/prueba.txt; echo ESCRITURA-OVERLAY > /juegos/Cyberpunk2077/prueba.txt; test ! -x /run/ultra-juegos && echo D-PRIVADO-$((6*7))'")
        check('D-PRIVADO-42' in out, 'UID1000 escribe en ext4 y no puede recorrer D:')
        out = command(user+"/usr/bin/bash -c 'mkdir -p \"$HOME/.local/state/prueba\" \"$HOME/Juegos/cliente\" && echo PADRES-$((6*7))'")
        check('PADRES-42' in out, 'carpetas de estado y juegos pertenecen al usuario')
        out = command('test ! -e "/arch/run/ultra-juegos/volumen/Cyberpunk 2077/prueba.txt" && cat "/arch/run/ultra-juegos/volumen/Cyberpunk 2077/bin/x64/Cyberpunk2077.exe"')
        check('ULTRA-LAB-GAME-ORIGINAL' in out, 'escritura overlay no modifica el juego original NTFS')
        command('ultra-escritorio --hyprland --software > /tmp/audio-juegos-sesion.log 2>&1 &')
        check(wait_app('foot', 60), 'escritorio usa home persistente')
        if windows_test:
            monitor.sendall(b'sendkey meta_l-shift-e\n')
            check(wait_app('thunar', 25), 'Windows abre desde el teclado en el explorador de archivos')
            time.sleep(.5)
            snapshot('windows-nvme.png')
        out=command(user+'/usr/bin/python -c \'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print("LOOPBACK-"+str(6*7)); s.close()\'')
        check('LOOPBACK-42' in out, 'usuario puede crear un servicio local TCP en loopback')
        deadline=time.monotonic()+35
        while time.monotonic()<deadline:
            out=command(user+'/usr/bin/pactl list sinks short')
            if 'alsa_output.usb-' in out: break
            time.sleep(1)
        check('alsa_output.usb-' in out, 'PipeWire detecta salida USB mediante ALSA real emulada')
        out=command(user+'/usr/bin/wpctl inspect @DEFAULT_AUDIO_SINK@')
        check(bool(re.search(r'priority[.]session\s*=\s*"?2000',out)), 'USB tiene prioridad de salida configurada')
        command(user+'/usr/local/bin/ultra-audio boost150')
        for _ in range(20):
            out=command(user+'/usr/bin/wpctl get-volume @DEFAULT_AUDIO_SINK@')
            if '1.50' in out:break
            time.sleep(.2)
        if '1.50' not in out:
            command(user+'/usr/bin/pactl get-sink-volume @DEFAULT_SINK@; '+user+'/usr/bin/wpctl inspect @DEFAULT_AUDIO_SINK@')
            command(user+'/usr/bin/pactl set-sink-volume @DEFAULT_SINK@ 150%; sleep .5; '+user+'/usr/bin/wpctl get-volume @DEFAULT_AUDIO_SINK@')
        check('1.50' in out, 'boost maestro 150% aplicado')
        out=command(user+'/usr/local/bin/ultra-audio subir; '+user+'/usr/bin/wpctl get-volume @DEFAULT_AUDIO_SINK@')
        check('1.50' in out and '1.55' not in out, 'subir respeta el limite 150%')
        out=command(user+'/usr/local/bin/ultra-audio silencio; '+user+'/usr/bin/wpctl get-volume @DEFAULT_AUDIO_SINK@')
        check('[MUTED]' in out, 'silencio maestro')
        command(user+'/usr/local/bin/ultra-audio silencio; '+user+'/usr/local/bin/ultra-audio normal')
        out=command(user+'/usr/bin/timeout -k 2 10 /usr/bin/speaker-test -D default -c 2 -t sine -f 440 -l 0; echo REPRODUCCION-RC-$?', 25)
        check('REPRODUCCION-RC-124' in out, 'generador ALSA detenido al terminar la ventana de diez segundos')
        command(ipc+'eval \'hl.exec_cmd("ultra-juegos gog")\'')
        check(wait_app('heroic', 90), 'Heroic abre en Wayland como usuario sin root')
        snapshot('heroic-gog.png')
        command(ipc+'eval \'hl.exec_cmd("ultra-juegos steam")\'')
        steam_ready=wait_steam_login()
        if not steam_ready:
            command('ultra-entrar /usr/bin/tail -n 80 /juegos/usuarios/ultra/.cache/ultra/steam.log /juegos/usuarios/ultra/.local/share/Steam/logs/console-linux.txt /juegos/usuarios/ultra/.local/share/Steam/logs/cef_log.txt',60)
            snapshot('steam-diagnostico.png')
        check(steam_ready, 'Steam abre cliente/login sin iniciar sesion de cuenta')
        space = command('ultra-entrar /usr/bin/df -B1 --output=size,used,avail /juegos')
        snapshot('steam-login.png')
        snapshot('clientes-listos.png')
        out=command('for name in heroic steam pipewire wireplumber; do for p in $(pidof "$name"); do echo CRED-$name; cat /proc/$p/status; echo CRED-FIN; done; done')
        secured=set()
        for name,status in re.findall(r'^CRED-(\w+)\n(.*?)^CRED-FIN',out.replace('\r',''),re.M|re.S):
            good=bool(re.search(r'^Uid:\s+1000\s+1000\s+1000\s+1000\s*$',status,re.M))
            good &= all(bool(re.search(rf'^{field}:\s+0+\s*$',status,re.M)) for field in ('CapEff','CapBnd','CapAmb','CapPrm'))
            good &= bool(re.search(r'^NoNewPrivs:\s+1\s*$',status,re.M))
            if good:secured.add(name)
        check({'heroic','steam','pipewire','wireplumber'}<=secured, 'clientes mantienen UID1000, capacidades cero y NoNewPrivs')
        console.sendall(b'reboot\n')
        transcript += base.read_until(console, 'Restarting system', 40)
        process.wait(timeout=20); console.close(); monitor.close()
        if windows_test:
            check('VERDE [WINDOWS]: NVMe desmontado' in transcript,
                  'Windows se desmonta limpiamente antes de reiniciar')
        raw=capture.read_bytes()
        # QEMU on Windows can leave zero RIFF/data lengths when -no-reboot exits.
        # Validate its actual PCM bytes and write a playable container with real lengths.
        assert raw[:4]==b'RIFF' and raw[8:16]==b'WAVEfmt ' and raw[36:40]==b'data'
        fmt,channels,rate,_,align,bits=struct.unpack('<HHIIHH',raw[20:36])
        assert fmt==1 and bits==16 and align==channels*2
        pcm=raw[44:]
        check(len(pcm)>1000*align and len(pcm)%align==0 and any(pcm), 'audio USB produjo muestras PCM no silenciosas')
        with wave.open(str(base.BUILD/'audio-usb-prueba.wav'),'wb') as wav:
            wav.setnchannels(channels); wav.setsampwidth(bits//8); wav.setframerate(rate); wav.writeframes(pcm)
        start(); setup()
        out=command('ultra-entrar /usr/bin/cat /juegos/usuarios/ultra/prueba.txt /juegos/Cyberpunk2077/prueba.txt')
        check('PERSISTENCIA-BMO' in out and 'ESCRITURA-OVERLAY' in out, 'home y cambios de juego sobreviven a reinicio')
        console.sendall(b'poweroff\n')
        transcript += base.read_until(console, 'APAGADO', 30)
        process.wait(timeout=20)
        verified={Path(line.split()[1]).name:line.split()[0] for line in (base.BUILD/'SHA256SUMS').read_text().splitlines() if line.strip()}
        result={'perfil':profile,'pruebas':results,'disco_real_expuesto':False,
                'imagen':base.IMG.name,'imagen_sha256':verified[base.IMG.name],
                'cache_cliente_reutilizada':reuse}
        result['banco_grafica_12GiB'] = graphics_bank
        result['espacio_tras_abrir_clientes'] = space
        if windows_test:
            with windows_copy.open('rb') as stream:
                windows_after = hashlib.file_digest(stream, 'sha256').hexdigest()
            assert windows_after == windows_before, 'El NTFS Windows ficticio cambio'
            result['windows_ficticio_sha256_antes_despues'] = windows_after
        if preflight:result['helper_prevalidacion_sha256']=hashlib.sha256((base.ROOT/'src/rootfs/bin/ultra-preparar-juegos').read_bytes()).hexdigest()
        if dependencies:result['bibliotecas_prevalidacion_sha256']=dependencies_digest
        (base.BUILD/'audio-juegos-resultados.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    finally:
        if process is not None and process.poll() is None:
            try:
                console.sendall(b'poweroff\n')
                transcript+=base.read_until(console,'APAGADO',30)
                process.wait(timeout=20)
            except (OSError,base.Incomplete,subprocess.TimeoutExpired):
                process.kill(); process.wait(timeout=10)
        (base.BUILD/('bibliotecas.log' if libraries_test else 'warframe-boot.log' if warframe_test else 'audio-juegos.log')).write_text(transcript,encoding='utf-8')


if __name__=='__main__': main()
