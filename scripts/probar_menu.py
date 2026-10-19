#!/usr/bin/env python3
"""Exercise the real Bash menu in a Linux PTY, replacing only its backends."""
import os
from pathlib import Path
import pty
import select
import subprocess
import sys
import tempfile
import time


def main():
    source = Path(sys.argv[1]).read_text()
    with tempfile.TemporaryDirectory(prefix='ultra-menu-') as tmp:
        root = Path(tmp)
        backend = root / 'session'
        backend.write_text('#!/bin/bash\nprintf "BACKEND:%s:%s\\n" "$1" "$2"\n'
                           '[[ $1 != nvidia ]] || exit 13\n')
        backend.chmod(0o755)
        (root / 'ultima.log').write_text('GL renderer: TEST-RENDERER\n')
        smi = root / 'nvidia-smi'
        smi.write_text('#!/bin/bash\necho TEST-NVIDIA-STATUS\n')
        smi.chmod(0o755)
        menu = root / 'menu'
        menu.write_text(source.replace('/usr/local/libexec/ultra-sesion', str(backend))
                       .replace('/run/ultra-escritorio', tmp)
                       .replace('/usr/bin/nvidia-smi', str(smi)))
        subprocess.run(['bash', '-n', str(menu)], check=True)
        direct = subprocess.run(['bash', str(menu)], capture_output=True)
        assert direct.returncode == 2 and b'consola' in direct.stdout

        def exercise(keys, expected):
            master, slave = pty.openpty()
            process = subprocess.Popen(['bash', str(menu)], stdin=slave, stdout=slave,
                                       stderr=slave, start_new_session=True)
            os.close(slave)
            data = bytearray()

            def until(marker):
                end = time.monotonic() + 5
                start = len(data)
                while marker not in data[start:]:
                    if time.monotonic() > end:
                        raise AssertionError(f'Missing {marker!r}: {data!r}')
                    if select.select([master], [], [], 0.1)[0]:
                        data.extend(os.read(master, 65536))

            try:
                until(b'Al cerrar')
                for key, marker in keys:
                    os.write(master, key)
                    until(marker)
                os.write(master, b'q')
                assert process.wait(timeout=5) == 0
                for marker in expected:
                    assert marker in data, (marker, data)
            finally:
                if process.poll() is None:
                    process.kill()
                    process.wait()
                os.close(master)

        exercise([(b'\r', b'codigo 0'), (b'x', b'Al cerrar')], [b'BACKEND:auto:foot'])
        exercise([(b'\x1b[B', b'Al cerrar'), (b'\r', b'codigo 13'),
                  (b'x', b'Al cerrar'), (b'\x1b[B', b'Al cerrar'),
                  (b'\r', b'codigo 0'), (b'x', b'Al cerrar'),
                  (b'4', b'Al cerrar'), (b'\r', b'TEST-RENDERER'),
                  (b'x', b'Al cerrar'), (b'5', b'Al cerrar'),
                  (b'\r', b'TEST-NVIDIA-STATUS'), (b'x', b'Al cerrar')],
                 [b'BACKEND:nvidia:foot', b'BACKEND:software:foot'])
        # Up wraps to the exit item; Enter exits without invoking a backend.
        for key in (b'\x1b[A\r', b'\x1b', b'6\r'):
            master, slave = pty.openpty()
            process = subprocess.Popen(['bash', str(menu)], stdin=slave, stdout=slave,
                                       stderr=slave)
            os.close(slave)
            try:
                assert select.select([master], [], [], 5)[0]
                os.read(master, 65536)
                os.write(master, key)
                assert process.wait(timeout=5) == 0
            finally:
                if process.poll() is None:
                    process.kill()
                    process.wait()
                os.close(master)
        print('VERDE: PTY real; flechas, Enter, numeros, Esc, Q, modos, fallo y diagnosticos.')


if __name__ == '__main__':
    main()
