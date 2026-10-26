# SPDX-License-Identifier: GPL-2.0-only
"""Boot the downloaded kernel headless, drive its shell over serial and check the answers."""
import base64, hashlib, json, pathlib, re, secrets, shlex, shutil, socket, subprocess, sys, tempfile, time

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
RAM_MIB = 2048
if '--ram-mib' in sys.argv[2:]:
    try: RAM_MIB = int(sys.argv[sys.argv.index('--ram-mib')+1])
    except (ValueError, IndexError): raise SystemExit('Falta RAM valida para el banco')
    if not 2048 <= RAM_MIB <= 8192: raise SystemExit('RAM del banco: entre 2048 y 8192 MiB')
DISCO = BMO or '--disco' in sys.argv[2:]
QEMU_SHARE = pathlib.Path(r'C:\Program Files\qemu\share')
BUILD = ROOT / 'build' / {'produccion': 'baseline', 'vigia': 'vigia', 'generico': 'generico', 'pc': 'pc'}[PERFIL]
# Test a new delivery without replacing a disk image in use by another VM.
if '--artefactos' in sys.argv[2:]:
    index=sys.argv.index('--artefactos')
    if index+1>=len(sys.argv):raise SystemExit('Falta el directorio de artefactos')
    candidate=pathlib.Path(sys.argv[index+1])
    if not candidate.is_absolute():candidate=ROOT/candidate
    resolved=candidate.resolve()
    if resolved!=candidate.absolute() or not resolved.is_relative_to(ROOT/'build') or not resolved.is_dir():
        raise SystemExit('Artefactos fuera de build, redirigidos o ausentes')
    BUILD=resolved
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
              ('ultra-entrar /usr/bin/setpriv --reuid=1000 --regid=1000 --clear-groups --no-new-privs --bounding-set=-all /usr/local/libexec/ultra-red-contadores', 'RED '),
              ('ultra-entrar /usr/bin/setpriv --reuid=1000 --regid=1000 --clear-groups --no-new-privs --bounding-set=-all /usr/local/libexec/ultra-recursos-contadores', 'MEM MemTotal:'),
              ('ultra-entrar /usr/local/libexec/ultra-medir-recursos --inventario', 'fin_recursos snapshot_no_atomico=1')]
    BOOT1 += [('cat /proc/pressure/cpu /proc/pressure/memory /proc/pressure/io', 'some avg10='),
              ('test -r /proc/pressure/cpu && test -r /proc/pressure/memory && test -r /proc/pressure/io && echo PSI-TRES-$((4+5))', 'PSI-TRES-9'),
              ("ultra-entrar /usr/local/libexec/ultra-medir-recursos | grep '^PSI recurso=memory some'", 'total=')]
    # Disposable single-vCPU guest only: a bounded, unprivileged CPU load must
    # increase the real stall counter, not merely find a pressure file.
    psi_load = r'''read -r before < /proc/pressure/cpu
before=${before##*total=}
pids=()
for i in 1 2 3 4; do
    timeout -k 1 3 bash -c 'while :; do :; done' & pids+=("$!")
done
for pid in "${pids[@]}"; do wait "$pid" || :; done
read -r after < /proc/pressure/cpu
after=${after##*total=}
[[ $before =~ ^[0-9]+$ && $after =~ ^[0-9]+$ ]] || exit 2
((after > before)) || exit 1
printf 'PSI-CRECIO-%s\n' "$((4+5))"
'''
    BOOT1 += [('ultra-entrar /usr/bin/setpriv --reuid=1000 --regid=1000 --clear-groups --no-new-privs --bounding-set=-all /usr/bin/timeout -k 1 8 /usr/bin/bash -c '+shlex.quote(psi_load), 'PSI-CRECIO-9')]
    # QEMU has no NVIDIA: explicit acceleration must reject, never pretend software is GPU.
    BOOT1 += [('ultra-escritorio --nvidia 2>&1', 'ROJO [GPU]: se requiere exactamente un dispositivo DRM NVIDIA'),
              ('ultra-escritorio --nvidia >/tmp/gpu-rechazo.log 2>&1; echo GPU-ESTADO-$?', 'GPU-ESTADO-1')]
    if BRAVE:
        BOOT1 += [('ultra-entrar /opt/brave/brave --version', '1.97.59'),
                  ('test -e /proc/sys/user/max_user_namespaces && echo USERNS-DISPONIBLE-$((4+4))', 'USERNS-DISPONIBLE-8')]
