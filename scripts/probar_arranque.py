# SPDX-License-Identifier: GPL-2.0-only
"""Boot the downloaded kernel headless, drive its shell over serial and check the answers."""
import hashlib, pathlib, shutil, socket, subprocess, sys, time

ROOT = pathlib.Path(__file__).resolve().parent.parent
PERFIL = sys.argv[1] if len(sys.argv) > 1 else 'produccion'
# --disco boots ultra-esp.img through UEFI firmware, with no -kernel and no boot loader.
DISCO = '--disco' in sys.argv[2:]
QEMU_SHARE = pathlib.Path(r'C:\Program Files\qemu\share')
BUILD = ROOT / 'build' / {'produccion': 'baseline', 'vigia': 'vigia', 'generico': 'generico'}[PERFIL]
KERNEL = BUILD / 'bzImage-tiny64'
IMG = BUILD / 'ultra-esp.img'
LOG = BUILD / ('prueba-disco.log' if DISCO else 'prueba-shell.log')
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
    ('probar-caja', 'RESUMEN CAJA: VERDE=11 ROJO=0'),
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
if PERFIL == 'vigia':
    CHECKS += [
        ('dmesg | grep -m1 "AddressSanitizer init"', 'initialized'),
        ('dmesg | grep -m1 "Lock dependency validator"', 'Ingo Molnar'),
        (f'dmesg | grep -E "{REPORTS}" || echo VIGIA-$((1))-LIMPIO', 'VIGIA-1-LIMPIO'),
    ]

def read_until(conn, marker, timeout):
    data, end = b'', time.time() + timeout
    while marker.encode() not in data:
        if time.time() > end: raise TimeoutError(f'no aparecio {marker!r}')
        try:
            chunk = conn.recv(4096)
        except socket.timeout:
            continue
        if not chunk: raise ConnectionError('la consola serie se cerro')
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

def main():
    check_kernel()
    if DISCO:
        vars_copy = BUILD / 'ovmf-vars-prueba.fd'
        shutil.copyfile(QEMU_SHARE / 'edk2-i386-vars.fd', vars_copy)
        boot = ['-drive', f'if=pflash,format=raw,readonly=on,file={QEMU_SHARE / "edk2-x86_64-code.fd"}',
                '-drive', f'if=pflash,format=raw,file={vars_copy}',
                '-drive', f'file={IMG},format=raw,if=virtio,snapshot=on']
    else:
        boot = ['-kernel', str(KERNEL), '-append', 'console=ttyS0,115200 rdinit=/init panic=-1 noapic quiet '
                'GLIBC_TUNABLES=glibc.cpu.hwcaps=SHSTK:glibc.cpu.x86_shstk=on']
    qemu = subprocess.Popen([str(QEMU), '-machine', 'pc', '-accel', 'whpx', '-cpu', 'host', '-smp', '1', '-m', '1024',
                             *boot, '-nic', 'none', '-display', 'none', '-no-reboot',
                             '-serial', f'tcp:127.0.0.1:{PORT},server=on,wait=on'],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    transcript, failures = '', []
    try:
        for _ in range(50):
            try:
                conn = socket.create_connection(('127.0.0.1', PORT), timeout=1); break
            except OSError: time.sleep(0.2)
        else: raise ConnectionError('QEMU no abrio la consola serie')
        conn.settimeout(0.5)
        # Firmware start-up adds seconds before the kernel runs.
        transcript += read_until(conn, 'ultra:', 120 if DISCO else 30)
        for command, expected in CHECKS:
            conn.sendall(command.encode() + b'\n')
            out = read_until(conn, 'ultra:', 30)
            transcript += out
            print(('VERDE' if expected in out else 'ROJO'), f'[{command}]')
            if expected not in out: failures.append(command)
        conn.sendall(b'poweroff\n')
        transcript += read_until(conn, 'APAGADO', 10)
        qemu.wait(timeout=15)
        print('VERDE [poweroff]: init apago la maquina y QEMU termino')
    except Exception as exc:
        failures.append(str(exc)); print('ROJO', exc)
    finally:
        if qemu.poll() is None: qemu.kill()
        LOG.write_text(transcript, encoding='utf-8')
    sys.exit(1 if failures else 0)

if __name__ == '__main__':
    main()
