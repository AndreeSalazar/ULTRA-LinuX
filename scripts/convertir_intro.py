# SPDX-License-Identifier: GPL-2.0-only
"""Prepara la intro de arranque: el Tux personal de Eddi Salazar con el titulo debajo.

    python scripts/convertir_intro.py [alto_del_tux]      (por defecto 320)

Entrada: src/logo/ultra-tux-demonio.webp (fondo transparente o blanco).
Salida:  src/rootfs/etc/ultra-intro.bin, mismo formato que ultra-logo.bin:
         "ULGO" | u16 ancho | u16 alto (little-endian) | ancho*alto*4 bytes RGBA
El init la anima en /dev/fb0 (src/logo.c, show_intro) y devuelve la pantalla intacta.
"""
import pathlib
import struct
import sys

from PIL import Image, ImageDraw, ImageFilter, ImageFont

RAIZ = pathlib.Path(__file__).resolve().parent.parent
FUENTE = RAIZ / 'src' / 'logo' / 'ultra-tux-demonio.webp'
SALIDA = RAIZ / 'src' / 'rootfs' / 'etc' / 'ultra-intro.bin'
LETRAS = RAIZ / 'src' / 'rootfs' / 'fuentes'
CLAVE = (255, 0, 255)


def transparente(img):
    """Fondo blanco a transparente rellenando solo desde los bordes, como convertir_logo.py."""
    if img.mode == 'RGBA' and img.getextrema()[3][0] == 0:
        return img
    rgb = img.convert('RGB'); marcado = rgb.copy(); w, h = marcado.size; px = marcado.load()
    for x, y in [(x, 0) for x in range(0, w, 8)] + [(x, h - 1) for x in range(0, w, 8)] + \
                [(0, y) for y in range(0, h, 8)] + [(w - 1, y) for y in range(0, h, 8)]:
        if min(px[x, y]) >= 225: ImageDraw.floodfill(marcado, (x, y), CLAVE, thresh=40)
    rgba = rgb.convert('RGBA'); mpx, rpx = marcado.load(), rgba.load()
    for y in range(h):
        for x in range(w):
            if mpx[x, y] == CLAVE: rpx[x, y] = (0, 0, 0, 0)
    return rgba


def texto(lienzo, xy, cadena, fuente, color, espacio):
    """Texto con espacio entre letras (Pillow no lo hace solo)."""
    d = ImageDraw.Draw(lienzo); x, y = xy
    for letra in cadena:
        d.text((x, y), letra, font=fuente, fill=color)
        x += d.textlength(letra, font=fuente) + espacio
    return x


def ancho(cadena, fuente, espacio):
    d = ImageDraw.Draw(Image.new('L', (1, 1)))
    return sum(d.textlength(c, font=fuente) for c in cadena) + espacio * (len(cadena) - 1)


def main():
    alto = int(sys.argv[1]) if len(sys.argv) > 1 else 320
    tux = transparente(Image.open(FUENTE))
    tux = tux.crop(tux.getbbox())
    tux = tux.resize((round(tux.width * alto / tux.height), alto), Image.LANCZOS)
    titulo, sub = 'ULTRA LINUX', 'BMO-X  //  EDDI SALAZAR'
    f_titulo = ImageFont.truetype(str(LETRAS / 'Rajdhani-Bold.ttf'), 58)
    f_sub = ImageFont.truetype(str(LETRAS / 'ChakraPetch-SemiBold.ttf'), 17)
    a_titulo, a_sub = ancho(titulo, f_titulo, 9), ancho(sub, f_sub, 4)
    margen = 24
    lienzo_w = int(max(tux.width, a_titulo, a_sub)) + 2 * margen
    lienzo_h = alto + 140
    lienzo = Image.new('RGBA', (lienzo_w, lienzo_h), (0, 0, 0, 0))
    lienzo.alpha_composite(tux, ((lienzo_w - tux.width) // 2, 0))
    y_titulo = alto + 14
    # Brillo rojo detras del titulo, luego el titulo plateado y el subtitulo en rojo.
    brillo = Image.new('RGBA', lienzo.size, (0, 0, 0, 0))
    texto(brillo, ((lienzo_w - a_titulo) / 2, y_titulo), titulo, f_titulo, (255, 42, 79, 255), 9)
    lienzo.alpha_composite(brillo.filter(ImageFilter.GaussianBlur(7)))
    texto(lienzo, ((lienzo_w - a_titulo) / 2, y_titulo), titulo, f_titulo, (236, 236, 242, 255), 9)
    texto(lienzo, ((lienzo_w - a_sub) / 2, y_titulo + 70), sub, f_sub, (255, 42, 79, 255), 4)
    lienzo = lienzo.crop(lienzo.getbbox())
    SALIDA.write_bytes(b'ULGO' + struct.pack('<HH', lienzo.width, lienzo.height) + lienzo.tobytes())
    (RAIZ / 'build').mkdir(exist_ok=True)
    lienzo.save(RAIZ / 'build' / 'ultra-intro-vista.png')
    print(f'VERDE: {SALIDA.relative_to(RAIZ)}: {lienzo.width}x{lienzo.height}, {SALIDA.stat().st_size // 1024} KiB')


if __name__ == '__main__':
    main()