if '--gog-cyberpunk' in sys.argv:
    # Disposable ext4 + overlay wholly inside the guest; no host game or account.
    BOOT1 += [
        ('cat /sistema/etc/ultra/heronite-version; test ! -e /sistema/opt/heroic/heroic && echo SIN-ELECTRON-HEROIC', '0.15.0'),
        ('ultra-entrar /usr/bin/truncate -s 64M /tmp/gog-fixture.ext4; ultra-entrar /usr/bin/mkfs.ext4 -q -F /tmp/gog-fixture.ext4; echo GOG-FS-$?', 'GOG-FS-0'),
        ('ultra-entrar /usr/bin/bash -c \'set -e; umask 022; mkdir -p /juegos; mount -o loop,nosuid,nodev /tmp/gog-fixture.ext4 /juegos; mkdir -p /juegos/usuarios/ultra /juegos/overlay/upper /juegos/overlay/work /juegos/Cyberpunk2077 /tmp/gog-lower/bin/x64; printf MZ-fixture > /tmp/gog-lower/bin/x64/Cyberpunk2077.exe; chown -R 1000:1000 /juegos/usuarios; echo GOG-FIXTURE-$((6*7))\'', 'GOG-FIXTURE-42'),
        ('ultra-entrar /usr/bin/bash -c '+shlex.quote('printf \'%s\' \'{"gameId":"1423049311","rootGameId":"1423049311","buildId":"12345","languages":["en-US"]}\' > /tmp/gog-lower/goggame-1423049311.info; chmod 644 /tmp/gog-lower/goggame-1423049311.info; mount -t overlay overlay -o lowerdir=/tmp/gog-lower,upperdir=/juegos/overlay/upper,workdir=/juegos/overlay/work /juegos/Cyberpunk2077; echo GOG-OVERLAY-$?'), 'GOG-OVERLAY-0'),
        ('ultra-entrar /usr/bin/setpriv --reuid=1000 --regid=1000 --clear-groups --no-new-privs --bounding-set=-all /usr/bin/env HOME=/juegos/usuarios/ultra /usr/bin/heronite-rs --help', '--launch'),
        ('ultra-entrar /usr/bin/setpriv --reuid=1000 --regid=1000 --clear-groups --no-new-privs --bounding-set=-all /usr/bin/env HOME=/juegos/usuarios/ultra /usr/local/bin/ultra-gog-cyberpunk', 'Cyberpunk GOG registrado'),
        ('ultra-entrar /usr/bin/setpriv --reuid=1000 --regid=1000 --clear-groups --no-new-privs --bounding-set=-all /usr/bin/env HOME=/juegos/usuarios/ultra /usr/local/bin/ultra-gog-cyberpunk', 'Cyberpunk GOG ya reconocido'),
        ('ultra-entrar /usr/bin/bash -c \'test ! -e /juegos/usuarios/ultra/.config/heroic/gog_store/auth.json && test ! -e /tmp/gog-lower/.config && echo GOG-SIN-CUENTAS-$((6*7)); umount /juegos/Cyberpunk2077; umount /juegos\'', 'GOG-SIN-CUENTAS-42'),
    ]
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
    with its configured background; then "exit" typed on the PS/2 keyboard must reach foot
    through udev, libinput and Wayland, and cage must end cleanly.
    """
    t = ''
    conn.sendall(b'ultra-escritorio --auto > /tmp/escritorio.log 2>&1 &\n')
    t += read_until(conn, 'ultra:', 10)
    tmp = pathlib.Path(tempfile.gettempdir())
    colors=re.findall(r'^background=([0-9a-fA-F]{6})\s*$',(ROOT/'src/rootfs/desktop/foot.ini').read_text(),re.M)
    if len(colors)!=1:raise ValueError('color de fondo foot ausente o ambiguo')
    background=tuple(int(colors[0][i:i+2],16) for i in (0,2,4))
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
                       if all(abs(pixels[i + k] - background[k]) <= 6 for k in range(3)))
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
        colores = ['5ad8ff','00f0ff','ff2a4f','3dff9a','c03bff','fcee0a','ff7a90']
        def contar(path, color):
            target = tuple(int(color[i:i+2],16) for i in (0,2,4))
            with Image.open(path).convert('RGB') as shot:
                return sum(count for count, rgb in shot.getcolors(shot.width*shot.height)
                           if max(abs(a-b) for a,b in zip(rgb,target)) < 7)
        # El fondo Neon usa los mismos colores: se compara contra el escritorio sin menu.
        base_capture = pathlib.Path(tempfile.gettempdir()) / 'ultra-categoria-base.png'
        base_capture.unlink(missing_ok=True)
        mon.sendall(f'screendump {base_capture} -f png\n'.encode())
        for _ in range(50):
            try:
                from PIL import Image
                with Image.open(base_capture) as shot: shot.verify()
                break
            except (OSError, SyntaxError): time.sleep(.1)
        else: raise ValueError('captura base de categorias incompleta')
        base_pixels = {color: contar(base_capture, color) for color in colores}
        menu_checks = 0
        def esperar_menu(prompt):
            nonlocal transcript, menu_checks
            menu_checks += 1
            marker = f'MENU-LISTO-{60+menu_checks}'
            command = ("for intento in $(seq 1 50); do listo=; "
                       "for p in $(pidof fuzzel); do "
                       f"if grep -aqF -- {shlex.quote(prompt)} /proc/$p/cmdline; then listo=1; fi; "
                       "done; if test -n \"$listo\"; then "
                       f"echo MENU-LISTO-$((60+{menu_checks})); break; fi; sleep .1; done")
            transcript += run(conn, [(command, marker)], failures)
            # El proceso existe antes de que Wayland termine de dar foco al menu.
            time.sleep(.5)
        prompts = ['Aplicaciones / ACENTO', 'Discos / CYAN', 'Juegos / ROJO',
                   'Pantalla / VERDE', 'Audio / VIOLETA', 'Ambiente / AMARILLO', 'Sistema / CORAL']
        for number, color in enumerate(colores, 1):
            mon.sendall(b'sendkey meta_l-spc\n')
            esperar_menu('BMO-X // Categorias')
            mon.sendall(f'sendkey {number}\n'.encode()); time.sleep(.15)
            mon.sendall(b'sendkey ret\n')
            esperar_menu(prompts[number-1])
            category_capture = pathlib.Path(tempfile.gettempdir()) / f'ultra-categoria-{number}.png'
            # Un proceso listo no garantiza que su primer cuadro ya se presento.
            # Mantener el mismo umbral visual, esperando el cuadro hasta 10 s.
            deadline = time.monotonic() + 10
            pixels = 0
            while time.monotonic() < deadline:
                category_capture.unlink(missing_ok=True)
                mon.sendall(f'screendump {category_capture} -f png\n'.encode())
                for _ in range(50):
                    try:
                        from PIL import Image
                        with Image.open(category_capture) as shot: shot.verify()
                        break
                    except (OSError, SyntaxError): time.sleep(.1)
                else: raise ValueError('captura de categoria incompleta')
                pixels = contar(category_capture, color) - base_pixels[color]
                if pixels > 40: break
                time.sleep(.2)
            expect(pixels > 40, f'Categoria {number}: color propio visible')
            shutil.copyfile(category_capture,LOG.with_name(LOG.stem+f'-categoria-{number}.png'))
            mon.sendall(b'sendkey esc\n')  # Submenu -> root.
            esperar_menu('BMO-X // Categorias')
            transcript += run(conn, [('test -n "$(pidof fuzzel)" && echo CATEGORIA-RAIZ-$((1+1))', 'CATEGORIA-RAIZ-2')], failures)
            mon.sendall(b'sendkey esc\n'); time.sleep(.5)
        # Drive the actual Super+Space menu. A fresh VM has no personal storage;
        # its first panel must explain the temporary HOME instead of closing.
        mon.sendall(b'sendkey meta_l-spc\n')
        time.sleep(.7)
        for key in ('d','i','s','c','o','s','ret'):
            mon.sendall(('sendkey '+key+'\n').encode()); time.sleep(.12)
        time.sleep(.6)
        mon.sendall(b'sendkey ret\n')  # First action in the Discos category.
        disk_apps = wait_apps(lambda apps: any(a.get('title')=='ULTRA / Discos y bibliotecas' for a in apps))
        expect(bool(disk_apps), 'Discos: Super+Space abre el panel')
        mon.sendall(b'sendkey esc\n'); time.sleep(.5)
        # Only this disposable VM's RAM tree: no host or personal disk attached.
        transcript += run(conn, [('mkdir -p /arch/juegos/usuarios/ultra; chown 1000:1000 /arch/juegos/usuarios/ultra; echo DISCOS-HOME-$((9+1))', 'DISCOS-HOME-10')], failures)
        code = 'hl.exec_cmd("foot --title=ULTRA-Discos-Prueba env HOME=/juegos/usuarios/ultra ultra-discos panel")'
        conn.sendall(('ultra-entrar /usr/bin/env XDG_RUNTIME_DIR=/run/user/1000 /usr/bin/hyprctl -i 0 eval '+shlex.quote(code)+'; echo DISCOS-LANZADO-$((9+2))\n').encode())
        transcript += response('DISCOS-LANZADO-11')
        disk_apps = wait_apps(lambda apps: any(a.get('title')=='ULTRA-Discos-Prueba' for a in apps))
        expect(bool(disk_apps), 'Discos: panel con home persistente simulado')
        time.sleep(3)
        transcript += run(conn, [('test -s /arch/juegos/usuarios/ultra/.cache/ultra/discos/automatico.json && echo DISCOS-AUTO-$((9+3))', 'DISCOS-AUTO-12')], failures)
        disk_capture = pathlib.Path(tempfile.gettempdir()) / 'ultra-discos.png'
        disk_capture.unlink(missing_ok=True)
        mon.sendall(f'screendump {disk_capture} -f png\n'.encode())
        for _ in range(50):
            try:
                from PIL import Image
                with Image.open(disk_capture) as shot: shot.verify()
                break
            except (OSError, SyntaxError): time.sleep(.1)
        else: raise ValueError('captura de discos incompleta')
        shutil.copyfile(disk_capture, LOG.with_name(LOG.stem + '-discos.png'))
        mon.sendall(b'sendkey esc\n'); time.sleep(.5)
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
        if '--diagnostico-warframe' in sys.argv:
            # The desktop has a private mount namespace (ultra-raiz). A plain
            # chroot from the serial console cannot see its new frame tmpfs.
            # Join that namespace before dropping every user capability.
            user_diag='ultra-entrar /usr/bin/nsenter --target $(pidof Hyprland) --mount --root --wd -- /usr/bin/setpriv --reuid=1000 --regid=1000 --clear-groups --no-new-privs --bounding-set=-all --inh-caps=-all --ambient-caps=-all /usr/bin/env HOME=/juegos/usuarios/ultra XDG_RUNTIME_DIR=/run/user/1000 '
            transcript += run(conn,[(user_diag+'/usr/local/bin/ultra-diagnostico-juego preparar; echo DIAG-PREPARADO-$?', 'DIAG-PREPARADO-0')],failures)
            source=(ROOT/'scripts/banco_diagnostico_warframe.py').read_bytes()
            encoded=base64.b64encode(source).decode()
            conn.sendall(b': > /arch/tmp/diag-probe.b64; echo DIAG-CARGA-$((40+2))\n')
            transcript += response('DIAG-CARGA-42')
            for offset in range(0,len(encoded),192):
                conn.sendall(("printf '%s' '"+encoded[offset:offset+192]+"' >> /arch/tmp/diag-probe.b64; echo DIAG-CARGA-$((40+2))\n").encode())
                transcript += response('DIAG-CARGA-42')
            conn.sendall(b'busybox base64 -d /arch/tmp/diag-probe.b64 > /arch/tmp/diag-probe.py; echo DIAG-CARGA-$((40+2))\n')
            transcript += response('DIAG-CARGA-42')
            conn.sendall((user_diag+'/usr/bin/python /tmp/diag-probe.py resources; echo DIAG-RECURSOS-RC-$?; echo DIAG-RESULTADO-$((40+2))\n').encode())
            result=response('DIAG-RESULTADO-42',timeout=30); transcript+=result
            for marker,label in [('DIAG-KERNEL-SYSLOG-42','lector kernel con capacidad SYSLOG y canal de salida numerico'),
                                 ('DIAG-RAM-64M-42','cuadros en tmpfs de 64 MiB con restricciones'),
                                 ('DIAG-RECURSOS-UID1000-42','grupo ficticio AppID230410 con CPU/hilos reales y sin exportar secretos'),
                                 ('DIAG-RECURSOS-RC-0','captura de recursos acotada termina correctamente')]:
                expect(marker in result,'Diagnostico: '+label)
            code='hl.exec_cmd("ULTRA_DIAGNOSTICO_HUD=1 MANGOHUD_CONFIG=read_cfg,autostart_log=1,log_duration=3 ultra-hud opengl timeout -k 1 7 glxgears")'
            conn.sendall(('ultra-entrar /usr/bin/setpriv --reuid=1000 --regid=1000 --clear-groups /usr/bin/env XDG_RUNTIME_DIR=/run/user/1000 /usr/bin/hyprctl -i 0 eval '+shlex.quote(code)+'; echo DIAG-GL-$((40+2))\n').encode())
            transcript+=response('DIAG-GL-42'); time.sleep(9)
            conn.sendall((user_diag+'/usr/bin/python /tmp/diag-probe.py frames; echo DIAG-CUADROS-RC-$?; echo DIAG-RESULTADO-$((40+2))\n').encode())
            result=response('DIAG-RESULTADO-42',timeout=20); transcript+=result
            expect('DIAG-CUADROS-OPENGL-42' in result,'Diagnostico: CSV por cuadro real de OpenGL software')
            expect('DIAG-CSV-ARCHIVADO-42' in result and 'DIAG-CUADROS-RC-0' in result,'Diagnostico: resumen y copia acotada del CSV de prueba')
            mon.sendall(b'sendkey meta_l-ctrl-f11\n'); time.sleep(1)
            panel=wait_apps(lambda apps:any(a.get('title')=='ULTRA / Warframe diagnostico' for a in apps))
            expect(bool(panel),'Diagnostico: Super+Ctrl+F11 abre el panel de recursos')
            if panel:
                target=pathlib.Path(tempfile.gettempdir())/'ultra-diagnostico-panel.png'
                target.unlink(missing_ok=True)
                mon.sendall(f'screendump {target} -f png\n'.encode())
                for _ in range(50):
                    try:
                        with Image.open(target) as shot: shot.verify()
                        break
                    except (OSError,SyntaxError): time.sleep(.1)
                else: raise ValueError('captura del diagnostico incompleta')
                shutil.copyfile(target,LOG.with_name(LOG.stem+'-diagnostico.png'))
        if '--puntero' in sys.argv:
            encoded = base64.b64encode((ROOT/'scripts/banco_puntero.py').read_bytes()).decode()
            conn.sendall(b': > /arch/tmp/puntero-probe.b64; echo PUNTERO-CARGA-$((40+4))\n')
            transcript += response('PUNTERO-CARGA-44')
            for offset in range(0, len(encoded), 192):
                conn.sendall(("printf '%s' '"+encoded[offset:offset+192]+"' >> /arch/tmp/puntero-probe.b64; echo PUNTERO-CARGA-$((40+4))\n").encode())
                transcript += response('PUNTERO-CARGA-44')
            conn.sendall(b'busybox base64 -d /arch/tmp/puntero-probe.b64 > /arch/tmp/puntero-probe.py; echo PUNTERO-CARGA-$((40+4))\n')
            transcript += response('PUNTERO-CARGA-44')
            command = 'python /tmp/puntero-probe.py > /tmp/puntero-resultado.txt 2>&1'
            conn.sendall(('ultra-entrar /usr/bin/env XDG_RUNTIME_DIR=/run/user/1000 /usr/bin/hyprctl -i 0 eval '
                          + "'hl.exec_cmd(" + json.dumps(command) + ")'; echo PUNTERO-INICIO-$((40+4))\n").encode())
            transcript += response('PUNTERO-INICIO-44')
            result = ''
            clicked = False
            for _ in range(30):
                conn.sendall(b'cat /arch/tmp/puntero-resultado.txt; echo PUNTERO-LECTURA-$((40+4))\n')
                result = response('PUNTERO-LECTURA-44'); transcript += result
                if 'PUNTERO-ESPERA-CLICK' in result and not clicked:
                    mon.sendall(b'mouse_button 1\n'); time.sleep(.15)
                    mon.sendall(b'mouse_button 0\n'); clicked = True
                if re.search(r'(?m)^PUNTERO-RC-[0-9]+\r?$', result): break
                if 'Traceback (most recent call last)' in result: break
                time.sleep(1)
            for marker in ('PUNTERO-PERFILES-OK', 'PUNTERO-XWAYLAND-LAUNCHER-OK',
                           'PUNTERO-CLICK-XWAYLAND-OK', 'PUNTERO-XWAYLAND-JUEGO-OK',
                           'PUNTERO-TRANSPARENTE-REPARADO-OK', 'PUNTERO-JUEGO-NO-ALTERADO-OK',
                           'PUNTERO-COMPATIBLE-OK', 'PUNTERO-RC-0'):
                expect(marker in result, 'Puntero: ' + marker)
        if '--tema-nucleo' in sys.argv:
            source=(ROOT/'scripts/banco_tema_nucleo.py').read_bytes()
            encoded=base64.b64encode(source).decode()
            conn.sendall(b': > /arch/tmp/nucleo-probe.b64; echo NUCLEO-CARGA-$((40+4))\n')
            transcript+=response('NUCLEO-CARGA-44')
            for offset in range(0,len(encoded),192):
                conn.sendall(("printf '%s' '"+encoded[offset:offset+192]+"' >> /arch/tmp/nucleo-probe.b64; echo NUCLEO-CARGA-$((40+4))\n").encode())
                transcript+=response('NUCLEO-CARGA-44')
            conn.sendall(b'busybox base64 -d /arch/tmp/nucleo-probe.b64 > /arch/tmp/nucleo-probe.py; echo NUCLEO-CARGA-$((40+4))\n')
            transcript+=response('NUCLEO-CARGA-44')
            command='ultra-entrar /usr/bin/nsenter --target $(pidof Hyprland) --mount --root --wd -- /usr/bin/setpriv --reuid=1000 --regid=1000 --clear-groups --no-new-privs --bounding-set=-all --inh-caps=-all --ambient-caps=-all /usr/bin/env HOME=/home/ultra /usr/bin/python /tmp/nucleo-probe.py; echo NUCLEO-RC-$?; echo NUCLEO-FIN-$((40+4))\n'
            conn.sendall(command.encode()); result=response('NUCLEO-FIN-44',timeout=20); transcript+=result
            for marker,label in [('NUCLEO-CURSORES-OK','11 cursores XCursor con 5 tamanos y hotspots validos'),
                                 ('NUCLEO-ICONOS-OK','iconos vectoriales y herencia Papirus'),
                                 ('NUCLEO-EQUIPO-OK','hardware real y JSON de barra sin root'),
                                 ('NUCLEO-SESION-OK','GTK usa el tema y cursor nuevos'),
                                 ('NUCLEO-CURSOR-APLICADO','Hyprland acepta aplicar el cursor propio'),
                                 ('NUCLEO-RESPALDO-OK','migracion conserva respaldo y cambios posteriores'),
                                 ('NUCLEO-RC-0','banco visual termina sin errores')]:
                expect(marker in result,'Nucleo: '+label)
            if '--diagnostico-warframe' in sys.argv:
                mon.sendall(b'sendkey meta_l-q\n'); time.sleep(.5)
            def tarjetas_visibles():
                nonlocal transcript
                conn.sendall(b'ultra-entrar /usr/bin/env XDG_RUNTIME_DIR=/run/user/1000 /usr/bin/hyprctl -i 0 -j layers; echo TARJETAS-CAPAS-$((50+1))\n')
                capas=response('TARJETAS-CAPAS-51'); transcript+=capas
                return re.search(r'"namespace"\s*:\s*"ultra-tarjetas"',capas) is not None
            if tarjetas_visibles():
                # Con eww: las tarjetas abren al iniciar; Super+F9 las oculta y las vuelve a mostrar.
                expect(True,'Nucleo: tarjetas eww abiertas al iniciar')
                mon.sendall(b'sendkey meta_l-f9\n')
                deadline=time.monotonic()+15
                ocultas=False
                while time.monotonic()<deadline:
                    ocultas=not tarjetas_visibles()
                    if ocultas: break
                    time.sleep(.25)
                expect(ocultas,'Nucleo: Super+F9 oculta las tarjetas')
                mon.sendall(b'sendkey meta_l-f9\n')
                deadline=time.monotonic()+15
                equipment=False
                while time.monotonic()<deadline:
                    equipment=tarjetas_visibles()
                    if equipment: break
                    time.sleep(.25)
                if not equipment:
                    transcript += run(conn, [('head -c 4096 /arch/run/user/1000/ultra-tarjetas.log; echo TARJETAS-LOG-$((50+2))', 'TARJETAS-LOG-52')], failures)
                expect(equipment,'Nucleo: Super+F9 vuelve a mostrar las tarjetas')
            else:
                # Sin eww: Super+F9 abre la version de texto.
                mon.sendall(b'sendkey meta_l-f9\n'); time.sleep(1)
                equipment=wait_apps(lambda apps:any(a.get('title')=='ULTRA / Equipo en vivo' for a in apps))
                expect(bool(equipment),'Nucleo: Super+F9 abre equipo en vivo')
            if equipment:
                target=pathlib.Path(tempfile.gettempdir())/'ultra-equipo-panel.png'; target.unlink(missing_ok=True)
                mon.sendall(f'screendump {target} -f png\n'.encode())
                for _ in range(50):
                    try:
                        with Image.open(target) as shot: shot.verify()
                        break
                    except (OSError,SyntaxError): time.sleep(.1)
                else: raise ValueError('captura de equipo incompleta')
                shutil.copyfile(target,LOG.with_name(LOG.stem+'-equipo.png'))
        if '--energia' in sys.argv:
            transcript += run(conn, [('ultra-entrar /usr/bin/perf --version', 'perf version')], failures)
            probe = "import json,os,stat; from pathlib import Path; p=Path('/run/ultra-energia/estado.json'); d=json.loads(p.read_text()); assert p.stat().st_uid==0 and not p.stat().st_mode&0o022; assert d['solicitado']=='eficiente'; assert d['cpu_paquete_W'] is None; print('ENERGIA-ESTADO-VERIFICADO')"
            transcript += run(conn, [('ultra-entrar /usr/bin/setpriv --reuid=1000 --regid=1000 --clear-groups /usr/bin/python -c '+shlex.quote(probe), 'ENERGIA-ESTADO-VERIFICADO')], failures)
            mon.sendall(b'sendkey meta_l-f8\n')
            transcript += run(conn, [("for i in $(seq 1 50); do for p in $(pidof fuzzel); do if grep -aqF 'Energia CPU /' /proc/$p/cmdline; then echo ENERGIA-MENU-$((70+1)); break 2; fi; done; sleep .1; done", 'ENERGIA-MENU-71')], failures)
            time.sleep(1)
            target=pathlib.Path(tempfile.gettempdir())/'ultra-energia-menu.png'; target.unlink(missing_ok=True)
            mon.sendall(f'screendump {target} -f png\n'.encode())
            for _ in range(50):
                try:
                    with Image.open(target) as shot: shot.verify()
                    break
                except (OSError,SyntaxError): time.sleep(.1)
            else: raise ValueError('captura energia incompleta')
            shutil.copyfile(target, LOG.with_name(LOG.stem+'-energia.png'))
            mon.sendall(b'sendkey esc\n')
            for perfil in ('original', 'fresco', 'eficiente'):
                probe = "import json; from pathlib import Path; assert json.loads(Path('/run/ultra-energia/estado.json').read_text())['solicitado']=="+repr(perfil)+"; print('ENERGIA-PERFIL-VERIFICADO')"
                transcript += run(conn, [('ultra-entrar /usr/bin/setpriv --reuid=1000 --regid=1000 --clear-groups /usr/local/bin/ultra-energia '+perfil+'; sleep 3; ultra-entrar /usr/bin/python -c '+shlex.quote(probe), 'ENERGIA-PERFIL-VERIFICADO')], failures)
        if '--vigilante' in sys.argv:
            encoded = base64.b64encode((ROOT/'scripts/probar_vigilante_instalado.py').read_bytes()).decode()
            transcript += run(conn, [(': > /arch/tmp/vigilante.b64; echo VIG-CARGA-$((40+5))', 'VIG-CARGA-45')], failures)
            for offset in range(0, len(encoded), 192):
                conn.sendall(("printf '%s' '"+encoded[offset:offset+192]+"' >> /arch/tmp/vigilante.b64; echo VIG-CARGA-$((40+5))\n").encode())
                transcript += response('VIG-CARGA-45')
            transcript += run(conn, [('base64 -d /arch/tmp/vigilante.b64 > /arch/tmp/vigilante.py; ultra-entrar /usr/bin/setpriv --reuid=1000 --regid=1000 --clear-groups /usr/bin/python3 /tmp/vigilante.py', 'VIGILANTE-INSTALADO-OK')], failures)
        if '--super-grafo' in sys.argv:
            encoded = base64.b64encode((ROOT/'scripts/probar_super_grafo_instalado.py').read_bytes()).decode()
            transcript += run(conn, [(': > /arch/tmp/sg-banco.b64; echo SG-CARGA-$((40+5))', 'SG-CARGA-45')], failures)
            for offset in range(0, len(encoded), 192):
                conn.sendall(("printf '%s' '"+encoded[offset:offset+192]+"' >> /arch/tmp/sg-banco.b64; echo SG-CARGA-$((40+5))\n").encode())
                transcript += response('SG-CARGA-45')
            transcript += run(conn, [('base64 -d /arch/tmp/sg-banco.b64 > /arch/tmp/sg-banco.py; ultra-entrar /usr/bin/setpriv --reuid=1000 --regid=1000 --clear-groups /usr/bin/python /tmp/sg-banco.py', 'SUPER-GRAFO-INSTALADO-OK')], failures)
        if '--carpetas' in sys.argv or '--super-grafo' in sys.argv:
            transcript += run(conn, [('mkdir -p /arch/home/ultra/Proyectos /arch/home/ultra/Juegos /arch/home/ultra/Museo /arch/home/ultra/Documentos; chown 1000:1000 /arch/home/ultra/Proyectos /arch/home/ultra/Juegos /arch/home/ultra/Museo /arch/home/ultra/Documentos; echo CARPETAS-DATOS-$((50+2))', 'CARPETAS-DATOS-52')], failures)
            if '--super-grafo' in sys.argv:
                command = "printf '#include \"a.h\"\\nint main(void) { return valor(); }\\n' > /arch/home/ultra/Proyectos/main.c; printf '#include \"b.h\"\\nint valor(void);\\n' > /arch/home/ultra/Proyectos/a.h; printf '#define VALOR 42\\n' > /arch/home/ultra/Proyectos/b.h; chown 1000:1000 /arch/home/ultra/Proyectos/*; echo SG-EDIFICIOS-$((50+6))"
                transcript += run(conn, [(command, 'SG-EDIFICIOS-56')], failures)
            conn.sendall(b'ultra-entrar /usr/bin/env XDG_RUNTIME_DIR=/run/user/1000 /usr/bin/hyprctl -i 0 eval \'hl.exec_cmd("ultra-tarjetas parar")\'; echo CARPETAS-INICIO-$((50+3))\n')
            transcript += response('CARPETAS-INICIO-53')
            mon.sendall(b'sendkey meta_l-4\n'); time.sleep(1)
            mon.sendall(b'sendkey meta_l-ctrl-e\n')
            opened = wait_apps(lambda apps: any(a.get('class') == 'ultra-carpetas' or a.get('title') == 'ULTRA Carpetas' for a in apps))
            expect(bool(opened), 'Carpetas: Super+Ctrl+E abre la aplicacion real')
            if opened:
                app = next(a for a in opened if a.get('class') == 'ultra-carpetas' or a.get('title') == 'ULTRA Carpetas')
                # La ventana puede mapearse antes de terminar de crear el adaptador.
                time.sleep(3)
                transcript += run(conn, [(f"grep -Eq '^Uid:[[:space:]]+1000[[:space:]]+1000[[:space:]]+1000[[:space:]]+1000' /proc/{app['pid']}/status && echo CARPETAS-USUARIO-$((50+4))", 'CARPETAS-USUARIO-54'),
                                         (f"grep -q 'libvulkan_lvp' /proc/{app['pid']}/maps && echo CARPETAS-VULKAN-$((50+5))", 'CARPETAS-VULKAN-55')], failures)
                captures = [('carpetas', None), ('carpetas-ajustes', b'sendkey ctrl-comma\n')]
                if '--super-grafo' in sys.argv: captures.append(('carpetas-ciudad', b'sendkey esc\n'))
                for suffix, keys in captures:
                    if keys: mon.sendall(keys); time.sleep(1)
                    if suffix == 'carpetas-ciudad':
                        probe = '''import json,os,subprocess,time
home=os.environ['HOME']
def q(tool,args):
    return json.loads(subprocess.check_output(['/usr/local/bin/ultra-carpetas','--consulta',tool,home,json.dumps(args)],text=True,timeout=45))
r=q('mostrar',{'accion':'grupo','rutas':['Proyectos/main.c','Proyectos/a.h'],'modo':'ciudad'})
assert r['ventana_abierta'],r
for _ in range(30):
    v=q('ventana',{})
    if v.get('vista',{}).get('modo')=='ciudad' and len(v['vista'].get('grupo',[]))==2: break
    time.sleep(.2)
else: raise AssertionError(v)
time.sleep(2)
assert q('ventana',{})['abierta']
print('SUPER-GRAFO-'+'CIUDAD-VIVA-OK')'''
                        command = 'ultra-entrar /usr/bin/setpriv --reuid=1000 --regid=1000 --clear-groups /usr/bin/env HOME=/home/ultra XDG_RUNTIME_DIR=/run/user/1000 /usr/bin/python -c '+shlex.quote(probe)
                        transcript += run(conn, [(command, 'SUPER-GRAFO-CIUDAD-VIVA-OK')], failures)
                    target = pathlib.Path(tempfile.gettempdir())/('ultra-'+suffix+'.png'); target.unlink(missing_ok=True)
                    mon.sendall(f'screendump {target} -f png\n'.encode())
                    for _ in range(50):
                        try:
                            with Image.open(target) as shot: shot.verify()
                            break
                        except (OSError, SyntaxError): time.sleep(.1)
                    else: raise ValueError('captura de Carpetas incompleta')
                    shutil.copyfile(target, LOG.with_name(LOG.stem+'-'+suffix+'.png'))
                mon.sendall(b'sendkey esc\n'); mon.sendall(b'sendkey meta_l-q\n'); time.sleep(1)
                expect(not any(a.get('pid') == app['pid'] for a in clients()), 'Carpetas: cierre normal desde el escritorio')
            mon.sendall(b'sendkey meta_l-1\n')
        if '--heronite' in sys.argv:
            if '--hud-nativo' in sys.argv:
                fixture="""import json
