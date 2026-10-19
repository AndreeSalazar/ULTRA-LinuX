# SPDX-License-Identifier: GPL-2.0-only
"""Turns the ULTRA logo into raw pixels the init can draw on the framebuffer.

    python scripts/convertir_logo.py [alto]        (alto en pixeles, por defecto 300)

Input:  src/logo/ultra-tux.webp (any size, on a white background).
Output: src/rootfs/etc/ultra-logo.bin
        "ULGO" | u16 width | u16 height (little-endian) | width*height*4 bytes RGBA

The white background becomes transparent by flood-filling from the borders only, so
the white and metallic parts inside the penguin (belly, teeth) keep their colour.
Based on Tux by Larry Ewing (lewing@isc.tamu.edu), drawn with The GIMP.
"""
import pathlib, struct, sys
from PIL import Image, ImageDraw

ROOT = pathlib.Path(__file__).resolve().parent.parent
SRC = ROOT / 'src' / 'logo' / 'ultra-tux.webp'
OUT = ROOT / 'src' / 'rootfs' / 'etc' / 'ultra-logo.bin'
KEY = (255, 0, 255)  # a colour the drawing never uses, to mark the background

def main():
    height = int(sys.argv[1]) if len(sys.argv) > 1 else 300
    img = Image.open(SRC).convert('RGB')
    # Mark the background: flood fill near-white from every border pixel that is near-white.
    marked = img.copy()
    w, h = marked.size
    px = marked.load()
    for x, y in [(x, 0) for x in range(0, w, 8)] + [(x, h - 1) for x in range(0, w, 8)] + \
                [(0, y) for y in range(0, h, 8)] + [(w - 1, y) for y in range(0, h, 8)]:
        if min(px[x, y]) >= 225:
            ImageDraw.floodfill(marked, (x, y), KEY, thresh=40)
    rgba = img.convert('RGBA')
    alpha = Image.new('L', (w, h), 255)
    mpx, apx = marked.load(), alpha.load()
    for y in range(h):
        for x in range(w):
            if mpx[x, y] == KEY:
                apx[x, y] = 0
    rgba.putalpha(alpha)
    # Trim the transparent margin, then scale with a smooth filter.
    rgba = rgba.crop(rgba.getbbox())
    width = round(rgba.width * height / rgba.height)
    rgba = rgba.resize((width, height), Image.LANCZOS)
    OUT.write_bytes(b'ULGO' + struct.pack('<HH', width, height) + rgba.tobytes())
    rgba.save(ROOT / 'build' / 'ultra-logo-vista.png')
    print(f'VERDE: {OUT.relative_to(ROOT)}: {width}x{height}, {OUT.stat().st_size // 1024} KiB')

if __name__ == '__main__':
    main()
