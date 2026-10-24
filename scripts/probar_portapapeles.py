#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Prueba ultra-portapapeles con cliphist, fuzzel, wl-copy y wl-paste falsos."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile

RAIZ = Path(__file__).resolve().parents[1]


def main():
    with tempfile.TemporaryDirectory(prefix='ultra-portapapeles-') as tmp:
        tmp = Path(tmp); bin_ = tmp / 'bin'; bin_.mkdir(); reg = tmp / 'llamadas'
        runtime = tmp / 'run'; runtime.mkdir(mode=0o700)
        falsos = {
            # Dos entradas en la lista; decode devuelve el texto de la 1.
            'cliphist': 'printf "cliphist %s\\n" "$*" >> REG; case " $* " in *" list "*) printf "1\\thola mundo\\n2\\tclave\\n";; '
                        '*" decode "*) read -r l; [[ $l == 1* ]] && printf "hola mundo";; esac',
            'fuzzel': 'cat > TMP/entrada; printf "fuzzel %s\\n" "$*" >> REG; cat TMP/eleccion',
            'wl-copy': 'printf "wl-copy %s\\n" "$(cat)" >> REG',
            'wl-paste': 'printf "wl-paste %s\\n" "$*" >> REG',
            'notify-send': 'printf "aviso %s\\n" "$*" >> REG',
            'ultra-personalizar': 'echo 3dff9aff',
        }
        for nombre, cuerpo in falsos.items():
            (bin_ / nombre).write_text('#!/bin/bash\n' + cuerpo.replace('REG', str(reg)).replace('TMP', str(tmp)) + '\n')
            (bin_ / nombre).chmod(0o755)
        herramienta = tmp / 'ultra-portapapeles'
        herramienta.write_text((RAIZ / 'src/rootfs/bin/ultra-portapapeles').read_text().replace('if os.geteuid() != 1000:', 'if False:'))
        entorno = dict(os.environ, PATH=f'{bin_}:{os.environ["PATH"]}', XDG_RUNTIME_DIR=str(runtime))
        db = runtime / 'ultra-portapapeles.db'

        def correr(*args, eleccion='', ok=True, env=entorno):
            reg.write_text(''); (tmp / 'eleccion').write_bytes(eleccion.encode())
            r = subprocess.run([sys.executable, '-I', str(herramienta), *args], env=env, capture_output=True, text=True, timeout=30)
            assert (r.returncode == 0) == ok, (args, r.returncode, r.stderr)
            return reg.read_text()

        # Vigilar: wl-paste --watch guarda en RAM (XDG_RUNTIME_DIR) con limite.
        llamadas = correr('vigilar')
        assert f'wl-paste --watch cliphist -db-path {db} -max-items 200 store' in llamadas
        # Elegir la entrada 1: se decodifica y vuelve al portapapeles; el menu usa el acento.
        llamadas = correr('menu', eleccion='1\thola mundo\n')
        assert f'cliphist -db-path {db} -max-items 200 decode' in llamadas and 'wl-copy hola mundo' in llamadas
        assert '--selection-color=3dff9aff' in llamadas
        assert (tmp / 'entrada').read_text().endswith('Borrar todo el historial\n')
        # Borrar desde el menu.
        llamadas = correr('menu', eleccion='Borrar todo el historial\n')
        assert 'wipe' in llamadas and 'aviso' in llamadas and 'wl-copy' not in llamadas
        # Esc o una linea inventada no copian nada.
        assert 'decode' not in correr('menu')
        llamadas = correr('menu', eleccion='9\tinventada\n')
        assert 'decode' not in llamadas and 'wl-copy' not in llamadas
        # Sin XDG_RUNTIME_DIR no se guarda en disco.
        sin = {k: v for k, v in entorno.items() if k != 'XDG_RUNTIME_DIR'}
        correr('vigilar', ok=False, env=sin)
        correr('nada', ok=False)
    print('VERDE: portapapeles en RAM, elegir, borrar y rechazar entradas inventadas')


if __name__ == '__main__':
    main()