from pathlib import Path
h=Path('/juegos/usuarios/ultra');d=h/'.local/share/heronite'
for name in ('GamesConfig','sideload_apps'): (d/name).mkdir(parents=True,exist_ok=True)
(h/'.config/ultra').mkdir(parents=True,exist_ok=True)
(h/'.config/ultra/hud-desactivado').touch()
(d/'config.json').write_text(json.dumps({'defaultSettings':{'showMangohud':False},'version':'v0'}))
(d/'GamesConfig/ultra_hud_banco.json').write_text(json.dumps({'ultra_hud_banco':{'showMangohud':False},'version':'v0'}))
(d/'sideload_apps/library.json').write_text(json.dumps({'games':[{'runner':'sideload','app_name':'ultra_hud_banco','title':'ULTRA HUD banco Vulkan','is_installed':True,'is_linux_native':True,'install':{'executable':'/usr/bin/vkcube','platform':'linux','install_path':'/tmp'}}]}))
print('HUD-NATIVO-FIXTURE')"""
                user='ultra-entrar /usr/bin/setpriv --reuid=1000 --regid=1000 --clear-groups /usr/bin/env HOME=/juegos/usuarios/ultra '
                transcript += run(conn,[(user+'/usr/bin/python -c '+shlex.quote(fixture),'HUD-NATIVO-FIXTURE')],failures)
            home_env='env HOME=/juegos/usuarios/ultra XDG_DATA_HOME=/juegos/usuarios/ultra/.local/share XDG_CONFIG_HOME=/juegos/usuarios/ultra/.config XDG_CACHE_HOME=/juegos/usuarios/ultra/.cache '
            code = 'hl.exec_cmd("'+home_env+'ultra-heronite biblioteca >/tmp/heronite-ui.log 2>&1")'
            transcript += run(conn, [('ultra-entrar /usr/bin/env XDG_RUNTIME_DIR=/run/user/1000 /usr/bin/hyprctl -i 0 eval '+shlex.quote(code), 'ok')], failures)
            opened = wait_apps(lambda apps: any('heronite' in (str(a.get('class',''))+' '+str(a.get('title',''))).lower() for a in apps))
            expect(bool(opened), 'Heronite: interfaz nativa abre desde el adaptador ULTRA')
            if opened:
                app = next(a for a in opened if 'heronite' in (str(a.get('class',''))+' '+str(a.get('title',''))).lower())
                transcript += run(conn, [('grep -q "heronite-rs,heronite-epic-login" /arch/juegos/usuarios/ultra/.config/ultra/MangoHud.conf && echo HERONITE-HUD-EXCLUIDO', 'HERONITE-HUD-EXCLUIDO')], failures)
                transcript += run(conn, [(f"grep -Eq '^Uid:[[:space:]]+1000[[:space:]]+1000[[:space:]]+1000[[:space:]]+1000' /proc/{app['pid']}/status && grep -q '^NoNewPrivs:[[:space:]]*1' /proc/{app['pid']}/status && echo HERONITE-USUARIO-SEGURO", 'HERONITE-USUARIO-SEGURO')], failures)
                time.sleep(3)
                target = pathlib.Path(tempfile.gettempdir())/'ultra-heronite.png'; target.unlink(missing_ok=True)
                mon.sendall(f'screendump {target} -f png\n'.encode())
                for _ in range(50):
                    try:
                        with Image.open(target) as shot: shot.verify()
                        break
                    except (OSError, SyntaxError): time.sleep(.1)
                else: raise ValueError('captura Heronite incompleta')
                shutil.copyfile(target, LOG.with_name(LOG.stem+'-heronite.png'))
                if '--hud-nativo' in sys.argv:
                    probe="import json;from pathlib import Path;d=Path('/juegos/usuarios/ultra/.local/share/heronite');assert json.loads((d/'config.json').read_text())['defaultSettings']['showMangohud'];assert json.loads((d/'GamesConfig/ultra_hud_banco.json').read_text())['ultra_hud_banco']['showMangohud'];print('HUD-NATIVO-ACTIVADO')"
                    transcript += run(conn,[('ultra-entrar /usr/bin/python -c '+shlex.quote(probe),'HUD-NATIVO-ACTIVADO')],failures)
                    # Forward to the same native instance/data folder used above.
                    code='hl.exec_cmd("'+home_env+'heronite-rs --launch sideload_ultra_hud_banco >/tmp/heronite-hud-game.log 2>&1")'
                    transcript += run(conn,[('ultra-entrar /usr/bin/env XDG_RUNTIME_DIR=/run/user/1000 /usr/bin/hyprctl -i 0 eval '+shlex.quote(code),'ok')],failures)
                    game_apps=wait_apps(lambda apps:any('vkcube' in (str(a.get('class',''))+' '+str(a.get('title',''))).lower() for a in apps))
                    expect(bool(game_apps),'HUD: Heronite lanza la aplicacion Vulkan con su opcion nativa')
                    if game_apps:
                        game_app=next(a for a in game_apps if 'vkcube' in (str(a.get('class',''))+' '+str(a.get('title',''))).lower())
                        transcript += run(conn,[(f"grep -qi libMangoHud /proc/{game_app['pid']}/maps && echo HUD-CAPA-VULKAN-CARGADA",'HUD-CAPA-VULKAN-CARGADA')],failures)
                        time.sleep(3)
                        target=pathlib.Path(tempfile.gettempdir())/'ultra-hud-nativo.png';target.unlink(missing_ok=True)
                        mon.sendall(f'screendump {target} -f png\n'.encode())
                        for _ in range(50):
                            try:
                                with Image.open(target) as shot:shot.verify()
                                break
                            except (OSError,SyntaxError):time.sleep(.1)
                        else:raise ValueError('captura HUD nativo incompleta')
                        shutil.copyfile(target,LOG.with_name(LOG.stem+'-hud-nativo.png'))
                        transcript += run(conn,[(f'ultra-entrar /usr/bin/kill -TERM {game_app["pid"]}; echo HUD-BANCO-CERRADO','HUD-BANCO-CERRADO')],failures)
                transcript += run(conn, [(f'ultra-entrar /usr/bin/kill -TERM {app["pid"]}; echo HERONITE-CIERRE-SOLICITADO', 'HERONITE-CIERRE-SOLICITADO')], failures)
            transcript += run(conn, [('cat /arch/tmp/heronite-ui.log; echo HERONITE-LOG-FIN', 'HERONITE-LOG-FIN')], failures)
        if '--limites' in sys.argv:
            encoded=base64.b64encode((ROOT/'scripts/banco_limites.py').read_bytes()).decode()
            conn.sendall(b': > /arch/tmp/limites.b64; echo LIM-CARGA-$((40+6))\n');transcript+=response('LIM-CARGA-46')
            for offset in range(0,len(encoded),192):
                conn.sendall(("printf '%s' '"+encoded[offset:offset+192]+"' >> /arch/tmp/limites.b64; echo LIM-CARGA-$((40+6))\n").encode());transcript+=response('LIM-CARGA-46')
            conn.sendall(b'busybox base64 -d /arch/tmp/limites.b64 > /arch/tmp/limites.py; echo LIM-CARGA-$((40+6))\n');transcript+=response('LIM-CARGA-46')
            payload='printf "%s\\n" "$$" > /sys/fs/cgroup/ultra-usuario/escritorio/cgroup.procs; exec /usr/bin/setpriv --reuid=1000 --regid=1000 --clear-groups --no-new-privs --bounding-set=-all --inh-caps=-all --ambient-caps=-all /usr/bin/env HOME=/home/ultra /usr/bin/python /tmp/limites.py'
            command='ultra-entrar /usr/bin/nsenter --target $(pidof Hyprland) --mount --root --wd -- /usr/bin/bash -c '+shlex.quote(payload)+'; echo LIM-RC-$?; echo LIM-FIN-$((40+6))\n'
            conn.sendall(command.encode());result=response('LIM-FIN-46',timeout=40);transcript+=result
            for marker,label in [('LIMITES-ANCESTRO-OK','techo de RAM reservado y no ampliable sin root'),('LIMITES-CPU-PIDFD-OK','cuota de CPU medida y cierre por pidfd'),('LIMITES-RAM-OOM-OK','limite de RAM detiene al consumidor de prueba'),('LIM-RC-0','banco de limites finaliza correctamente')]:
                expect(marker in result,'Recursos: '+label)
            mon.sendall(b'sendkey meta_l-esc\n');time.sleep(1)
            found=wait_apps(lambda apps:any(a.get('title')=='ULTRA / Aplicaciones' for a in apps))
            expect(bool(found),'Recursos: Super+Esc abre el administrador')
            if found:
                target=pathlib.Path(tempfile.gettempdir())/'ultra-aplicaciones.png';target.unlink(missing_ok=True)
                mon.sendall(f'screendump {target} -f png\n'.encode())
                for _ in range(50):
                    try:
                        with Image.open(target) as shot:shot.verify()
                        break
                    except (OSError,SyntaxError):time.sleep(.1)
                else:raise ValueError('captura del administrador incompleta')
                shutil.copyfile(target,LOG.with_name(LOG.stem+'-aplicaciones.png'))
        finished_auto = False
        for _ in range(65):
            conn.sendall(b'grep -q \'"running": false\' /arch/juegos/usuarios/ultra/.cache/ultra/discos/automatico.json && echo DISCOS-FIN-$((9+4)); echo DISCOS-QUERY-$((9+5))\n')
            out = response('DISCOS-QUERY-14')
            transcript += out
            if 'DISCOS-FIN-13' in out:
                finished_auto = True; break
            time.sleep(1)
        expect(finished_auto, 'Discos: revision automatica de 60 s termina sin bloquear el escritorio')
        mon.sendall(b'sendkey meta_l-shift-m\n')
        time.sleep(4)
        conn.sendall(b'echo HYPR-SINC-$((3+4))\n')
        transcript += response('HYPR-SINC-7')
        transcript += run(conn, [('echo HYPR-$(grep -c "hyprland termino (salida 0)" /tmp/hyprland-session.log)', 'HYPR-1'),
                                ('test ! -S /arch/run/seatd.sock && echo HYPR-SEATD-LIMPIO', 'HYPR-SEATD-LIMPIO')], failures)
        if '--energia' in sys.argv:
            probe = "import json; from pathlib import Path; d=json.loads(Path('/run/ultra-energia/estado.json').read_text()); assert d['perfil']=='sesion_cerrada'; assert d['errores']==[]; print('ENERGIA-CIERRE-LIMPIO')"
            transcript += run(conn, [('ultra-entrar /usr/bin/python -c '+shlex.quote(probe), 'ENERGIA-CIERRE-LIMPIO')], failures)
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
        proc = subprocess.Popen([str(QEMU), '-machine', 'pc', '-accel', 'whpx', '-cpu', 'host', '-smp', '1', '-m', str(RAM_MIB),
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
            # The author's Tux intro, animated in the centre and the screen put back.
            expect_boot(first, 'VERDE [INTRO]: 419x421, 23 cuadros', 'arranque 1: intro de arranque', failures)
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
                if '--puntero' in sys.argv:
                    transcript += run(conn, [('grep -Eq \'"reparaciones_transparentes": [1-9][0-9]*\' /bmo/ULTRA/informe/gpu-$(uname -r)-software-hyprland.txt && echo PUNTERO-EXPORTADO-SIN-ROOT', 'PUNTERO-EXPORTADO-SIN-ROOT')], failures)
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
