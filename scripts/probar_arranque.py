# SPDX-License-Identifier: GPL-2.0-only
"""Boot the downloaded kernel headless, drive its shell over serial and check the answers."""
import base64, hashlib, json, pathlib, re, secrets, shutil, socket, subprocess, sys, tempfile, time

ROOT = pathlib.Path(__file__).resolve().parent.parent
PERFIL = sys.argv[1] if len(sys.argv) > 1 else 'produccion'
# --disco boots ultra-esp.img through UEFI firmware, with no -kernel and no boot loader.
# --bmo boots disco-bmo.img: ULTRA as files on a FAT32 partition labelled BMO, beside a
# simulated Windows EFI partition, the way it lives on A:\ of the PC.
# --auto (perfil pc) boots disco-bmo-doble.img three times: the generic kernel prepares the
# F11 entry of the tuned one and asks for it once; the firmware starts it; it marks itself good.
# --auto-falla: the same, but the tuned kernel is deleted first, so the safety net must act.
FALLA = '--auto-falla' in sys.argv[2:]
AUTO = FALLA or '--auto' in sys.argv[2:]
BMO = AUTO or '--bmo' in sys.argv[2:]
BRAVE = '--brave' in sys.argv[2:]
DISCO = BMO or '--disco' in sys.argv[2:]
QEMU_SHARE = pathlib.Path(r'C:\Program Files\qemu\share')
BUILD = ROOT / 'build' / {'produccion': 'baseline', 'vigia': 'vigia', 'generico': 'generico', 'pc': 'pc'}[PERFIL]
KERNEL = BUILD / 'bzImage-tiny64'
IMG = BUILD / ('disco-bmo-doble.img' if AUTO else 'disco-bmo.img' if BMO else 'ultra-esp.img')
# The disk test writes to /datos, so it boots a fresh copy and never the verified image.
IMG_PRUEBA = BUILD / (IMG.stem + '-prueba.img')
LOG = BUILD / ('prueba-auto-falla.log' if FALLA else 'prueba-auto.log' if AUTO else 'prueba-bmo.log' if BMO else 'prueba-disco.log' if DISCO else 'prueba-shell.log')
QEMU = pathlib.Path(r'C:\Program Files\qemu\qemu-system-x86_64.exe')
PORT = 15555
# Each command and a fragment its output must contain.
CHECKS = [
    ('cat /etc/os-release', 'NAME="ULTRA LinuX"'),
    ('uname -sm', 'Linux x86_64'),
    ('ps', '/init'),
    ('echo $((6*7))', '42'),
    ("date -d @0 '+%Y-%m-%d %H:%M %z'", '1969-12-31 19:00 -0500'),
    ('ls /proc/self/fd', '0'),
    # Guardians on the ULTRA kernel itself: CET, a UB trap and the box.
    ('minas --cet', 'ACTIVA (shstk)'),
    ('minas retorno-secuestrado', 'SEGV_CPERR'),
    ('minas entero-desborde', 'GUARDIAN-UB'),
    ('minas copia-pila', 'buffer overflow detected'),
    ('minas memcpy-solapado', 'FRONTERA-SOLAPE'),
    ('minas alias-estricto', 'NEUTRALIZADA'),
    ('minas secuestrar-arranque', 'FRONTERA-MEMORIA'),
    ('grep x86_Thread /proc/1/status', 'shstk'),
    ('probar-caja', 'RESUMEN CAJA: VERDE=13 ROJO=0'),
    ('dmesg | grep -m1 "kfence: init"', 'bytes for'),
]
# The vigia kernel must show its detectors alive, then a clean log after the whole run.
# The expected text differs from the typed command, so the shell's echo never matches it.
REPORTS = 'BUG:|UBSAN:|KASAN:|KFENCE:|WARNING:|circular locking|recursive locking|INFO: task|soft lockup'
if PERFIL == 'pc':
    # The Zen 3 kernel: its own uname -r, and modules built for it.
    CHECKS += [('uname -r', '-pc-g')]
if PERFIL in ('generico', 'pc'):
    # The generic kernel sees the PC's buses and drivers, not only a serial console.
    CHECKS += [
        ("ultra-entrar /usr/bin/date -d @0 '+%Y-%m-%d %H:%M %z'", '1969-12-31 19:00 -0500'),
        ("ultra-entrar /usr/bin/date -d @0 '+%B'", 'diciembre'),
        ('ultra-entrar /usr/local/bin/ultra-hora', 'America/Lima'),
        ('ls /sys/bus/pci/drivers', 'ahci'),
        ('grep -c processor /proc/cpuinfo', '1'),
        ('cat /proc/filesystems', 'ext4'),
    ]
if DISCO:
    # Proof that the firmware ran the kernel itself and that the built-in arguments arrived.
    CHECKS += [
        ('cat /sys/firmware/efi/fw_platform_size', '64'),
        ('cat /proc/cmdline', 'x86_shstk=on'),
        # SATA ports scanned in parallel; and nothing from the firmware after the built-in line.
        # The value the kernel applied, not the text on the line: "ahci.ignore_sss" was
        # silently ignored on the real PC because the parameter belongs to libahci.
        ('echo SSS-$(cat /sys/module/libahci/parameters/ignore_sss)', 'SSS-1'),
        ('echo FIN-$(sed "s/.*x86_shstk=on//" /proc/cmdline | wc -c)', 'FIN-1'),
        ('cat /sys/class/tty/console/active', 'ttyS0'),
    ]
# Persistence: boot 1 detects the hardware and writes to /datos, then a real reboot.
# QEMU adds an Intel HD Audio controller so detection has a module to find.
TOKEN = 'PERSISTE-' + secrets.token_hex(4)
BOOT1 = [
    ('grep " /sistema " /proc/mounts', 'erofs ro'),
    ('grep " /datos " /proc/mounts', 'ext4 rw'),
    ('cat /sistema/ULTRA-SISTEMA', 'kernel=7.'),
    ('touch /sistema/intruso', 'Read-only file system'),
    ('cat /datos/PC.config*', 'snd_hda_intel'),
    ('lsmod', 'snd_hda_intel'),
    (f'echo {TOKEN} > /datos/prueba-persistencia; sync; echo ESCRITO-$((1+1))', 'ESCRITO-2'),
    # Wayland stage W1: Arch's base system at /arch, dynamic glibc programs under ULTRA.
    ("ultra-entrar /usr/bin/bash -c 'echo ARCH-$BASH_VERSION'", 'ARCH-5.'),
    ('ultra-entrar /usr/bin/ls --version', 'GNU coreutils'),
    # The machine has a name, and Arch's login shell says where you are.
    ('echo HOST-$(hostname)', 'HOST-ultra'),
    ("ultra-entrar /usr/bin/bash -lc 'echo LISTO'", 'Dentro del sistema Arch de ULTRA LinuX'),
    # The shadow stack also guards Arch's dynamic programs: grep reading its own status.
    ('ultra-entrar /usr/bin/grep x86_Thread /proc/self/status', 'shstk'),
    ('ultra-entrar /usr/bin/ls /datos', 'prueba-persistencia'),
    # The image itself never changes: writes land in the layer in memory.
    ("ultra-entrar /usr/bin/bash -c 'echo capa-$((2+2)) > /etc/prueba-capa && cat /etc/prueba-capa'", 'capa-4'),
    ('grep -c prueba-capa /sistema/etc/prueba-capa 2>&1; echo IMAGEN-$((1+1))', 'No such file'),
]
if PERFIL == 'pc':
    # W3a, the RTX 3060: module, firmware and userspace in the tuned image. QEMU has no NVIDIA
    # GPU, so nvidia-smi must start and report that it finds no driver: binary and libraries work.
    BOOT1 += [
        ('ls /sistema/lib/modules/$(uname -r)/kernel/drivers/video/', 'nvidia-drm.ko'),
        ('ls /sistema/usr/lib/firmware/nvidia/*/', 'gsp_ga10x.bin'),
        ('cat /proc/cmdline', 'nvidia-drm.modeset=1'),
        ('ultra-entrar /usr/bin/nvidia-smi', 'NVIDIA-SMI has failed'),
    ]
