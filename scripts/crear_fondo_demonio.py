# SPDX-License-Identifier: GPL-2.0-only
"""Fondo «Demonio»: la ciudad Neon en rojo con el Tux del autor dentro del aro.

    python scripts/crear_fondo_demonio.py

Necesita Pillow y Playwright con Chromium (para pasar el SVG de la ciudad a imagen).
Salida: src/rootfs/desktop/wallpaper-demonio.png (1920x1080). Se elige con
ultra-personalizar > Colores y fondo > Fondo: demonio. Tux: Eddi Salazar.
"""
import pathlib
import random
import subprocess
import sys
import tempfile

from PIL import Image, ImageChops, ImageFilter

RAIZ = pathlib.Path(__file__).resolve().parent.parent
TUX = RAIZ / 'src/logo/ultra-tux-demonio.webp'
SALIDA = RAIZ / 'src/rootfs/desktop/wallpaper-demonio.png'
W, H = 1920, 1080


def ciudad(destino):
    svg = subprocess.run([sys.executable, '-I', str(RAIZ / 'src/rootfs/bin/ultra-crear-fondo'), '--acento', '#ff2a4f'],
                         capture_output=True, text=True, check=True).stdout
    with tempfile.TemporaryDirectory() as tmp:
        archivo = pathlib.Path(tmp) / 'ciudad.svg'; archivo.write_text(svg)
        codigo = ("const {chromium}=require('playwright');(async()=>{const b=await chromium.launch();"
                  "const p=await b.newPage({viewport:{width:%d,height:%d}});await p.goto('file://%s');"
                  "await p.waitForTimeout(300);await p.screenshot({path:'%s'});await b.close();})();" % (W, H, archivo, destino))
        subprocess.run(['node', '-e', codigo], check=True, cwd=tmp)


def main():
    with tempfile.TemporaryDirectory() as tmp:
        base_png = pathlib.Path(tmp) / 'base.png'
        ciudad(base_png)
        base = Image.open(base_png).convert('RGBA')
    # Oscurecer un poco la ciudad para que el Tux mande.
    base = Image.blend(base, Image.new('RGBA', base.size, (4, 2, 6, 255)), .28)
    tux = Image.open(TUX).convert('RGBA')
    tux = tux.crop(tux.getbbox())
    alto = 690
    tux = tux.resize((round(tux.width * alto / tux.height), alto), Image.LANCZOS)
    # La cabeza queda dentro del aro de ultra-crear-fondo (centro 0.65*W, 0.23*H).
    x = round(W * .65 - tux.width / 2); y = round(H * .23 - alto * .20)
    # Resplandor rojo detras.
    sombra = Image.new('RGBA', base.size, (0, 0, 0, 0))
    mascara = tux.getchannel('A').point(lambda a: 255 if a > 20 else 0)
    rojo = Image.new('RGBA', tux.size, (255, 42, 79, 255)); rojo.putalpha(mascara)
    sombra.alpha_composite(rojo, (x, y))
    base.alpha_composite(Image.blend(Image.new('RGBA', base.size, (0, 0, 0, 0)), sombra.filter(ImageFilter.GaussianBlur(38)), .75))
    base.alpha_composite(tux, (x, y))
    # Cortes glitch fijos: franjas desplazadas y una copia roja desfasada sobre el Tux.
    azar = random.Random(2077)
    for _ in range(9):
        top = y + azar.randint(0, alto - 30); alto_franja = azar.randint(4, 22)
        caja = (x - 40, top, x + tux.width + 40, top + alto_franja)
        franja = base.crop(caja)
        base.paste(franja, (caja[0] + azar.choice([-1, 1]) * azar.randint(10, 46), top))
    canal_rojo = tux.getchannel('R').point(lambda v: v // 3)
    fantasma = Image.merge('RGBA', (canal_rojo, Image.new('L', tux.size, 0), Image.new('L', tux.size, 8), tux.getchannel('A').point(lambda a: a // 4)))
    base.alpha_composite(fantasma, (x + 9, y))
    # Lineas de escaneo suaves en toda la imagen.
    lineas = Image.new('RGBA', base.size, (0, 0, 0, 0))
    for fila in range(0, H, 4):
        lineas.paste((0, 0, 0, 40), (0, fila, W, fila + 1))
    base.alpha_composite(lineas)
    base.convert('RGB').save(SALIDA, optimize=True)
    print(f'VERDE: {SALIDA.relative_to(RAIZ)}: {SALIDA.stat().st_size // 1024} KiB')


if __name__ == '__main__':
    main()
