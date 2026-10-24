#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Genera fondo-neon.svg: ciudad nocturna original para BMO-X / Neon.

Semilla fija: la misma salida en cada ejecucion. Sin descargas ni fuentes
externas; los letreros usan DejaVu Sans Condensed, ya incluida en el sistema.
Uso: python3 crear-fondo.py [ancho alto] > fondo-neon.svg
"""
import random
import sys

W, H = (int(sys.argv[1]), int(sys.argv[2])) if len(sys.argv) == 3 else (1920, 1080)
HORIZONTE = int(H * 0.70)
rnd = random.Random(2077)
out = []
p = out.append

p(f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">')
p('<title>BMO-X / Neon · ciudad nocturna original</title>')
p('<desc>Arte estatico. Los letreros y ventanas son decoracion, no datos del equipo.</desc>')
p('''<defs>
<linearGradient id="cielo" x1="0" y1="0" x2="0" y2="1">
 <stop offset="0" stop-color="#040309"/><stop offset=".45" stop-color="#0b0816"/>
 <stop offset=".62" stop-color="#2a0a1e"/><stop offset=".70" stop-color="#4a0d24"/>
 <stop offset=".701" stop-color="#0a0710"/><stop offset="1" stop-color="#030205"/>
</linearGradient>
<radialGradient id="brillo" cx=".5" cy="1" r=".8"><stop offset="0" stop-color="#ff2a4f" stop-opacity=".45"/><stop offset="1" stop-color="#ff2a4f" stop-opacity="0"/></radialGradient>
<linearGradient id="reflejo" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ff2a4f" stop-opacity=".10"/><stop offset="1" stop-color="#ff2a4f" stop-opacity="0"/></linearGradient>
<pattern id="lineas" width="4" height="4" patternUnits="userSpaceOnUse"><path d="M0 .5h4" stroke="#000" stroke-opacity=".28"/></pattern>
<pattern id="peligro" width="22" height="22" patternUnits="userSpaceOnUse" patternTransform="rotate(45)"><rect width="11" height="22" fill="#fcee0a"/><rect x="11" width="11" height="22" fill="#0a0710"/></pattern>
<filter id="neon" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="6" result="b"/><feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter>
<filter id="suave"><feGaussianBlur stdDeviation="3"/></filter>
<linearGradient id="haz" x1="0" y1="1" x2="0" y2="0"><stop offset="0" stop-color="#00f0ff" stop-opacity=".16"/><stop offset="1" stop-color="#00f0ff" stop-opacity="0"/></linearGradient>
<linearGradient id="haz2" x1="0" y1="1" x2="0" y2="0"><stop offset="0" stop-color="#fcee0a" stop-opacity=".10"/><stop offset="1" stop-color="#fcee0a" stop-opacity="0"/></linearGradient>
<radialGradient id="bruma" cx=".8" cy=".1" r=".6"><stop offset="0" stop-color="#5a1a6e" stop-opacity=".35"/><stop offset="1" stop-color="#5a1a6e" stop-opacity="0"/></radialGradient>
</defs>''')
p(f'<rect width="{W}" height="{H}" fill="url(#cielo)"/>')
p(f'<rect width="{W}" height="{H}" fill="url(#bruma)"/>')
p(f'<path d="M{W*.30} {HORIZONTE}L{W*.18} 0H{W*.24}Z" fill="url(#haz)"/>')
p(f'<path d="M{W*.64} {HORIZONTE}L{W*.74} 0H{W*.82}Z" fill="url(#haz2)"/>')
p(f'<ellipse cx="{W/2}" cy="{HORIZONTE}" rx="{W*0.7}" ry="{H*0.42}" fill="url(#brillo)"/>')

# Tres capas de edificios: lejos (morado), medio y cerca (casi negro).
p('<g id="ciudad">')
capas = [
    ("#160f24", 0.30, 0.55, 40, 110, 0.10),
    ("#0e0a17", 0.18, 0.42, 60, 150, 0.20),
    ("#07050c", 0.08, 0.30, 90, 210, 0.32),
]
for color, alto_min, alto_max, ancho_min, ancho_max, luz in capas:
    x = -20
    while x < W:
        ancho = rnd.randint(ancho_min, ancho_max)
        # El centro se mantiene mas bajo para dejar despejada la zona de trabajo.
        centro = 1 - abs((x + ancho / 2) / W - 0.5) * 2
        alto = int(H * rnd.uniform(alto_min, alto_max) * (1 - 0.45 * centro))
        top = HORIZONTE - alto
        p(f'<rect x="{x}" y="{top}" width="{ancho}" height="{alto}" fill="{color}"/>')
        if rnd.random() < 0.35:
            antena = rnd.randint(20, 70)
            p(f'<path d="M{x + ancho // 2} {top}v-{antena}" stroke="{color}" stroke-width="3"/>')
            p(f'<circle cx="{x + ancho // 2}" cy="{top - antena}" r="2.5" fill="#ff2a4f" filter="url(#neon)"/>')
        for fy in range(top + 10, HORIZONTE - 8, 14):
            for fx in range(x + 6, x + ancho - 8, 11):
                if rnd.random() < luz:
                    tono = rnd.choices(["#fcee0a", "#00f0ff", "#ff2a4f", "#f6e7c1"], [5, 3, 1, 3])[0]
                    p(f'<rect x="{fx}" y="{fy}" width="5" height="6" fill="{tono}" opacity="{rnd.uniform(.25, .8):.2f}"/>')
        x += ancho + rnd.randint(0, 14)

# Letreros verticales con brillo.
p('<g id="letreros">')
letreros = [(int(W * .085), "ULTRA", "#fcee0a"), (int(W * .87), "BMO-X", "#00f0ff"), (int(W * .78), "24H", "#ff2a4f")]
for lx, texto, color in letreros:
    ly = HORIZONTE - int(H * .40)
    alto = len(texto) * 42 + 20
    p(f'<rect x="{lx - 4}" y="{ly}" width="44" height="{alto}" fill="#05040a" stroke="{color}" stroke-width="2" filter="url(#neon)"/>')
    for i, letra in enumerate(texto):
        p(f'<text x="{lx + 18}" y="{ly + 44 + i * 42}" font-family="DejaVu Sans Condensed, DejaVu Sans, sans-serif" font-weight="bold" font-size="34" text-anchor="middle" fill="{color}" filter="url(#neon)">{letra}</text>')

p('</g></g>')
# Suelo oscuro: el brillo rojo queda solo en el cielo.
p(f'<rect y="{HORIZONTE}" width="{W}" height="{H - HORIZONTE}" fill="#05030a"/>')
# Reflejo de la ciudad en el suelo mojado.
p(f'<use href="#ciudad" transform="matrix(1 0 0 -0.55 0 {HORIZONTE * 1.55})" opacity=".22" filter="url(#suave)"/>')
# Linea de horizonte y reflejos en el suelo mojado.
p(f'<rect y="{HORIZONTE}" width="{W}" height="{H - HORIZONTE}" fill="url(#reflejo)"/>')
p(f'<path d="M0 {HORIZONTE}H{W}" stroke="#ff2a4f" stroke-width="2" filter="url(#neon)" opacity=".9"/>')
for i in range(1, 9):
    y = HORIZONTE + i * i * 4
    p(f'<path d="M0 {y}H{W}" stroke="#ff2a4f" stroke-opacity="{0.22 - i * 0.02:.2f}"/>')
for i in range(-14, 15):
    p(f'<path d="M{W/2 + i * 30} {HORIZONTE}L{W/2 + i * 260} {H}" stroke="#00f0ff" stroke-opacity=".07"/>')

# Lluvia fina.
for _ in range(260):
    x, y, largo = rnd.randint(0, W), rnd.randint(0, H), rnd.randint(14, 40)
    p(f'<path d="M{x} {y}l-{largo // 4} {largo}" stroke="#a8c8ff" stroke-opacity="{rnd.uniform(.05, .16):.2f}"/>')

# Placa amarilla inferior izquierda: firma del tema.
px, py = 0, H - 150
p(f'<path d="M{px} {py}H{px + 520}L{px + 470} {py + 70}H{px}Z" fill="#fcee0a"/>')
p(f'<path d="M{px} {py + 78}H{px + 440}L{px + 425} {py + 98}H{px}Z" fill="url(#peligro)"/>')
p(f'<text x="{px + 34}" y="{py + 48}" font-family="DejaVu Sans Condensed, DejaVu Sans, sans-serif" font-weight="bold" font-size="36" letter-spacing="6" fill="#0a0710">ULTRA LINUX</text>')
p(f'<text x="{px + 378}" y="{py + 48}" font-family="DejaVu Sans Mono, monospace" font-size="15" fill="#0a0710">//BMO-X</text>')
p(f'<text x="{px + 34}" y="{py + 128}" font-family="DejaVu Sans Mono, monospace" font-size="13" letter-spacing="3" fill="#fcee0a" opacity=".7">SYS.01 · NEON · PERSONAL SYSTEM</text>')

# Esquinas de HUD.
c = 36
for x0, y0, dx, dy in [(24, 24, 1, 1), (W - 24, 24, -1, 1), (W - 24, H - 24, -1, -1)]:
    p(f'<path d="M{x0} {y0 + dy * c}V{y0}H{x0 + dx * c}" fill="none" stroke="#00f0ff" stroke-opacity=".55" stroke-width="2"/>')
p(f'<path d="M{W - 260} {H - 40}H{W - 60}" stroke="#ff2a4f" stroke-width="2"/><path d="M{W - 300} {H - 40}h20" stroke="#ff2a4f" stroke-width="2"/>')

p(f'<rect width="{W}" height="{H}" fill="url(#lineas)"/>')
p('</svg>')
print('\n'.join(out))
