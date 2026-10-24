#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Prueba ultra-captura con grim, slurp, wl-copy y hyprctl falsos."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile

RAIZ = Path(__file__).resolve().parents[1]


def main():
    with tempfile.TemporaryDirectory(prefix='ultra-captura-') as tmp:
        tmp = Path(tmp); home = tmp / 'home'; home.mkdir(); bin_ = tmp / 'bin'; bin_.mkdir(); reg = tmp / 'llamadas'
        falsos = {
            'grim': 'printf "grim %s\\n" "$*" >> REG; for a; do last=$a; done; printf "\\x89PNG" > "$last"',
            'slurp': 'printf "slurp %s\\n" "$*" >> REG; [[ -f TMP/cancelar ]] && exit 1; cat TMP/geometria',
            'wl-copy': 'printf "wl-copy %s %s\\n" "$*" "$(head -c4 | od -An -tx1 | tr -d " ")" >> REG',
            'hyprctl': 'if [[ $2 == activewindow ]]; then echo \'{"at":[10,20],"size":[300,200]}\'; else echo \'[{"name":"DP-1","focused":true}]\'; fi',
            'notify-send': 'printf "aviso %s\\n" "$*" >> REG',
            'ultra-personalizar': 'printf "personalizar %s\\n" "$*" >> REG; [[ $1 != acento ]] || echo ff2a4fff',
        }
        for nombre, cuerpo in falsos.items():
            (bin_ / nombre).write_text('#!/bin/bash\n' + cuerpo.replace('REG', str(reg)).replace('TMP', str(tmp)) + '\n')
            (bin_ / nombre).chmod(0o755)
        herramienta = tmp / 'ultra-captura'
        herramienta.write_text((RAIZ / 'src/rootfs/bin/ultra-captura').read_text().replace('if os.geteuid() != 1000:', 'if False:'))
        entorno = dict(os.environ, HOME=str(home), PATH=f'{bin_}:{os.environ["PATH"]}')

        def correr(modo, ok=True):
            reg.write_text('')
            r = subprocess.run([sys.executable, '-I', str(herramienta), modo], env=entorno, capture_output=True, text=True, timeout=30)
            assert (r.returncode == 0) == ok, (modo, r.returncode, r.stderr)
            return r.stdout.strip(), reg.read_text()

        (tmp / 'geometria').write_text('100,200 640x480\n')
        salida, llamadas = correr('region')
        archivo = Path(salida)
        assert archivo.parent == home / 'Imágenes/Capturas' and archivo.read_bytes() == b'\x89PNG'
        assert '-c ff2a4fff' in llamadas and '-g 100,200 640x480' in llamadas and 'wl-copy --type image/png 89504e47' in llamadas
        assert oct(archivo.stat().st_mode & 0o777) == '0o600'
        assert 'personalizar sonar captura' in llamadas
        _, llamadas = correr('ventana'); assert '-g 10,20 300x200' in llamadas
        _, llamadas = correr('pantalla'); assert 'grim -o DP-1' in llamadas
        (tmp / 'cancelar').touch()
        salida, llamadas = correr('region'); assert salida == '' and 'grim' not in llamadas and 'aviso' not in llamadas
        (tmp / 'cancelar').unlink(); (tmp / 'geometria').write_text('100,200 640x480; rm -rf ~\n')
        correr('region', ok=False)
        correr('nada', ok=False)
    print('VERDE: captura region, ventana y pantalla; Esc cancela; geometria rara rechazada')


if __name__ == '__main__':
    main()
