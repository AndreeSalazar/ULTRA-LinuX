#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Prueba el clic en CPU/RAM de la barra (ultra-aplicaciones rapido) con fuzzel falso.

Necesita root: lanza un proceso como UID 1000 y ejecuta la herramienta como ese usuario,
igual que en el escritorio. Comprueba elegir, confirmar, forzar cierre y que los
procesos protegidos no aparezcan.
"""
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

RAIZ = Path(__file__).resolve().parents[1]


def main():
    if os.geteuid() != 0:
        print('AMARILLO: probar_controles_barra necesita root para correr como UID 1000'); return
    tmp = Path(tempfile.mkdtemp(prefix='ultra-controles-')); tmp.chmod(0o755)
    try:
        bin_ = tmp / 'bin'; bin_.mkdir(); trabajo = tmp / 'trabajo'; trabajo.mkdir(); os.chown(trabajo, 1000, 1000)
        cola = trabajo / 'cola'; reg = trabajo / 'llamadas'
        # fuzzel falso: anota lo que le ofrecen y elige la linea que contiene el patron de la cola.
        (bin_ / 'fuzzel').write_text(f'''#!/bin/bash
entrada=$(cat); printf '%s\\n---\\n' "$entrada" >> {reg}
patron=$(head -n1 {cola}); sed -i 1d {cola}
[[ -n $patron ]] || exit 1
printf '%s\\n' "$entrada" | grep -F -m1 -- "$patron"
''')
        (bin_ / 'notify-send').write_text(f'#!/bin/bash\nprintf "aviso %s\\n" "$*" >> {reg}\n')
        (bin_ / 'ultra-personalizar').write_text('#!/bin/bash\necho ff2a4fff\n')
        for f in bin_.iterdir(): f.chmod(0o755)
        cola.write_text(''); reg.write_text(''); os.chown(cola, 1000, 1000); os.chown(reg, 1000, 1000)
        herramienta = tmp / 'ultra-aplicaciones'; shutil.copy(RAIZ / 'src/rootfs/bin/ultra-aplicaciones', herramienta); herramienta.chmod(0o755)
        entorno = dict(PATH=f'{bin_}:/usr/bin:/bin', HOME=str(tmp))
        # Un proceso "que se come la CPU" y un waybar (protegido) del mismo usuario.
        glotón = subprocess.Popen(['/bin/sh', '-c', 'while :; do :; done'], user=1000, group=1000)
        shutil.copy(shutil.which('sleep'), bin_ / 'waybar')
        barra = subprocess.Popen([str(bin_ / 'waybar'), '30'], user=1000, group=1000)
        time.sleep(1)

        def correr(*args, elecciones):
            cola.write_text('\n'.join(elecciones) + '\n'); reg.write_text('')
            r = subprocess.run([sys.executable, '-I', str(herramienta), *args], user=1000, group=1000, env=entorno,
                               capture_output=True, text=True, timeout=30)
            assert r.returncode == 0, r.stderr
            return reg.read_text()

        # Cancelar en la confirmacion: el proceso sigue vivo.
        llamadas = correr('rapido', 'cpu', elecciones=[f'[{glotón.pid}]', 'Forzar cierre', 'No, volver'])
        primera = llamadas.split('---')[0].strip().splitlines()
        assert f'[{glotón.pid}]' in primera[0], primera[:3]  # el que mas CPU usa sale primero
        assert f'[{barra.pid}]' not in llamadas.split('---')[0], 'waybar esta protegido'
        assert 'Abrir el administrador completo' in llamadas
        assert glotón.poll() is None
        # Confirmar: se fuerza el cierre y avisa.
        llamadas = correr('rapido', 'cpu', elecciones=[f'[{glotón.pid}]', 'Forzar cierre', 'Si, forzar'])
        glotón.wait(timeout=5)
        assert glotón.returncode == -9 and 'aviso' in llamadas, (glotón.returncode, llamadas)
        # Por RAM, Esc no hace nada.
        correr('rapido', 'ram', elecciones=[])
        r = subprocess.run([sys.executable, '-I', str(herramienta), 'rapido', 'disco'], user=1000, group=1000, env=entorno, capture_output=True, text=True)
        assert r.returncode != 0
        barra.kill()
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    print('VERDE: clic en CPU/RAM lista lo que mas consume, protege el escritorio, confirma y cierra')


if __name__ == '__main__':
    main()
