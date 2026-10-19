# SPDX-License-Identifier: GPL-2.0-only
"""Boot generico and pc several times from the A:\\ test disk and compare their times.

Each profile boots a fresh copy of its disco-bmo.img: the first boot detects the
hardware, the rest are guided. Numbers come from /datos/arranques.log, written by
the init itself: uptime_ms (kernel start up to the end of the hardware stage) and
the stage times. Medians, because WHPX adds noise.
"""
import pathlib, re, shutil, socket, statistics, subprocess, sys, time

ROOT = pathlib.Path(__file__).resolve().parent.parent
QEMU_SHARE = pathlib.Path(r'C:\Program Files\qemu\share')
QEMU = pathlib.Path(r'C:\Program Files\qemu\qemu-system-x86_64.exe')
PORT = 15565
BOOTS = int(sys.argv[1]) if len(sys.argv) > 1 else 6
PROFILES = sys.argv[2:] or ['generico', 'pc']

def read_until(conn, marker, timeout):
    data, end = b'', time.time() + timeout
    while marker.encode() not in data:
        if time.time() > end: raise TimeoutError(f'no aparecio {marker!r}')
        try: data += conn.recv(4096)
        except socket.timeout: pass
    return data.decode(errors='replace')

def boot_once(disk, vars_copy):
    qemu = subprocess.Popen([str(QEMU), '-machine', 'pc', '-accel', 'whpx', '-cpu', 'host', '-smp', '1', '-m', '1024',
                             '-drive', f'if=pflash,format=raw,readonly=on,file={QEMU_SHARE / "edk2-x86_64-code.fd"}',
                             '-drive', f'if=pflash,format=raw,file={vars_copy}',
                             '-drive', f'file={disk},format=raw,if=virtio', '-device', 'intel-hda',
                             '-nic', 'none', '-display', 'none', '-no-reboot',
                             '-serial', f'tcp:127.0.0.1:{PORT},server=on,wait=on'],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(50):
            try: conn = socket.create_connection(('127.0.0.1', PORT), timeout=1); break
            except OSError: time.sleep(0.2)
        else: raise ConnectionError('QEMU no abrio la consola serie')
        conn.settimeout(0.5)
        read_until(conn, 'ultra:', 120)
        conn.sendall(b'tail -n 1 /datos/arranques.log; echo FIN-$((1+1))\n')
        line = read_until(conn, 'FIN-2', 20)
        conn.sendall(b'poweroff\n')
        read_until(conn, 'APAGADO', 10)
        qemu.wait(timeout=20)
        return dict(re.findall(r'(\w+)=(-?\d+)', line.split('arranque=', 1)[1].splitlines()[0])) | \
            {'modo': re.search(r'arranque=(\w+)', line).group(1)}
    finally:
        if qemu.poll() is None: qemu.kill()

def main():
    results = {}
    for profile in PROFILES:
        build = ROOT / 'build' / profile
        disk, vars_copy = build / 'medida-bmo.img', build / 'ovmf-vars-medida.fd'
        shutil.copyfile(build / 'disco-bmo.img', disk)
        shutil.copyfile(QEMU_SHARE / 'edk2-i386-vars.fd', vars_copy)
        runs = []
        for n in range(BOOTS):
            r = boot_once(disk, vars_copy)
            runs.append(r)
            print(f'{profile} arranque {n + 1}: {r["modo"]} uptime_ms={r.get("uptime_ms")} total_us={r.get("total_us")}')
        disk.unlink()
        results[profile] = runs
    print('\nMEDIANAS (arranques guiados; el primero es la deteccion completa)')
    print(f'{"perfil":10} {"uptime_ms":>10} {"etapa_us":>10} {"carga_us":>10}  deteccion_us (1.er arranque)')
    for profile, runs in results.items():
        guided = [r for r in runs if r['modo'] == 'guiado']
        first = runs[0]
        med = lambda key: statistics.median(int(r[key]) for r in guided) if guided else float('nan')
        print(f'{profile:10} {med("uptime_ms"):>10.0f} {med("total_us"):>10.0f} {med("carga_us"):>10.0f}  {first.get("deteccion_us")}')

if __name__ == '__main__':
    main()
