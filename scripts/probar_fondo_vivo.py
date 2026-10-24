#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Prueba ultra-fondo-vivo: el bucle no salta, se guarda en cache y se maneja awww (falso)."""
import importlib.machinery
import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import tempfile

from PIL import Image, ImageChops

RAIZ = Path(__file__).resolve().parents[1]
FUENTE = RAIZ / 'src/rootfs/bin/ultra-fondo-vivo'


def cargar(ruta):
    cargador = importlib.machinery.SourceFileLoader('fondo_vivo', str(ruta))
    modulo = importlib.util.module_from_spec(importlib.util.spec_from_loader('fondo_vivo', cargador))
    cargador.exec_module(modulo); return modulo


def main():
    fv = cargar(FUENTE)
    with tempfile.TemporaryDirectory(prefix='ultra-fondo-vivo-') as tmp:
        tmp = Path(tmp)
        # 1. El bucle cierra: t=0 y t=1 son iguales salvo la linea de escaneo (arriba y abajo).
        base = Image.open(RAIZ / 'src/rootfs/desktop/wallpaper-demonio.png').convert('RGBA').resize((480, 270))
        rgb = lambda t: fv.cuadro(base, (255, 42, 79), t).convert('RGB')  # getbbox de RGBA solo mira el alfa
        a, b = rgb(0.0), rgb(1.0)
        assert ImageChops.difference(a.crop((0, 4, 480, 262)), b.crop((0, 4, 480, 262))).getbbox() is None, 'el bucle salta'
        assert ImageChops.difference(a, rgb(.5)).getbbox() is not None, 'no se mueve'
        # 1b. Ciudad 3D: el bucle cierra (camara, cuadricula, coches, aro y lluvia) y se mueve.
        for acento in ((90, 216, 255), (255, 42, 79)):
            c0, c1 = fv.ciudad3d(320, 180, acento, 0.0).convert('RGB'), fv.ciudad3d(320, 180, acento, 1.0).convert('RGB')
            assert ImageChops.difference(c0, c1).getbbox() is None, 'la ciudad 3D salta'
            assert ImageChops.difference(c0, fv.ciudad3d(320, 180, acento, .3).convert('RGB')).getbbox() is not None
        g3, q3 = fv.crear3d('#5ad8ff', tmp / 'c3d.gif', tam=(160, 90), cuadros=4)
        assert Image.open(g3).n_frames == 4 and Image.open(q3).size == (160, 90)
        # 2. crear: GIF en bucle con su cuadro quieto.
        r = subprocess.run([sys.executable, '-I', str(FUENTE), 'crear', str(RAIZ / 'src/rootfs/desktop/wallpaper-demonio.png'),
                            '#ff2a4f', str(tmp / 'mini.gif'), '--tam', '192x108', '--cuadros', '6'], capture_output=True, text=True, timeout=60)
        assert r.returncode == 0, r.stderr
        gif = Image.open(tmp / 'mini.gif')
        assert gif.n_frames == 6 and gif.info.get('loop') == 0 and gif.size == (192, 108) and (tmp / 'mini.png').is_file()
        for args in (('crear', 'x.png', 'rojo', 'y.gif'), ('crear', 'x.png', '#ff2a4f', 'y.gif', '--tam', '1;rm')):
            assert subprocess.run([sys.executable, '-I', str(FUENTE), *args], capture_output=True, timeout=30).returncode == 1
        # 3. poner, pausar, seguir y parar con awww falso.
        home = tmp / 'home'; home.mkdir(); run = tmp / 'run'; run.mkdir(); bin_ = tmp / 'bin'; bin_.mkdir(); reg = tmp / 'llamadas'
        falsos = {
            'awww': f'printf "awww %s\\n" "$*" >> {reg}; [[ $1 != query ]] || [[ -f {tmp}/daemon ]]',
            'awww-daemon': f'touch {tmp}/daemon; printf "daemon\\n" >> {reg}',
            'pkill': f'printf "pkill %s\\n" "$*" >> {reg}',
            'rsvg-convert': f'printf "rsvg %s\\n" "$*" >> {reg}; for a; do [[ $prev == -o ]] && out=$a; prev=$a; done; '
                            f'{sys.executable} -c "from PIL import Image; Image.new(\'RGB\',(320,180),(6,5,10)).save(\'$out\')"',
        }
        for nombre, cuerpo in falsos.items():
            (bin_ / nombre).write_text('#!/bin/bash\n' + cuerpo + '\n'); (bin_ / nombre).chmod(0o755)
        herramienta = tmp / 'ultra-fondo-vivo'
        herramienta.write_text(FUENTE.read_text()
            .replace("ETC = Path('/etc/ultra')", f"ETC = Path({str(RAIZ / 'src/rootfs/desktop')!r})")
            .replace("CREAR_FONDO = Path('/usr/local/libexec/ultra-crear-fondo')", f"CREAR_FONDO = Path({str(RAIZ / 'src/rootfs/bin/ultra-crear-fondo')!r})")
            .replace("['/usr/bin/python', str(CREAR_FONDO)", f"[{sys.executable!r}, '-I', str(CREAR_FONDO)")
            .replace('if os.geteuid() != 1000:', 'if False:'))
        entorno = dict(os.environ, HOME=str(home), XDG_RUNTIME_DIR=str(run), PATH=f'{bin_}:/usr/bin:/bin')

        def correr(*args, env=entorno):
            reg.write_text('')
            r = subprocess.run([sys.executable, '-I', str(herramienta), *args], env=env, capture_output=True, text=True, timeout=120)
            assert r.returncode == 0, (args, r.stderr)
            return reg.read_text(), r

        llamadas, _ = correr('poner', 'neon', '#3DFF9A')
        cache = home / '.cache/ultra/fondo-vivo/neon-3dff9a-v1.gif'
        assert cache.is_file() and cache.with_suffix('.png').is_file()
        assert 'rsvg' in llamadas and 'daemon' in llamadas and f'awww img {cache} --transition-type fade' in llamadas
        assert 'pkill -x -u' in llamadas and 'swaybg' in llamadas and (run / 'ultra-fondo-vivo').read_text().strip() == str(cache)
        llamadas, _ = correr('poner', 'neon', '#3dff9a')
        assert 'rsvg' not in llamadas and 'daemon' not in llamadas, 'segunda vez: cache y daemon ya listos'
        llamadas, _ = correr('pausar'); assert f'awww img {cache.with_suffix(".png")} --transition-type none' in llamadas
        llamadas, _ = correr('seguir'); assert f'awww img {cache} --transition-type fade' in llamadas
        llamadas, _ = correr('parar'); assert 'awww kill' in llamadas and not (run / 'ultra-fondo-vivo').exists()
        llamadas, _ = correr('pausar'); assert 'img' not in llamadas, 'parado: pausar no hace nada'
        # Tu propia imagen: se recorta sin deformar, sin aro ni letreros, cache por archivo.
        (home / 'Imágenes').mkdir()
        from PIL import Image as I
        propia = home / 'Imágenes/cyberpunk.jpg'; I.new('RGB', (800, 800), (40, 10, 60)).save(propia)
        llamadas, _ = correr('poner', 'imagen', '#5ad8ff', str(propia))
        gifs = sorted((home / '.cache/ultra/fondo-vivo').glob('imagen-*-5ad8ff-v1.gif'))
        assert len(gifs) == 1 and I.open(gifs[0]).size == (1920, 1080) and f'awww img {gifs[0]}' in llamadas
        enlace = home / 'Imágenes/enlace.jpg'; enlace.symlink_to(propia)
        for ruta in (str(enlace), '/etc/passwd', str(tmp / 'mini.png')):
            r = subprocess.run([sys.executable, '-I', str(herramienta), 'poner', 'imagen', '#5ad8ff', ruta], env=entorno, capture_output=True, text=True, timeout=30)
            assert r.returncode == 1 and 'imagen no valida' in r.stderr, ruta
        plano = I.new('RGBA', (320, 180), (20, 20, 30, 255))
        a0, a1 = fv.cuadro(plano, (90, 216, 255), 0.0, propio=True).convert('RGB'), fv.cuadro(plano, (90, 216, 255), 1.0, propio=True).convert('RGB')
        assert ImageChops.difference(a0.crop((0, 3, 320, 176)), a1.crop((0, 3, 320, 176))).getbbox() is None, 'el bucle propio salta'
        # Sin awww ni swww: se queda el fondo estatico, sin error.
        (bin_ / 'awww').unlink(); (bin_ / 'awww-daemon').unlink()
        llamadas, r = correr('poner', 'demonio')
        assert 'AMARILLO' in r.stderr and 'swaybg' not in llamadas
    print('VERDE: fondo vivo en bucle sin saltos, cache por acento, pausa en juego y sin awww se queda el estatico')


if __name__ == '__main__':
    main()
