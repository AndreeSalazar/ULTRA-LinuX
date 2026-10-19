# SPDX-License-Identifier: GPL-2.0-only
"""Boot the downloaded kernel headless, drive its shell over serial and check the answers."""
import hashlib, pathlib, secrets, shutil, socket, subprocess, sys, tempfile, time

ROOT = pathlib.Path(__file__).resolve().parent.parent
PERFIL = sys.argv[1] if len(sys.argv) > 1 else 'produccion'
# --disco boots ultra-esp.img through UEFI firmware, with no -kernel and no boot loader.
# --bmo boots disco-bmo.img: ULTRA as files on a FAT32 partition labelled BMO, beside a
# simulated Windows EFI partition, the way it lives on A:\ of the PC.
BMO = '--bmo' in sys.argv[2:]
DISCO = BMO or '--disco' in sys.argv[2:]
QEMU_SHARE = pathlib.Path(r'C:\Program Files\qemu\share')
BUILD = ROOT / 'build' / {'produccion': 'baseline', 'vigia': 'vigia', 'generico': 'generico'}[PERFIL]
KERNEL = BUILD / 'bzImage-tiny64'
IMG = BUILD / ('disco-bmo.img' if BMO else 'ultra-esp.img')
# The disk test writes to /datos, so it boots a fresh copy and never the verified image.
IMG_PRUEBA = BUILD / (IMG.stem + '-prueba.img')
LOG = BUILD / ('prueba-bmo.log' if BMO else 'prueba-disco.log' if DISCO else 'prueba-shell.log')
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
if PERFIL == 'generico':
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
        ('ls /bmo/ULTRA', 'PC-configuracion.txt'),
    ]
    # The copy Windows reads on A:\ULTRA is updated on every boot.
    BOOT2 += [('cat /bmo/ULTRA/arranques.txt', 'arranque=guiado')]
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
            transcript += run(conn, BOOT1, failures)
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