elif PERFIL == 'generico':
    # The generic image stays without NVIDIA: it is the safety net if the driver ever fails.
    BOOT1 += [('ls /sistema/usr/bin/nvidia-smi 2>&1', 'No such file')]
if PERFIL in ('pc', 'generico'):
    BOOT1 += [('test -s /sistema/usr/lib/firmware/rtl_nic/rtl8168h-2.fw && echo REALTEK-FW-$((4+5))', 'REALTEK-FW-9'),
              ('cat /sys/module/firmware_class/parameters/path', '/sistema/usr/lib/firmware'),
              ('ultra-entrar /usr/bin/setpriv --reuid=1000 --regid=1000 --clear-groups --no-new-privs --bounding-set=-all /usr/local/libexec/ultra-red-contadores', 'RED ')]
    # QEMU has no NVIDIA: explicit acceleration must reject, never pretend software is GPU.
    BOOT1 += [('ultra-escritorio --nvidia 2>&1', 'ROJO [GPU]: se requiere exactamente un dispositivo DRM NVIDIA'),
              ('ultra-escritorio --nvidia >/tmp/gpu-rechazo.log 2>&1; echo GPU-ESTADO-$?', 'GPU-ESTADO-1')]
    if BRAVE:
        BOOT1 += [('ultra-entrar /opt/brave/brave --version', '1.97.59'),
                  ('test -e /proc/sys/user/max_user_namespaces && echo USERNS-DISPONIBLE-$((4+4))', 'USERNS-DISPONIBLE-8')]
BOOT2 = [
    ('cat /datos/prueba-persistencia', TOKEN),
    ('lsmod', 'snd_hda_intel'),
    ('cat /datos/arranques.log', 'arranque=deteccion'),
    ('cat /datos/arranques.log', 'arranque=guiado'),
]
if BMO:
    BOOT1 += [
        ('grep " /bmo " /proc/mounts', 'vfat rw'),
        # Only BMO is FAT and mounted: the simulated Windows EFI partition stays untouched.
        ('echo VFAT-$(grep -c vfat /proc/mounts)', 'VFAT-1'),
        ('ls /bmo/ULTRA/informe', 'PC-configuracion.txt'),
        # The kernel log of this boot is saved for Windows on its own.
        ('echo DMESG-$(cat /bmo/ULTRA/informe/dmesg-*.txt | grep -c "Linux version")', 'DMESG-1'),
    ]
    # The copy Windows reads on A:\ULTRA is updated on every boot.
    BOOT2 += [('cat /bmo/ULTRA/informe/arranques.txt', 'arranque=guiado')]
if PERFIL == 'vigia':
    CHECKS += [
        ('dmesg | grep -m1 "AddressSanitizer init"', 'initialized'),
        ('dmesg | grep -m1 "Lock dependency validator"', 'Ingo Molnar'),
        (f'dmesg | grep -E "{REPORTS}" || echo VIGIA-$((1))-LIMPIO', 'VIGIA-1-LIMPIO'),
    ]

class Incomplete(Exception):
    """A wait that failed; what did arrive still goes to the log."""
    def __init__(self, why, partial):
        super().__init__(why)
        self.partial = partial

def read_until(conn, marker, timeout):
    data, end = b'', time.time() + timeout
    while marker.encode() not in data:
        if time.time() > end: raise Incomplete(f'no aparecio {marker!r}', data.decode(errors='replace'))
        try:
            chunk = conn.recv(4096)
        except socket.timeout:
            continue
        if not chunk: raise Incomplete('la consola serie se cerro', data.decode(errors='replace'))
        data += chunk
    return data.decode(errors='replace')

def check_kernel():
    """Refuse to boot anything but the downloaded, hash-verified kernel, and say why."""
    sums = KERNEL.parent / 'SHA256SUMS'
    if not KERNEL.exists() or not sums.exists():
        print(f'AMARILLO [SIN-KERNEL]: no hay kernel {PERFIL} descargado en {KERNEL.parent}.')
        print(f'Primero: .\\construir-kernel.ps1 -Accion Estado -Perfil {PERFIL}  (esperar el VERDE)')
        print(f'Despues: .\\construir-kernel.ps1 -Accion Descargar -Perfil {PERFIL}')
        sys.exit(2)
    expected = {line.split()[1].rsplit('/', 1)[-1]: line.split()[0].lower()
                for line in sums.read_text(encoding='utf-8').splitlines() if line.strip()}
    for f in [KERNEL] + ([IMG] if DISCO else []):
        if not f.exists() or f.name not in expected:
            print(f'AMARILLO [SIN-DISCO]: falta {f.name}; el disco EFI solo existe en el perfil generico.')
            sys.exit(2)
        with f.open('rb') as source:
            actual_hash = hashlib.file_digest(source, 'sha256').hexdigest()
        if actual_hash != expected[f.name]:
            print(f'ROJO [HASH]: {f.name} no coincide con SHA256SUMS; volver a descargar.')
            sys.exit(1)

def run(conn, checks, failures):
    transcript = ''
    for command, expected in checks:
        conn.sendall(command.encode() + b'\n')
        out = read_until(conn, 'ultra:', 30)
        transcript += out
        print(('VERDE' if expected in out else 'ROJO'), f'[{command}]')
        if expected not in out: failures.append(command)
    return transcript

def expect_boot(boot, expected, label, failures):
    """What PID 1 reported while booting, before the first prompt."""
    print(('VERDE' if expected in boot else 'ROJO'), f'[{label}]')
    if expected not in boot: failures.append(label)

def check_menu(conn, failures):
    """Use the installed short command on the guest's interactive serial console."""
    conn.sendall(b'e\n')
    text = read_until(conn, 'Al cerrar', 10)
    expect_boot(text, 'Flechas arriba/abajo y Enter', 'menu instalado: comando e', failures)
    conn.sendall(b'\x1b[B')
    text += read_until(conn, 'Al cerrar', 10)
    conn.sendall(b'\r')
    result = read_until(conn, 'Pulsa una tecla', 30)
    text += result
    expect_boot(result, 'ROJO [GPU]: se requiere exactamente un dispositivo DRM NVIDIA',
                'menu: flecha y Enter seleccionan NVIDIA, rechazo sin GPU', failures)
    conn.sendall(b'x')
    text += read_until(conn, 'Al cerrar', 10)
    conn.sendall(b'4')
    text += read_until(conn, 'Al cerrar', 10)
    conn.sendall(b'\r')
    result = read_until(conn, 'Pulsa una tecla', 10)
    text += result
    expect_boot(result, 'ULTIMO INTENTO REGISTRADO', 'menu: diagnostico con numero y Enter', failures)
    conn.sendall(b'x')
    text += read_until(conn, 'Al cerrar', 10)
    conn.sendall(b'q')
    text += read_until(conn, 'ultra:', 10)
    print('VERDE [menu: Q devuelve la consola]')
    return text


