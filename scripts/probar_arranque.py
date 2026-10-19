# SPDX-License-Identifier: GPL-2.0-only
"""Boot the downloaded kernel headless, drive its shell over serial and check the answers."""
import hashlib, pathlib, re, secrets, shutil, socket, subprocess, sys, tempfile, time

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
    # QEMU has no NVIDIA: explicit acceleration must reject, never pretend software is GPU.
    BOOT1 += [('ultra-escritorio --nvidia 2>&1', 'ROJO [GPU]: se requiere exactamente un dispositivo DRM NVIDIA')]
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
        if hashlib.sha256(f.read_bytes()).hexdigest() != expected[f.name]:
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

def check_window(conn, failures):
    """W2: the first Wayland window, seen on the emulated screen and closed with its keyboard.

    ultra-escritorio starts udevd, seatd and cage with foot. foot paints the whole screen
    with its background (#242424); then "exit" typed on the PS/2 keyboard must reach foot
    through udev, libinput and Wayland, and cage must end cleanly.
    """
    t = ''
    conn.sendall(b'ultra-escritorio > /tmp/escritorio.log 2>&1 &\n')
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
                       if all(abs(pixels[i + k] - 0x24) <= 6 for k in range(3)))
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
        proc = subprocess.Popen([str(QEMU), '-machine', 'pc', '-accel', 'whpx', '-cpu', 'host', '-smp', '1', '-m', '1024',
                                 *boot, '-nic', 'none', '-display', 'none', '-no-reboot',
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
            # Wayland stage W2: the first window, seen and closed through the emulated hardware.
            transcript += check_window(conn, failures)
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
