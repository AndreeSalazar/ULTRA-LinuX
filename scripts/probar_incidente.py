#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Prueba ultra-incidente en un HOME temporal con eventos del kernel y registros falsos."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

RAIZ = Path(__file__).resolve().parents[1]


def main():
    fuente = (RAIZ / 'src/rootfs/bin/ultra-incidente').read_text()
    with tempfile.TemporaryDirectory(prefix='ultra-incidente-') as tmp:
        tmp = Path(tmp); home = tmp / 'home'; bin_ = tmp / 'bin'; bin_.mkdir()
        (bin_ / 'notify-send').write_text('#!/bin/bash\nexit 0\n'); (bin_ / 'notify-send').chmod(0o755)
        eventos = tmp / 'eventos.json'
        eventos.write_text(json.dumps(dict(status='escuchando', events=[
            dict(type='seccomp_audit', kernel_s=12.5, pid=os.getpid(), code=165),
            dict(type='nvidia_xid', kernel_s=30.0, pid=1, code=109),
            dict(type='storage_error', kernel_s=31.0, pid=None, code=None),
            dict(type='inventado', kernel_s=1.0)])))
        cache = home / '.cache/ultra'; logs = home / '.local/share/Steam/logs'
        cache.mkdir(parents=True); logs.mkdir(parents=True)
        (cache / 'steam.log').write_text('inicio\nusuario eddi@example.com\ntoken=abc123secreto\n' + 'x' * 10 + '\n')
        (logs / 'content_log.txt').write_text(''.join(f'[2026-10-24] AppID 230410 verificando {i}\n' for i in range(5000)))
        (logs / 'connection_log.txt').write_text('cuenta secreta\n')
        afuera = tmp / 'ajeno.log'; afuera.write_text('no debe copiarse\n')
        (cache / 'cyberpunk.log').symlink_to(afuera)
        herramienta = tmp / 'ultra-incidente'
        herramienta.write_text(fuente.replace("EVENTOS = Path('/run/ultra-kernel/eventos.json')", f'EVENTOS = Path({str(eventos)!r})')
                                     .replace('if os.geteuid() != 1000:', 'if False:'))
        entorno = dict(os.environ, HOME=str(home), PATH=f'{bin_}:{os.environ["PATH"]}')

        def correr(*args, ok=True):
            r = subprocess.run([sys.executable, '-I', str(herramienta), *args], env=entorno, capture_output=True, text=True, timeout=60)
            assert (r.returncode == 0) == ok, (args, r.returncode, r.stderr)
            return r.stdout.strip()

        destino = Path(correr('guardar', 'Steam', 'verificando', 'Warframe'))
        resumen = json.loads((destino / 'resumen.json').read_text())
        tipos = [e['tipo'] for e in resumen['kernel']['eventos']]
        assert tipos == ['seccomp_audit', 'nvidia_xid', 'storage_error'], tipos
        assert resumen['kernel']['eventos'][0]['syscall'] == 165 and resumen['nota'] == 'Steam verificando Warframe'
        assert 'steam' in resumen['registros']['guardados'] and 'cyberpunk' in resumen['registros']['ausentes']
        steam = (destino / 'registros/steam.txt').read_text()
        assert 'eddi@example.com' not in steam and 'abc123secreto' not in steam and '[correo]' in steam
        contenido = (destino / 'registros/steam-content_log.txt').read_text().splitlines()
        assert len(contenido) == 200 and contenido[-1].endswith('verificando 4999')
        assert not (destino / 'registros/cyberpunk.txt').exists()
        assert not any('connection' in p.name for p in (destino / 'registros').iterdir())
        assert oct(destino.stat().st_mode & 0o777) == '0o700'
        assert all(k in resumen for k in ('memoria', 'presion_avg10', 'espacio', 'montajes', 'procesos'))
        # Limite: ocho incidentes como maximo.
        for i in range(7):
            (home / f'.cache/ultra/incidentes/incidente-relleno-{i}').mkdir()
        correr('guardar', ok=False)
    print('VERDE: incidente guarda eventos, tapa datos, rechaza enlaces y respeta el limite')


if __name__ == '__main__':
    main()