def check_window(conn, failures):
    """W2: the first Wayland window, seen on the emulated screen and closed with its keyboard.

    ultra-escritorio starts udevd, seatd and cage with foot. foot paints the whole screen
    with its background (#050a12); then "exit" typed on the PS/2 keyboard must reach foot
    through udev, libinput and Wayland, and cage must end cleanly.
    """
    t = ''
    conn.sendall(b'ultra-escritorio --auto > /tmp/escritorio.log 2>&1 &\n')
    t += read_until(conn, 'ultra:', 10)
    tmp = pathlib.Path(tempfile.gettempdir())
    share, png = 0.0, tmp / 'ultra-ventana.png'
    try:
        mon = socket.create_connection(('127.0.0.1', PORT + 1), timeout=5)
        for _ in range(30):
            time.sleep(2)
            for name in ('ultra-ventana.ppm', 'ultra-ventana.png'):
                (tmp / name).unlink(missing_ok=True)
                mon.sendall(f'screendump {tmp / name} -f {name.rsplit(".", 1)[1]}\n'.encode())
            for _ in range(50):
                time.sleep(0.1)
                if png.exists() and png.stat().st_size > 0: break
            time.sleep(0.3)
            pixels = (tmp / 'ultra-ventana.ppm').read_bytes().split(maxsplit=4)[4]
            foot = sum(1 for i in range(0, len(pixels) - 2, 3)
                       if all(abs(pixels[i + k] - (5, 10, 18)[k]) <= 6 for k in range(3)))
            share = foot / max(1, len(pixels) // 3)
            if share > 0.6: break
        shutil.copyfile(png, LOG.with_name(LOG.stem + '-ventana.png'))
        ok = share > 0.6
        print(('VERDE' if ok else 'ROJO'), f'[ventana Wayland: foot cubre el {share:.0%} de la pantalla]')
        if not ok:
            failures.append('ventana Wayland')
            t += run(conn, [('cat /tmp/escritorio.log; echo DIAG-ESCRITORIO', 'DIAG-ESCRITORIO')], failures)
        # Inspect the running compositor and terminal, not just the launcher's declaration.
        cred_command = 'for n in cage foot; do for p in $(pidof "$n"); do echo ULTRA-CRED-$n; cat /proc/$p/status; echo ULTRA-CRED-FIN; done; done'
        credentials = run(conn, [(cred_command, 'ULTRA-CRED-cage')], failures)
        t += credentials
        seen = set()
        for name, status in re.findall(r'^ULTRA-CRED-(cage|foot)\n(.*?)^ULTRA-CRED-FIN', credentials.replace('\r', ''), re.M | re.S):
            good = all(re.search(rf'^{field}:\s+1000\s+1000\s+1000\s+1000\s*$', status, re.M)
                       for field in ('Uid', 'Gid'))
            good = good and all(re.search(rf'^{field}:\s+0+\s*$', status, re.M)
                                for field in ('CapInh', 'CapPrm', 'CapEff', 'CapBnd', 'CapAmb'))
            good = good and bool(re.search(r'^NoNewPrivs:\s+1\s*$', status, re.M))
            if good: seen.add(name)
        secured = seen == {'cage', 'foot'}
        print(('VERDE' if secured else 'ROJO'), '[cage y foot: UID/GID 1000, capacidades cero y NoNewPrivs=1]')
        if not secured: failures.append('credenciales de cage y foot')
        # Close it from the emulated keyboard: udev -> libinput -> cage -> foot.
        for key in ['e', 'x', 'i', 't', 'ret']:
            mon.sendall(f'sendkey {key}\n'.encode()); time.sleep(0.05)
        mon.close()
    except (OSError, IndexError) as exc:
        print('ROJO [ventana Wayland]:', exc); failures.append('ventana Wayland')
    time.sleep(4)
    # The shell printed "[1]+ Done" and an extra prompt meanwhile: sync on a unique marker first.
    conn.sendall(b'echo SINC-$((3+3))\n')
    t += read_until(conn, 'SINC-6', 10) + read_until(conn, 'ultra:', 10)
    t += run(conn, [('echo CAGE-$(grep -c "cage termino (salida 0)" /tmp/escritorio.log)', 'CAGE-1')], failures)
    t += run(conn, [('test ! -S /arch/run/seatd.sock && echo SEATD-LIMPIO', 'SEATD-LIMPIO')], failures)
    return t

def check_screen(failures):
    """The screen must be alive: typing on the keyboard into the tty1 shell changes it.

    Counting lit pixels is not enough: with no console driver the firmware's last
    image stays frozen on the monitor, text and logo included.
    """
    # HMP splits arguments on spaces and this project's path has one: capture in TEMP.
    tmp = pathlib.Path(tempfile.gettempdir())
    def dump(mon, name, fmt):
        (tmp / name).unlink(missing_ok=True)
        mon.sendall(f'screendump {tmp / name} -f {fmt}\n'.encode())
        for _ in range(50):
            time.sleep(0.1)
            if (tmp / name).exists() and (tmp / name).stat().st_size > 0: break
        time.sleep(0.3)
        return (tmp / name).read_bytes()
    try:
        mon = socket.create_connection(('127.0.0.1', PORT + 1), timeout=5)
        before = dump(mon, 'ultra-antes.ppm', 'ppm').split(maxsplit=4)[4]
        # PS/2 keyboard -> tty1 shell: "echo 4242" and Enter.
        for key in ['e', 'c', 'h', 'o', 'spc', '4', '2', '4', '2', 'ret']:
            mon.sendall(f'sendkey {key}\n'.encode()); time.sleep(0.05)
        time.sleep(1.5)
        after = dump(mon, 'ultra-despues.ppm', 'ppm').split(maxsplit=4)[4]
        dump(mon, 'ultra-pantalla.png', 'png')
        mon.close()
        shutil.copyfile(tmp / 'ultra-pantalla.png', LOG.with_name(LOG.stem + '-pantalla.png'))
    except (OSError, IndexError) as exc:
        print('ROJO [pantalla]:', exc); failures.append('pantalla'); return
    changed = sum(1 for i in range(0, min(len(before), len(after)), 3) if before[i:i + 3] != after[i:i + 3])
    ok = len(before) == len(after) and changed > 100
    print(('VERDE' if ok else 'ROJO'), f'[pantalla viva: el teclado cambio {changed} pixeles en tty1]')
    if not ok: failures.append('pantalla congelada')


def check_hyprland(conn, failures):
    """Actual compositor, apps and keyboard on virtual DRM with CPU rendering."""
    transcript = ''
    conn.sendall(b'ultra-escritorio --hyprland --software > /tmp/hyprland-session.log 2>&1 &\n')
    transcript += read_until(conn, 'ultra:', 10)
    ipc = 'ultra-entrar /usr/bin/env XDG_RUNTIME_DIR=/run/user/1000 /usr/bin/hyprctl -i 0 -j clients'

    def response(marker, timeout=15):
        out = read_until(conn, marker, timeout)
        if 'ultra:' not in out.rsplit(marker, 1)[-1]:
            out += read_until(conn, 'ultra:', 10)
        return out

    def clients():
        nonlocal transcript
        conn.sendall(ipc.encode() + b'; echo HYPR-IPC-$((4+3))\n')
        out = response('HYPR-IPC-7')
        transcript += out
        try:
            start = re.search(r'(?m)^\s*\[\s*(?:\{|\])', out)
            if not start: return []
            decoded = json.JSONDecoder().raw_decode(out[start.start():].lstrip())[0]
            return decoded if isinstance(decoded, list) and all(isinstance(a, dict) for a in decoded) else []
        except (ValueError, TypeError):
            return []

    def expect(ok, label):
        print(('VERDE' if ok else 'ROJO'), '[' + label + ']')
        if not ok: failures.append(label)

    def wait_apps(predicate):
        for _ in range(25):
            found = clients()
            if predicate(found): return found
            time.sleep(1)
        return []

    def focus_app(name):
        nonlocal transcript
        apps = clients()
        app = next((a for a in apps if name in a.get('class', '').lower()), None)
        address = app.get('address', '') if app else ''
        if not re.fullmatch(r'0x[0-9a-fA-F]+', address): return False
        command = ('ultra-entrar /usr/bin/env XDG_RUNTIME_DIR=/run/user/1000 /usr/bin/hyprctl -i 0 '
                   'dispatch \'hl.dsp.focus({ window = "address:' + address + '" })\'; echo FOCO-ENVIO-$((30+1))\n')
        conn.sendall(command.encode())
        transcript += response('FOCO-ENVIO-31')
        for _ in range(10):
            conn.sendall(b'ultra-entrar /usr/bin/env XDG_RUNTIME_DIR=/run/user/1000 /usr/bin/hyprctl -i 0 -j activewindow; echo FOCO-ACTIVO-$((30+2))\n')
            active = response('FOCO-ACTIVO-32')
            transcript += active
            if re.search(r'"address"\s*:\s*"' + re.escape(address) + r'"', active): return True
            time.sleep(.2)
        return False

    mon = socket.create_connection(('127.0.0.1', PORT + 1), timeout=5)
    try:
        initial = wait_apps(lambda apps: len(apps) >= 2)
        expect(bool(initial), 'Hyprland: dos ventanas iniciales mapeadas')
        if not initial:
            transcript += run(conn, [('cat /tmp/hyprland-session.log; echo HYPR-DIAG', 'HYPR-DIAG')], failures)
            return transcript
        # The shell must actually draw both panels, not merely leave waybar alive.
        panels_ready = False
        for _ in range(20):
            conn.sendall(b'ultra-entrar /usr/bin/env XDG_RUNTIME_DIR=/run/user/1000 /usr/bin/hyprctl -i 0 -j layers; echo BMO-LAYERS-$((10+1))\n')
            layers = response('BMO-LAYERS-11')
            transcript += layers
            panels_ready = all(re.search(r'"namespace"\s*:\s*"'+name+r'"', layers) for name in ('hud', 'dock'))
            if panels_ready: break
            time.sleep(.25)
        expect(panels_ready, 'BMO-X: barra y dock mapeados en Wayland')
        conn.sendall(b'ultra-entrar /usr/bin/env XDG_RUNTIME_DIR=/run/user/1000 /usr/bin/hyprctl -i 0 eval \'hl.exec_cmd("ultra-graficos informe")\'; echo GRAFICA-INICIO-$((40+1))\n')
        transcript += response('GRAFICA-INICIO-41')
        report_ready = False
        for _ in range(45):
            conn.sendall(b'ultra-entrar /usr/bin/cat /run/user/1000/ultra-graficos.txt; echo GRAFICA-REPORTE-$((40+2))\n')
            report = response('GRAFICA-REPORTE-42')
            transcript += report
            if 'ULTRA GRAFICOS v1' in report:
                report_ready = True
                break
            time.sleep(1)
        expect(report_ready and '[VULKAN]' in report and 'OpenGL' in report,
               'Graficos: informe atomico Vulkan/OpenGL con proveedores reales')
        expect('llvmpipe' in report.lower(), 'Graficos: laboratorio identifica render software')
        if PERFIL == 'generico':
            expect('PHYSICAL_DEVICE_TYPE_CPU' in report or 'deviceType         = CPU' in report,
                   'Vulkan: dispositivo CPU del banco identificado')
        # Ask an activated service to open its real Wayland connection, not just check env strings.
        portal_cmd = ('gdbus call --session --dest org.freedesktop.portal.Desktop '
                      '--object-path /org/freedesktop/portal/desktop '
                      '--method org.freedesktop.portal.Settings.Read '
                      'org.freedesktop.appearance color-scheme > /run/user/1000/portal-prueba.txt 2>&1')
        conn.sendall(('ultra-entrar /usr/bin/env XDG_RUNTIME_DIR=/run/user/1000 /usr/bin/hyprctl -i 0 eval '
                      + "'hl.exec_cmd(\"" + portal_cmd + "\")'; echo PORTAL-INICIO-$((40+3))\n").encode())
        transcript += response('PORTAL-INICIO-43')
        portal_ready = False
        for _ in range(20):
            conn.sendall(b'ultra-entrar /usr/bin/cat /run/user/1000/portal-prueba.txt; echo PORTAL-RESULTADO-$((40+4))\n')
            portal = response('PORTAL-RESULTADO-44')
            transcript += portal
            if 'uint32' in portal:
                portal_ready = True
                break
            time.sleep(1)
        expect(portal_ready, 'Wayland: portal Settings responde tras activacion D-Bus')
        if PERFIL == 'generico':
            conn.sendall(b'ultra-entrar /usr/bin/env XDG_RUNTIME_DIR=/run/user/1000 /usr/bin/hyprctl -i 0 eval \'hl.exec_cmd("ultra-graficos validar")\'; echo VULKAN-CUBO-$((40+5))\n')
            transcript += response('VULKAN-CUBO-45')
            cube = wait_apps(lambda apps: any('vkcube' in (a.get('class', '') + ' ' + a.get('title', '')).lower() for a in apps))
            expect(bool(cube), 'Vulkan: cubo Wayland mapeado con validacion por proceso')
            for _ in range(35):
                conn.sendall(b'ultra-entrar /usr/bin/cat /home/ultra/.cache/ultra/vulkan-cubo.log; echo VULKAN-CUBO-FIN-$((40+6))\n')
                cube_log = response('VULKAN-CUBO-FIN-46')
                transcript += cube_log
                if 'ULTRA_CUBO codigo_salida=' in cube_log: break
                time.sleep(1)
            expect('llvmpipe' in cube_log.lower() and 'Validation Error' not in cube_log and
                   'ULTRA_CUBO codigo_salida=0' in cube_log,
                   'Vulkan: cubo CPU cierra normalmente y sin errores de validacion')
        transcript += run(conn, [('test -n "$(pidof mako)" && test -f /arch/usr/share/icons/Papirus-Dark/index.theme && ultra-entrar /usr/bin/fc-match "Font Awesome 7 Free:charset=f120" | grep -q "[.]otf" && echo BMO-ICONOS-$((10+2))', 'BMO-ICONOS-12')], failures)
        for wanted in (1, 0):
            mon.sendall(b'sendkey meta_l-shift-a\n')
            time.sleep(1)
            conn.sendall(b'ultra-entrar /usr/bin/env XDG_RUNTIME_DIR=/run/user/1000 /usr/bin/hyprctl -i 0 -j getoption animations:enabled; echo BMO-EFECTOS-$((10+3))\n')
            effect = response('BMO-EFECTOS-13')
            transcript += effect
            expect(bool(re.search(r'"bool"\s*:\s*'+('true' if wanted else 'false')+r'\b', effect)),
                   f'BMO-X: teclado cambia animaciones a {wanted}')
        mon.sendall(b'sendkey meta_l-spc\n')
        time.sleep(1)
        transcript += run(conn, [('test -n "$(pidof fuzzel)" && echo BMO-CENTRO-$((10+4))', 'BMO-CENTRO-14')], failures)
        control_capture = pathlib.Path(tempfile.gettempdir()) / 'ultra-bmox-centro.png'
        control_capture.unlink(missing_ok=True)
        mon.sendall(f'screendump {control_capture} -f png\n'.encode())
        for _ in range(50):
            try:
                from PIL import Image
                with Image.open(control_capture) as shot: shot.verify()
                break
            except (OSError, SyntaxError): time.sleep(.1)
        else: raise ValueError('captura del centro BMO-X incompleta')
        shutil.copyfile(control_capture, LOG.with_name(LOG.stem + '-centro.png'))
        mon.sendall(b'sendkey esc\n')
        time.sleep(.4)
        transcript += run(conn, [('test -z "$(pidof fuzzel)" && echo BMO-CENTRO-CERRADO-$((10+5))', 'BMO-CENTRO-CERRADO-15')], failures)
        mon.sendall(b'sendkey meta_l-n\n')
        edited = wait_apps(lambda apps: any('mousepad' in a.get('class', '').lower() for a in apps))
        expect(bool(edited), 'Hyprland: Super+N abre Mousepad')
        mon.sendall(b'sendkey meta_l-e\n')
        files = wait_apps(lambda apps: any('thunar' in a.get('class', '').lower() for a in apps))
        expect(bool(files), 'Hyprland: Super+E abre archivos')
        if BRAVE:
            mon.sendall(b'sendkey meta_l-b\n')
            browsers = wait_apps(lambda apps: any('brave' in a.get('class', '').lower() and 'ULTRA' in a.get('title', '') for a in apps))
            expect(bool(browsers), 'Brave: Super+B abre la pagina local en Wayland')
            if browsers:
                browser = next(a for a in browsers if 'brave' in a.get('class', '').lower())
                pid = browser['pid']
                cmd = (f'echo BRAVE-BROKER; cat /proc/{pid}/status; '
                       f'echo ns_user=$(readlink /proc/{pid}/ns/user); '
                       f'echo ns_pid=$(readlink /proc/{pid}/ns/pid); echo BRAVE-BROKER-FIN; '
                       'for p in /proc/[0-9]*; do '
                       'args=$(tr "\\0" " " < "$p/cmdline" 2>/dev/null); '
                       'case "$args" in /opt/brave/brave*--type=renderer*) '
                       'echo BRAVE-RENDERER; cat "$p/status"; '
                       'echo ns_user=$(readlink "$p/ns/user"); '
                       'echo ns_pid=$(readlink "$p/ns/pid"); '
                       'cat "$p/uid_map"; echo BRAVE-RENDERER-FIN;; esac; done')
                conn.sendall(cmd.encode() + b'; echo BRAVE-CRED-LISTO-$((9+4))\n')
                credentials = response('BRAVE-CRED-LISTO-13')
                transcript += credentials
                clean = credentials.replace('\r', '')
                broker_match = re.search(r'^BRAVE-BROKER\n(.*?)^BRAVE-BROKER-FIN', clean, re.M | re.S)
                def identity(status, caps):
                    return (bool(re.search(r'^Uid:\s+1000\s+1000\s+1000\s+1000\s*$', status, re.M))
                            and bool(re.search(r'^NoNewPrivs:\s+1\s*$', status, re.M))
                            and all(re.search(rf'^{field}:\s+0+\s*$', status, re.M) for field in caps))
                broker_status = broker_match.group(1) if broker_match else ''
                expect(identity(broker_status, ('CapInh', 'CapPrm', 'CapEff', 'CapBnd', 'CapAmb')),
                       'Brave: navegador UID1000, capacidades cero y NNP')
                def namespace(status, name):
                    match = re.search(rf'^ns_{name}=(.+)$', status, re.M)
                    return match.group(1) if match else None
                renderers = re.findall(r'^BRAVE-RENDERER\n(.*?)^BRAVE-RENDERER-FIN', clean, re.M | re.S)
                isolated = [status for status in renderers
                            if identity(status, ('CapPrm', 'CapEff', 'CapAmb'))
                            and re.search(r'^Seccomp:\s+2\s*$', status, re.M)
                            and all(namespace(status, name) is not None
                                    and namespace(status, name) != namespace(broker_status, name)
                                    for name in ('user', 'pid'))]
                expect(bool(isolated), 'Brave: renderer UID1000 inicial, user/PID namespaces propios y seccomp activo')
                # Fresh disposable profile: cancel first-run keyring provisioning.
                # No test credentials are saved and libsecret/sandbox stay enabled.
                for _ in range(3):
                    if not any('gcr-prompter' in a.get('class', '').lower() for a in clients()): break
                    if not focus_app('gcr-prompter'): break
                    mon.sendall(b'sendkey meta_l-q 30\n')
                    time.sleep(.8)
                expect(not any('gcr-prompter' in a.get('class', '').lower() for a in clients()),
                       'Brave: dialogo inicial de llavero cancelado en perfil desechable')
                expect(focus_app('brave'), 'Brave: ventana enfocada antes de enviar teclas')
                capture = pathlib.Path(tempfile.gettempdir()) / 'ultra-brave.png'
                mon.sendall(b'sendkey f11\n')
                time.sleep(1)
                capture.unlink(missing_ok=True)
                mon.sendall(f'screendump {capture} -f png\n'.encode())
                complete = False
                for _ in range(50):
                    try:
                        from PIL import Image
                        with Image.open(capture) as screenshot:
                            screenshot.verify()
                        complete = True
                        break
                    except (OSError, SyntaxError):
                        pass
                    time.sleep(.1)
                if not complete: raise ValueError('captura de Brave incompleta')
                shutil.copyfile(capture, LOG.with_name(LOG.stem + '-brave.png'))
                mon.sendall(b'sendkey f11 30\n')
                time.sleep(1)
                mon.sendall(b'sendkey ctrl-l\n')
                time.sleep(.6)
                url_keys = ['h', 't', 't', 'p', 's', 'shift-semicolon', 'slash', 'slash',
                            'e', 'x', 'a', 'm', 'p', 'l', 'e', 'dot', 'c', 'o', 'm', 'ret']
                for key in url_keys:
                    mon.sendall(f'sendkey {key} 30\n'.encode())
                    time.sleep(.12)
                loaded = wait_apps(lambda apps: any('brave' in a.get('class', '').lower() and 'Example Domain' in a.get('title', '') for a in apps))
                expect(bool(loaded), 'Brave: pagina HTTPS real cargada desde la barra de direcciones')
                if not loaded:
                    conn.sendall(b'ultra-entrar /usr/bin/tail -n 90 /home/ultra/.cache/ultra/brave.log; ultra-entrar /usr/bin/cat /run/ultra-escritorio/red.log; echo BRAVE-HTTPS-DIAG-$((5+6))\n')
                    transcript += response('BRAVE-HTTPS-DIAG-11')
                expect(focus_app('brave'), 'Brave: foco verificado antes de cerrar navegador')
                mon.sendall(b'sendkey meta_l-q\n')
                closed_browser = wait_apps(lambda apps: len(apps) >= 2 and not any('brave' in a.get('class', '').lower() for a in apps))
                expect(bool(closed_browser), 'Brave: cerrar navegador conserva el escritorio')
            else:
                conn.sendall(b'ultra-entrar /usr/bin/tail -n 60 /home/ultra/.cache/ultra/brave.log; echo BRAVE-ERROR-$((3+4))\n')
                transcript += response('BRAVE-ERROR-7')
            conn.sendall(b'ultra-entrar /usr/bin/curl --fail --silent --show-error --max-time 12 -I https://example.com && echo HTTPS-VERIFICADO-$((3+5)); echo HTTPS-FIN-$((4+5))\n')
            network = response('HTTPS-FIN-9', timeout=20)
            transcript += network
            expect('HTTPS-VERIFICADO-8' in network, 'Red: DHCP, DNS y HTTPS con certificados validos')
            conn.sendall(b'ultra-entrar /usr/bin/cat /run/ultra-hora/sincronizado; ultra-entrar /usr/bin/cat /run/ultra-hora/ultima.log; echo HORA-NTS-FIN-$((40+8))\n')
            hour = response('HORA-NTS-FIN-48')
            transcript += hour
            expect(bool(re.search(r'^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ\s*$', hour.replace('\r', ''), re.M)),
                   'Hora: sincronizacion NTS real confirmada por el auxiliar')
        mon.sendall(b'sendkey meta_l-r\n')
        launcher = False
        for _ in range(15):
            conn.sendall(b'test -n "$(pidof fuzzel)" && echo LANZADOR-$((2+2)); echo HYPR-QUERY-$((2+3))\n')
            out = response('HYPR-QUERY-5')
            transcript += out
            if 'LANZADOR-4' in out:
                launcher = True
                break
            time.sleep(1)
        expect(launcher, 'Hyprland: Super+R abre Fuzzel')
        # QEMU may still hold Super+R when pidof first sees the launcher.
        time.sleep(.3)
        mon.sendall(b'sendkey esc\n')
        time.sleep(.4)
        transcript += run(conn, [('test -z "$(pidof fuzzel)" && echo BMO-APPS-CERRADO-$((10+6))', 'BMO-APPS-CERRADO-16')], failures)
        mon.sendall(b'sendkey meta_l-j\n')
        time.sleep(1)
        transcript += run(conn, [('test -n "$(pidof fuzzel)" && echo JUEGOS-MENU-$((10+7))', 'JUEGOS-MENU-17')], failures)
        games_capture = pathlib.Path(tempfile.gettempdir()) / 'ultra-juegos-menu.png'
        games_capture.unlink(missing_ok=True)
        mon.sendall(f'screendump {games_capture} -f png\n'.encode())
        for _ in range(50):
            try:
                from PIL import Image
                with Image.open(games_capture) as shot: shot.verify()
                break
            except (OSError, SyntaxError): time.sleep(.1)
        else: raise ValueError('captura del menu de juegos incompleta')
        shutil.copyfile(games_capture, LOG.with_name(LOG.stem + '-juegos.png'))
        mon.sendall(b'sendkey esc\n')
        time.sleep(.4)
        transcript += run(conn, [('test -z "$(pidof fuzzel)" && echo JUEGOS-CERRADO-$((10+8))', 'JUEGOS-CERRADO-18')], failures)
        capture = pathlib.Path(tempfile.gettempdir()) / 'ultra-hyprland.png'
        capture.unlink(missing_ok=True)
        mon.sendall(f'screendump {capture} -f png\n'.encode())
        for _ in range(50):
            try:
                from PIL import Image
                with Image.open(capture) as shot: shot.verify()
                break
            except (OSError, SyntaxError): time.sleep(.1)
        else: raise ValueError('captura de ventanas incompleta')
        shutil.copyfile(capture, LOG.with_name(LOG.stem + '-hyprland.png'))
        # GTK can change argv[0]/comm. Get app PIDs from compositor IPC, not pidof names.
        processes = {'Hyprland': '$(pidof Hyprland)'}
        for app in files:
            name = app.get('class', '').lower()
            for kind in ('foot', 'mousepad', 'thunar'):
                if kind in name and isinstance(app.get('pid'), int) and app['pid'] > 0:
                    processes[kind] = str(app['pid'])
        cmd = '; '.join(f'for p in {pid}; do echo HYPR-CRED-{name}; cat /proc/$p/status; echo HYPR-CRED-FIN; done'
                        for name, pid in processes.items())
        conn.sendall(cmd.encode() + b'; echo HYPR-CRED-LISTO-$((4+7))\n')
        creds = response('HYPR-CRED-LISTO-11')
        transcript += creds
        secured = set()
        for name, status in re.findall(r'^HYPR-CRED-(\w+)\n(.*?)^HYPR-CRED-FIN', creds.replace('\r', ''), re.M | re.S):
            good = all(re.search(rf'^{field}:\s+1000\s+1000\s+1000\s+1000\s*$', status, re.M) for field in ('Uid', 'Gid'))
            good &= all(bool(re.search(rf'^{field}:\s+0+\s*$', status, re.M)) for field in ('CapInh', 'CapPrm', 'CapEff', 'CapBnd', 'CapAmb'))
            good &= bool(re.search(r'^NoNewPrivs:\s+1\s*$', status, re.M))
            if good: secured.add(name)
        expect({'Hyprland', 'foot', 'mousepad', 'thunar'} <= secured,
               'Hyprland y apps: UID/GID 1000, capacidades cero, NNP=1')
        expect(focus_app('thunar'), 'Hyprland: archivos enfocado antes de probar atajos')
        mon.sendall(b'sendkey meta_l-v\n')
        floated = wait_apps(lambda apps: any('thunar' in a.get('class', '').lower() and a.get('floating') for a in apps))
        expect(bool(floated), 'Hyprland: Super+V convierte archivos en ventana flotante')
        mon.sendall(b'sendkey meta_l-shift-2\n')
        moved = wait_apps(lambda apps: any('thunar' in a.get('class', '').lower() and a.get('workspace', {}).get('id') == 2 for a in apps))
        expect(bool(moved), 'Hyprland: mover ventana al escritorio 2')
        mon.sendall(b'sendkey meta_l-2\n')
        time.sleep(1)
        mon.sendall(b'sendkey meta_l-q\n')
        closed = wait_apps(lambda apps: len(apps) >= 2 and not any('thunar' in a.get('class', '').lower() for a in apps))
        expect(bool(closed), 'Hyprland: cambiar escritorio y cerrar solo la ventana activa')
        conn.sendall(b'test -n "$(pidof swaybg)" && echo FONDO-$((2+2)); echo FONDO-FIN-$((4+3))\n')
        bg = response('FONDO-FIN-7')
        transcript += bg
        expect('FONDO-4' in bg, 'Hyprland: fondo vectorial activo')
        # Transfer only test code to this fresh VM. No personal storage is attached.
        encoded = base64.b64encode((ROOT / 'scripts/pantallas-nativo.py').read_bytes()).decode()
        conn.sendall(b': > /arch/tmp/pantallas-nativo.b64; echo PANTALLAS-PREP-$((6+1))\n')
        transcript += response('PANTALLAS-PREP-7')
        # BusyBox's serial input line is bounded; keep transfer commands short.
        for offset in range(0, len(encoded), 200):
            conn.sendall(("printf '%s' '" + encoded[offset:offset+200] +
                          "' >> /arch/tmp/pantallas-nativo.b64; echo PANTALLAS-COPIA-$((6+1))\n").encode())
            transcript += response('PANTALLAS-COPIA-7')
        conn.sendall(b'base64 -d /arch/tmp/pantallas-nativo.b64 > /arch/tmp/pantallas-nativo.py; chmod 644 /arch/tmp/pantallas-nativo.py; echo PANTALLAS-ARCHIVO-$((6+1))\n')
        transcript += response('PANTALLAS-ARCHIVO-7')
        code = 'hl.exec_cmd("python /tmp/pantallas-nativo.py > /run/user/1000/pantallas-prueba.log 2>&1")'
        conn.sendall(('ultra-entrar /usr/bin/env XDG_RUNTIME_DIR=/run/user/1000 /usr/bin/hyprctl -i 0 eval ' +
                      "'"+code+"'; echo PANTALLAS-INICIO-$((6+1))\n").encode())
        transcript += response('PANTALLAS-INICIO-7')
        for _ in range(35):
            conn.sendall(b'ultra-entrar /usr/bin/cat /run/user/1000/pantallas-prueba.log; echo PANTALLAS-RESULTADO-$((6+1))\n')
            result = response('PANTALLAS-RESULTADO-7')
            transcript += result
            if 'PANTALLAS_PRUEBA_OK' in result or 'Traceback' in result: break
            time.sleep(1)
        for marker, label in [('PANTALLAS_CANCEL_OK', 'cancelar restaura el modo real'),
                              ('PANTALLAS_CONFIRM_OK', 'confirmar aplica y guarda el modo real'),
                              ('PANTALLAS_ESCALA_OK', 'escala 1.25 verificada por IPC real'),
                              ('PANTALLAS_RESTORE_OK', 'perfil guardado reaplicado con el comando restaurar'),
                              ('PANTALLAS_PRUEBA_OK', 'configuracion original restaurada')]:
            expect(marker in result, 'Pantallas: ' + label)
        mon.sendall(b'sendkey meta_l-p\n')
        time.sleep(1)
        transcript += run(conn, [('test -n "$(pidof fuzzel)" && echo PANTALLAS-MENU-$((6+1))', 'PANTALLAS-MENU-7')], failures)
        panel_capture = pathlib.Path(tempfile.gettempdir()) / 'ultra-pantallas.png'
        panel_capture.unlink(missing_ok=True)
        mon.sendall(f'screendump {panel_capture} -f png\n'.encode())
        for _ in range(50):
            try:
                from PIL import Image
                with Image.open(panel_capture) as shot: shot.verify()
                break
            except (OSError, SyntaxError): time.sleep(.1)
        else: raise ValueError('captura de pantallas incompleta')
        shutil.copyfile(panel_capture, LOG.with_name(LOG.stem + '-pantallas.png'))
        with Image.open(panel_capture).convert('RGB') as shot:
            # QEMU can return a valid PNG containing only its inactive-output
            # message. IPC state and PNG validity do not prove visible desktop.
            vivid = sum(1 for r, g, b in shot.resize((160, 100)).getdata()
                        if max(r, g, b) - min(r, g, b) > 25)
        expect(vivid > 100, 'Pantallas: selector visible tras restaurar el modo')
        mon.sendall(b'sendkey esc\n')
        time.sleep(.5)
        mon.sendall(b'sendkey meta_l-3\n')
        time.sleep(2)
        background = pathlib.Path(tempfile.gettempdir()) / 'ultra-neon-fondo.png'
        background.unlink(missing_ok=True)
        mon.sendall(f'screendump {background} -f png\n'.encode())
        for _ in range(50):
            try:
                from PIL import Image
                with Image.open(background) as shot: shot.verify()
                break
            except (OSError, SyntaxError): time.sleep(.1)
        else: raise ValueError('captura del fondo incompleta')
        shutil.copyfile(background, LOG.with_name(LOG.stem + '-fondo.png'))
        mon.sendall(b'sendkey meta_l-shift-m\n')
        time.sleep(4)
        conn.sendall(b'echo HYPR-SINC-$((3+4))\n')
        transcript += response('HYPR-SINC-7')
        transcript += run(conn, [('echo HYPR-$(grep -c "hyprland termino (salida 0)" /tmp/hyprland-session.log)', 'HYPR-1'),
                                ('test ! -S /arch/run/seatd.sock && echo HYPR-SEATD-LIMPIO', 'HYPR-SEATD-LIMPIO')], failures)
    finally:
        mon.close()
    return transcript

def auto_flow(start, failures):
    """The generic kernel hands over to the PC's own version by itself, and keeps a way back.

    --auto: generic jumps alone -> tuned kernel, PERFIL DE PC COMPLETO -> generic jumps
    again -> tuned confirms -> generic with a key pressed stays.
    --auto-falla: generic with a key stays; the tuned kernel is broken on purpose; generic
    jumps; the firmware cannot start it and falls back; generic marks fallido, deletes its entry.
    """
    t, procs = '', []
    WAIT = 'cualquier tecla para quedarse'
    def boot(label, marker):
        nonlocal t
        proc, conn = start()
        procs.append(proc)
        out = read_until(conn, marker, 120)
        t += f'\n===== {label} =====\n' + out
        return proc, conn, out
    def jumped(proc, conn):
        # The init reboots on its own after the countdown; -no-reboot then ends QEMU.
        nonlocal t
        t += read_until(conn, 'Restarting system', 30)
        proc.wait(timeout=20)
        conn.close()
    def stay(conn):
        # One key on the serial console, during the countdown.
        nonlocal t
        conn.sendall(b'q')
        out = read_until(conn, 'ultra:', 60)
        t += out
        return out
    def off(proc, conn):
        nonlocal t
        conn.sendall(b'poweroff\n')
        t += read_until(conn, 'APAGADO', 10)
        proc.wait(timeout=15)
        conn.close()
    try:
        if not FALLA:
            p, c, o = boot('ARRANQUE 1: generico', WAIT)
            expect_boot(o, '[KERNEL]: Linux 7.3.0-rc7-g', 'arranque 1: el firmware arranco el generico', failures)
            expect_boot(o, 'tu version propia (AMD', 'arranque 1: version propia preparada', failures)
            jumped(p, c)
            p, c, o = boot('ARRANQUE 2: version propia', 'ultra:')
            expect_boot(o, '[KERNEL]: Linux 7.3.0-rc7-pc-g', 'arranque 2: salto solo a la version propia', failures)
            expect_boot(o, 'PERFIL DE PC COMPLETO: tu version propia arranco bien', 'arranque 2: PERFIL DE PC COMPLETO', failures)
            check_screen(failures)
            off(p, c)
            p, c, o = boot('ARRANQUE 3: generico, salta solo', WAIT)
            expect_boot(o, '[KERNEL]: Linux 7.3.0-rc7-g', 'arranque 3: el generico por defecto', failures)
            jumped(p, c)
            p, c, o = boot('ARRANQUE 4: version propia otra vez', 'ultra:')
            expect_boot(o, '[KERNEL]: Linux 7.3.0-rc7-pc-g', 'arranque 4: salto de nuevo a la version propia', failures)
            expect_boot(o, 'salto confirmado', 'arranque 4: la version propia confirma el salto', failures)
            off(p, c)
            p, c, o = boot('ARRANQUE 5: generico, con tecla', WAIT)
            o = stay(c)
            expect_boot(o, 'te quedas en el generico', 'arranque 5: una tecla deja el generico', failures)
            t += run(c, [('cat /datos/perfil.estado', 'estado=bueno')], failures)
            off(p, c)
        else:
            p, c, o = boot('ARRANQUE 1: generico, con tecla', WAIT)
            o = stay(c)
            expect_boot(o, 'te quedas en el generico', 'arranque 1: una tecla deja el generico', failures)
            # A tuned kernel the firmware cannot start.
            t += run(c, [('echo roto > /bmo/EFI/ULTRA/ULTRA-PC.EFI; sync; echo ROTO-$((1+1))', 'ROTO-2')], failures)
            c.sendall(b'reboot\n')
            t += read_until(c, 'Restarting system', 20)
            p.wait(timeout=20)
            c.close()
            p, c, o = boot('ARRANQUE 2: generico salta a un kernel roto', WAIT)
            jumped(p, c)
            p, c, o = boot('ARRANQUE 3: el firmware vuelve al generico', 'ultra:')
            expect_boot(o, '[KERNEL]: Linux 7.3.0-rc7-g', 'arranque 3: el firmware volvio al generico', failures)
            expect_boot(o, 'no llego a arrancar; se queda el generico y su entrada F11 se borro',
                        'arranque 3: fallo detectado y entrada de ULTRA borrada', failures)
            t += run(c, [('cat /datos/perfil.estado', 'estado=fallido')], failures)
            off(p, c)
        print('VERDE [poweroff]: init apago la maquina y QEMU termino')
    except Exception as exc:
        t += getattr(exc, 'partial', '')
        failures.append(str(exc))
        print('ROJO', exc)
    finally:
        for proc in procs:
            if proc.poll() is None: proc.kill()
    return t

def main():
    check_kernel()
    if DISCO:
        vars_copy = BUILD / 'ovmf-vars-prueba.fd'
        shutil.copyfile(QEMU_SHARE / 'edk2-i386-vars.fd', vars_copy)
        # A fresh copy every run: the first boot must really be a first boot.
        shutil.copyfile(IMG, IMG_PRUEBA)
        boot = ['-drive', f'if=pflash,format=raw,readonly=on,file={QEMU_SHARE / "edk2-x86_64-code.fd"}',
                '-drive', f'if=pflash,format=raw,file={vars_copy}',
                '-drive', f'file={IMG_PRUEBA},format=raw,if=virtio', '-device', 'intel-hda']
    else:
        boot = ['-kernel', str(KERNEL), '-append', 'console=ttyS0,115200 rdinit=/init panic=-1 noapic quiet '
                'GLIBC_TUNABLES=glibc.cpu.hwcaps=SHSTK:glibc.cpu.x86_shstk=on']
    def start():
        proc = subprocess.Popen([str(QEMU), '-machine', 'pc', '-accel', 'whpx', '-cpu', 'host', '-smp', '1', '-m', '2048',
                                 *boot, '-vga', 'virtio', '-nic', 'user,model=virtio-net-pci' if BRAVE else 'none', '-display', 'none', '-no-reboot',
                                 '-serial', f'tcp:127.0.0.1:{PORT},server=on,wait=on',
                                 '-monitor', f'tcp:127.0.0.1:{PORT + 1},server=on,wait=off'],
                                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for _ in range(50):
            try:
                conn = socket.create_connection(('127.0.0.1', PORT), timeout=1); break
            except OSError: time.sleep(0.2)
        else:
            proc.kill(); raise ConnectionError('QEMU no abrio la consola serie')
        conn.settimeout(0.5)
        return proc, conn
    if AUTO:
        failures = []
        transcript = auto_flow(start, failures)
        LOG.write_text(transcript, encoding='utf-8')
        sys.exit(1 if failures else 0)
    transcript, failures = '', []
    qemu = None
    try:
        qemu, conn = start()
        # Firmware start-up adds seconds before the kernel runs.
        first = read_until(conn, 'ultra:', 120 if DISCO else 30)
        transcript += first
        transcript += run(conn, CHECKS, failures)
        if DISCO:
            expect_boot(first, 'VERDE [PARTICION]: ULTRA-DATOS', 'arranque 1: ULTRA-DATOS montada por nombre', failures)
            expect_boot(first, 'primer arranque en este PC', 'arranque 1: deteccion completa', failures)
            # The boot card, plain on the serial port: guardians asked to the kernel, not assumed.
            expect_boot(first, '[TARJETA]: ', 'arranque 1: tarjeta de arranque', failures)
            expect_boot(first, ' CET Landlock seccomp FORTIFY', 'arranque 1: los cuatro guardianes activos', failures)
            # The ULTRA logo drawn on the framebuffer, top-right corner.
            expect_boot(first, 'VERDE [LOGO]: 248x300 en la pantalla', 'arranque 1: logo en la pantalla', failures)
            # Wayland stage W1: the init mounts Arch's base system as a root at /arch.
            expect_boot(first, 'VERDE [SISTEMA]: base Arch en /arch', 'arranque 1: sistema base de Arch montado', failures)
            transcript += run(conn, BOOT1, failures)
            transcript += check_menu(conn, failures)
            # Wayland stage W2: the first window, seen and closed through the emulated hardware.
            transcript += check_window(conn, failures)
            transcript += check_hyprland(conn, failures)
            if BMO:
                transcript += run(conn, [('test -s /bmo/ULTRA/informe/gpu-$(uname -r).txt && echo GPU-INFORME-$((2+2))', 'GPU-INFORME-4'),
                                        ('grep -q "^AUSENTE /dev/nvidiactl$" /bmo/ULTRA/informe/gpu-fallo-$(uname -r).txt && echo GPU-FALLO-$((2+3))', 'GPU-FALLO-5'),
                                        ('grep -Eq "^RAM total_kB=[0-9]+ disponible_kB=[0-9]+ usada_estimada_kB=[0-9]+" /bmo/ULTRA/informe/gpu-$(uname -r)-software-hyprland.txt && echo RAM-MEDIDA-$((3+3))', 'RAM-MEDIDA-6'),
                                        ('grep -q "^PROCESO .*nombre=Hyprland .*pss_kB=[0-9]" /bmo/ULTRA/informe/gpu-$(uname -r)-software-hyprland.txt && echo PSS-CLIENTE-$((3+4))', 'PSS-CLIENTE-7'),
                                        ('grep -q "^GPU NA modo_software_o_nodos_ausentes" /bmo/ULTRA/informe/gpu-$(uname -r)-software-hyprland.txt && test ! -S /arch/run/seatd.sock && echo OBSERVADOR-FINAL-$((4+4))', 'OBSERVADOR-FINAL-8'),
                                        ('grep -q "ETHERNET: estado posterior" /bmo/ULTRA/informe/gpu-$(uname -r)-software-hyprland.txt && grep -q "contadores globales acumulados" /bmo/ULTRA/informe/gpu-$(uname -r)-software-hyprland.txt && echo RED-EXPORTADA-$((4+5))', 'RED-EXPORTADA-9')], failures)
                if BRAVE:
                    transcript += run(conn, [('grep -Eq "^RED iface=.* rx_bytes=[0-9]+ rx_packets=[0-9]+" /bmo/ULTRA/informe/gpu-$(uname -r)-software-hyprland.txt && echo RED-C-MEDIDA-$((5+5))', 'RED-C-MEDIDA-10')], failures)
                # Extract the actual measurements, without exposing BMO to apps.
                conn.sendall(b'busybox base64 /bmo/ULTRA/informe/gpu-$(uname -r)-software-hyprland.txt\n')
                encoded = read_until(conn, 'ultra:', 30)
                transcript += encoded
                lines = re.findall(r'^[A-Za-z0-9+/]{4,}={0,2}$', encoded.replace('\r', ''), re.M)
                report = base64.b64decode(''.join(lines), validate=True)
                if not report.startswith(b'ULTRA GPU - ultimo intento\n'):
                    raise ValueError('informe de medicion no extraido correctamente')
                (BUILD / 'gpu-qemu-software-hyprland.txt').write_bytes(report)
            # The guest reboots through init (unmount, RB_AUTOBOOT). Under WHPX a guest reset
            # leaves the vCPU stuck in the firmware, so -no-reboot ends QEMU and a second QEMU
            # boots the same disk: whatever /datos shows then came from the disk itself.
            conn.sendall(b'reboot\n')
            transcript += read_until(conn, 'Restarting system', 20)
            qemu.wait(timeout=20)
            expect_boot(transcript, 'VERDE [DESMONTAJE]: /datos desmontado limpio', 'reinicio: /datos desmontado limpio', failures)
            if BMO:
                expect_boot(transcript, 'VERDE [DESMONTAJE]: /bmo desmontado limpio', 'reinicio: /bmo desmontado limpio', failures)
            conn.close()
            qemu, conn = start()
            second = read_until(conn, 'ultra:', 120)
            transcript += '\n===== ARRANQUE 2 =====\n' + second
            expect_boot(second, 'arranque guiado por PC.configuracion', 'arranque 2: guiado por PC.configuracion', failures)
            # The curious fact on screen: this boot against the previous one of the same kernel.
            expect_boot(second, 'ms; el anterior con este kernel:', 'arranque 2: comparado con el anterior', failures)
            transcript += run(conn, BOOT2, failures)
            # The serial port is not the screen: a PC shows only what reaches tty1.
            check_screen(failures)
        conn.sendall(b'poweroff\n')
        transcript += read_until(conn, 'APAGADO', 10)
        qemu.wait(timeout=15)
        print('VERDE [poweroff]: init apago la maquina y QEMU termino')
    except Exception as exc:
        transcript += getattr(exc, 'partial', '')
        failures.append(str(exc)); print('ROJO', exc)
    finally:
        if qemu and qemu.poll() is None: qemu.kill()
        LOG.write_text(transcript, encoding='utf-8')
    sys.exit(1 if failures else 0)

if __name__ == '__main__':
    main()
