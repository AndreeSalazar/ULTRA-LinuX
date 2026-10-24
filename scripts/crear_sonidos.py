# SPDX-License-Identifier: GPL-2.0-only
"""Sonidos propios de BMO-X / Neon, sintetizados aqui (sin muestras de terceros).

    python3 scripts/crear_sonidos.py

Salida: src/rootfs/desktop/sonido-{inicio,captura,critico,juego}.wav
(PCM 16 bits, mono, 22050 Hz, menos de un segundo cada uno). Los reproduce
`ultra-personalizar sonar EVENTO` con pw-play; se apagan en Personalizar.
"""
import math
import pathlib
import random
import struct
import wave

RAIZ = pathlib.Path(__file__).resolve().parent.parent
TASA = 22050


def nota(freq, dur, vol=.35, ataque=.004, caida=6.0, armonico=.25, deslizar=0.0):
    """Seno con un armonico y caida exponencial; deslizar sube o baja el tono."""
    muestras = []
    fase = 0.0
    for i in range(int(TASA * dur)):
        t = i / TASA
        f = freq * (1 + deslizar * t / dur)
        fase += 2 * math.pi * f / TASA
        env = min(1.0, t / ataque) * math.exp(-caida * t)
        muestras.append(vol * env * (math.sin(fase) + armonico * math.sin(2 * fase)))
    return muestras


def silencio(dur):
    return [0.0] * int(TASA * dur)


def mezclar(*pistas):
    largo = max(len(p) for p in pistas)
    return [sum(p[i] for p in pistas if i < len(p)) for i in range(largo)]


def ruido(dur, vol=.08, semilla=2077):
    azar = random.Random(semilla)
    return [vol * azar.uniform(-1, 1) * math.exp(-30 * i / TASA) for i in range(int(TASA * dur))]


SONIDOS = {
    # Arranque: tres notas que suben, como el aro que se enciende.
    'inicio': lambda: nota(392, .16, caida=9) + nota(587, .16, caida=9) + mezclar(nota(784, .45, caida=5), nota(1175, .45, vol=.12, caida=7)),
    # Captura: obturador corto, chasquido y una nota alta.
    'captura': lambda: mezclar(ruido(.05, .22), silencio(.03) + nota(1568, .12, vol=.25, caida=28)),
    # Aviso critico: dos pulsos graves en rojo.
    'critico': lambda: nota(330, .12, caida=14, armonico=.5) + silencio(.05) + nota(262, .2, caida=10, armonico=.5),
    # Modo juego: barrido que baja, el escritorio se apaga para el juego.
    'juego': lambda: nota(880, .35, vol=.28, caida=7, deslizar=-.5),
}


def guardar(nombre, muestras):
    pico = max(abs(m) for m in muestras) or 1.0
    escala = min(1.0, .7 / pico) * 32767
    destino = RAIZ / 'src/rootfs/desktop' / f'sonido-{nombre}.wav'
    with wave.open(str(destino), 'wb') as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(TASA)
        w.writeframes(b''.join(struct.pack('<h', round(m * escala)) for m in muestras))
    return destino


def main():
    for nombre, crear in SONIDOS.items():
        destino = guardar(nombre, crear())
        print(f'VERDE: {destino.relative_to(RAIZ)}: {destino.stat().st_size // 1024} KiB')


if __name__ == '__main__':
    main()
