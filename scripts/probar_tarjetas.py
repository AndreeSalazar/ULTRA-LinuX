#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Prueba ultra-tarjetas: lecturas con /proc y /sys falsos, JSON que coincide con eww.yuck,
y abrir, ocultar o caer a la version de texto con eww y foot falsos."""
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile

RAIZ = Path(__file__).resolve().parents[1]
FUENTE = RAIZ / 'src/rootfs/bin/ultra-tarjetas'


def cargar(entorno):
    viejo = dict(os.environ); os.environ.update(entorno)
    try:
        cargador = importlib.machinery.SourceFileLoader('tarjetas', str(FUENTE))
        modulo = importlib.util.module_from_spec(importlib.util.spec_from_loader('tarjetas', cargador))
        cargador.exec_module(modulo); return modulo
    finally:
        os.environ.clear(); os.environ.update(viejo)


def main():
    with tempfile.TemporaryDirectory(prefix='ultra-tarjetas-') as tmp:
        tmp = Path(tmp); proc = tmp / 'proc'; sys_ = tmp / 'sys'
        (proc / 'net').mkdir(parents=True)
        (proc / 'meminfo').write_text('MemTotal: 16000000 kB\nMemAvailable: 4000000 kB\nSwapTotal: 2000000 kB\nSwapFree: 1500000 kB\n')
        (proc / 'net/dev').write_text('Inter-|\n face |\n    lo: 999999 0 0 0 0 0 0 0 999999 0\n  eth0: 1000 0 0 0 0 0 0 0 500 0\n'
                                      'veth1: 77777 0 0 0 0 0 0 0 77777 0\n')
        hw = sys_ / 'class/hwmon/hwmon0'; hw.mkdir(parents=True)
        (hw / 'temp1_input').write_text('54000\n'); (hw / 'temp2_input').write_text('71500\n'); (hw / 'temp3_input').write_text('200000\n')
        drm = sys_ / 'class/drm/card0/device'; drm.mkdir(parents=True)
        (drm / 'gpu_busy_percent').write_text('63\n'); (drm / 'mem_info_vram_used').write_text(str(3 * 2**30))
        (drm / 'mem_info_vram_total').write_text(str(8 * 2**30))
        t = cargar(dict(ULTRA_PROC=str(proc), ULTRA_SYS=str(sys_)))
        # Lecturas.
        assert t.uso({'cpu': (100, 1000)}, {'cpu': (400, 2000)}) == {'cpu': 30.0}
        m = t.memoria(); assert m['pct'] == 75.0 and m['swap_pct'] == 25.0 and m['total'] == 15.3
        assert t.red() == (1000, 500), 'lo y veth no cuentan'
        assert t.temperatura() == 72, 'la mas alta valida; 200 grados se descarta'
        g = t.gpu_amd(); assert g['pct'] == 63 and g['vram'] == 3.0 and g['vram_total'] == 8.0
        assert t.legible(1536) == '1.5 KB/s' and t.legible(3 * 2**20) == '3.0 MB/s'
        top = t.mas_cpu({'1': ('steam', 0), '2': ('foot', 0)}, {'1': ('steam', 300), '2': ('foot', 10), '3': ('nuevo', 999)}, 2.0)
        assert top['nombre'] == 'steam' and top['pct'] == 150
        assert t.mas_cpu({'1': ('a', 0)}, {'1': ('a', 0)}, 2.0)['nombre'] == 'en calma'
        # El JSON real tiene todas las claves que usa eww.yuck, y el :initial tambien.
        real = cargar({})
        import io, contextlib
        salida = io.StringIO()
        with contextlib.redirect_stdout(salida): real.datos(vueltas=1, intervalo=.2)
        estado = json.loads(salida.getvalue())
        yuck = (RAIZ / 'src/rootfs/desktop/eww.yuck').read_text()
        inicial = json.loads(re.search(r"\(deflisten d :initial '(.*?)'", yuck, re.S).group(1))
        for ruta in sorted(set(re.findall(r'\bd\.([a-z_]+(?:\.[a-z_]+)*)', yuck))):
            for fuente, nombre in ((estado, 'datos'), (inicial, ':initial')):
                valor = fuente
                for parte in ruta.split('.'):
                    assert isinstance(valor, dict) and parte in valor, f'{nombre} no tiene d.{ruta}'
                    valor = valor[parte]
        assert isinstance(estado['nucleos'], list) and 0 <= estado['cpu'] <= 100
        # eww falso: iniciar, alternar, recargar; sin eww, la version de texto.
        bin_ = tmp / 'bin'; bin_.mkdir(); reg = tmp / 'llamadas'; home = tmp / 'home'; home.mkdir()
        (bin_ / 'eww').write_text(f'#!/bin/bash\nprintf "eww %s\\n" "$*" >> {reg}\n'
                                  f'[[ $3 == active-windows && -f {tmp}/abierta ]] && echo "tarjetas: tarjetas"\nexit 0\n')
        (bin_ / 'foot').write_text(f'#!/bin/bash\nprintf "foot %s\\n" "$*" >> {reg}\n')
        for f in bin_.iterdir(): f.chmod(0o755)
        herramienta = tmp / 'ultra-tarjetas'
        herramienta.write_text(FUENTE.read_text().replace('if os.geteuid() != 1000:', 'if False:'))
        entorno = dict(os.environ, HOME=str(home), PATH=f'{bin_}:/usr/bin:/bin')

        def correr(*args):
            reg.write_text('')
            r = subprocess.run([sys.executable, '-I', str(herramienta), *args], env=entorno, capture_output=True, text=True, timeout=30)
            assert r.returncode == 0, (args, r.stderr)
            return reg.read_text()

        llamadas = correr('iniciar')
        assert 'eww --config /etc/ultra open tarjetas' in llamadas and 'update abierto=true' in llamadas
        (tmp / 'abierta').touch()
        llamadas = correr('alternar'); assert 'update abierto=false' in llamadas and 'close tarjetas' in llamadas
        (tmp / 'abierta').unlink()
        assert 'open tarjetas' in correr('alternar')
        # Ocultas en Personalizar: iniciar no abre; con tema personal usa esa carpeta.
        (home / '.config/ultra/tema-personal/eww').mkdir(parents=True)
        (home / '.config/ultra/tema-personal/eww/eww.yuck').write_text('')
        (home / '.config/ultra/personal.json').write_text(json.dumps({'tarjetas': 'ocultas'}))
        assert 'open' not in correr('iniciar')
        (home / '.config/ultra/personal.json').write_text(json.dumps({'tarjetas': 'visibles'}))
        llamadas = correr('recargar')
        assert f'--config {home}/.config/ultra/tema-personal/eww kill' in llamadas and 'open tarjetas' in llamadas
        (bin_ / 'eww').unlink()
        assert 'foot --title=ULTRA / Equipo en vivo watch -n 2 ultra-equipo texto' in correr('alternar')
        assert correr('iniciar') == ''
    print('VERDE: tarjetas leen CPU, RAM, GPU, red, discos y temperatura; JSON coincide con eww; abrir, ocultar y texto sin eww')


if __name__ == '__main__':
    main()
