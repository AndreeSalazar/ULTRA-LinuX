#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Prueba ultra-captura con grim, slurp, wl-copy y hyprctl falsos."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import base64
import io
import json
from PIL import Image

RAIZ = Path(__file__).resolve().parents[1]


def main():
    with tempfile.TemporaryDirectory(prefix='ultra-captura-') as tmp:
        tmp = Path(tmp); home = tmp / 'home'; home.mkdir(); bin_ = tmp / 'bin'; bin_.mkdir(); reg = tmp / 'llamadas'
        png=io.BytesIO();Image.new('RGB',(2,2),'cyan').save(png,format='PNG')
        (tmp/'imagen.b64').write_bytes(base64.b64encode(png.getvalue()))
        falsos = {
            'grim': 'printf "grim %s\\n" "$*" >> REG; [[ ! -f TMP/grim-falla ]] || { echo "denegado por compositor" >&2; exit 1; }; for a; do last=$a; done; base64 -d TMP/imagen.b64 > "$last"',
            'slurp': 'printf "slurp %s\\n" "$*" >> REG; [[ ! -f TMP/slurp-falla ]] || { echo "failed to connect to display" >&2; exit 1; }; [[ ! -f TMP/cancelar ]] || { echo "selection cancelled" >&2; exit 1; }; cat TMP/geometria',
            'wl-copy': '[[ ! -f TMP/clipboard-falla ]] || exit 1; printf "wl-copy %s %s\\n" "$*" "$(head -c4 | od -An -tx1 | tr -d " ")" >> REG',
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
        assert archivo.parent == home / 'Imágenes/Capturas' and archivo.read_bytes() == png.getvalue()
        assert '-c ff2a4fff' in llamadas and '-g 100,200 640x480' in llamadas and 'wl-copy --type image/png 89504e47' in llamadas
        assert oct(archivo.stat().st_mode & 0o777) == '0o600'
        assert 'personalizar sonar captura' in llamadas
        _, llamadas = correr('ventana'); assert '-g 10,20 300x200' in llamadas
        otra, llamadas = correr('pantalla'); assert 'grim -o DP-1' in llamadas
        assert Path(otra)!=archivo and archivo.read_bytes()==png.getvalue()
        (tmp/'clipboard-falla').touch()
        guardada, llamadas = correr('pantalla')
        assert Path(guardada).is_file() and 'Captura guardada' in llamadas
        estado=json.loads((home/'.cache/ultra/captura-estado.json').read_text())
        assert estado['resultado']=='guardada' and estado['portapapeles'] is False
        (tmp/'clipboard-falla').unlink()
        (tmp/'grim-falla').touch(); existentes=set(archivo.parent.glob('*.png'))
        _, llamadas=correr('pantalla',ok=False)
        assert set(archivo.parent.glob('*.png'))==existentes and 'denegado por compositor' in llamadas
        (tmp/'grim-falla').unlink()
        (tmp / 'cancelar').touch()
        salida, llamadas = correr('region'); assert salida == '' and 'grim' not in llamadas and 'aviso' not in llamadas
        (tmp / 'cancelar').unlink(); (tmp / 'geometria').write_text('100,200 640x480; rm -rf ~\n')
        correr('region', ok=False)
        (tmp/'slurp-falla').touch();_,llamadas=correr('region',ok=False)
        assert 'failed to connect to display' in llamadas
        (tmp/'slurp-falla').unlink()
        correr('nada', ok=False)
        carpeta=home/'Imágenes/Capturas'; carpeta.rename(tmp/'capturas-reubicadas');carpeta.symlink_to(tmp/'capturas-reubicadas',target_is_directory=True)
        reubicada,_=correr('pantalla')
        assert Path(reubicada).parent==tmp/'capturas-reubicadas'
    print('VERDE: captura region, ventana y pantalla; Esc cancela; geometria rara rechazada')


if __name__ == '__main__':
    main()
