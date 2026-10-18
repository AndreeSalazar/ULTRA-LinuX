# SPDX-License-Identifier: GPL-2.0-only
"""Boot the downloaded kernel headless, drive its shell over serial and check the answers."""
import pathlib, socket, subprocess, sys, time

ROOT = pathlib.Path(__file__).resolve().parent.parent
KERNEL = ROOT / 'build/baseline/bzImage-tiny64'
LOG = ROOT / 'build/baseline/prueba-shell.log'
QEMU = pathlib.Path(r'C:\Program Files\qemu\qemu-system-x86_64.exe')
PORT = 15555
# Each command and a fragment its output must contain.
CHECKS = [
    ('cat /etc/os-release', 'NAME="ULTRA LinuX"'),
    ('uname -sm', 'Linux x86_64'),
    ('ps', '/init'),
    ('echo $((6*7))', '42'),
    ('ls /proc/self/fd', '0'),
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

def main():
    qemu = subprocess.Popen([str(QEMU), '-machine', 'pc', '-accel', 'whpx', '-cpu', 'host', '-smp', '1', '-m', '512',
                             '-kernel', str(KERNEL), '-append', 'console=ttyS0,115200 rdinit=/init panic=-1 noapic',
                             '-nic', 'none', '-display', 'none', '-no-reboot',
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
        transcript += read_until(conn, 'ultra:', 30)
        for command, expected in CHECKS:
            conn.sendall(command.encode() + b'\n')
            out = read_until(conn, 'ultra:', 10)
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
