#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Prueba ultra-modo-juego con un socket2 de Hyprland falso, hyprctl y ultra-personalizar falsos."""
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import threading
import time

RAIZ = Path(__file__).resolve().parents[1]


def main():
    with tempfile.TemporaryDirectory(prefix='ultra-modo-juego-') as tmp:
        tmp = Path(tmp); bin_ = tmp / 'bin'; bin_.mkdir(); reg = tmp / 'llamadas'; reg.write_text('')
        runtime = tmp / 'run'; (runtime / 'hypr/prueba').mkdir(parents=True)
        ventana = tmp / 'ventana.json'
        (bin_ / 'hyprctl').write_text(f'#!/bin/bash\ncat {ventana}\n')
        (bin_ / 'ultra-personalizar').write_text(f'#!/bin/bash\nprintf "%s\\n" "$*" >> {reg}\n')
        for f in bin_.iterdir(): f.chmod(0o755)
        herramienta = tmp / 'ultra-modo-juego'
        herramienta.write_text((RAIZ / 'src/rootfs/bin/ultra-modo-juego').read_text().replace('if os.geteuid() != 1000:', 'if False:'))

        def poner(clase, completa):
            ventana.write_text(json.dumps({'class': clase, 'fullscreen': completa}))

        poner('foot', 0)
        ruta = runtime / 'hypr/prueba/.socket2.sock'
        servidor = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM); servidor.bind(str(ruta)); servidor.listen(1)
        pasos = [(('steam_app_230410', 2), b'fullscreen>>1\n'),          # Warframe en pantalla completa: entrar
                 (('steam_app_230410', 2), b'activewindow>>steam_app_230410,Warframe\n'),  # sin cambio
                 (('Brave-browser', 2), b'activewindow>>Brave-browser,YouTube\n'),       # navegador completo: salir
                 (('Cyberpunk2077.exe', 1), b'activewindowv2>>abc\n'),   # .exe (Heroic): entrar
                 (('Cyberpunk2077.exe', 0), b'fullscreen>>0\n'),         # ventana: salir
                 (('gamescope', 1), b'fullscreen>>1\n'),                 # entrar; luego Hyprland se cierra
                 ]

        def hyprland():
            con, _ = servidor.accept()
            time.sleep(.3)
            for (clase, completa), evento in pasos:
                poner(clase, completa); con.sendall(b'openlayer>>waybar\n' + evento); time.sleep(.3)
            con.close(); servidor.close(); ruta.unlink()

        hilo = threading.Thread(target=hyprland); hilo.start()
        entorno = dict(os.environ, PATH=f'{bin_}:{os.environ["PATH"]}', XDG_RUNTIME_DIR=str(runtime),
                       HYPRLAND_INSTANCE_SIGNATURE='prueba', ULTRA_MODO_JUEGO_REINTENTOS='1', ULTRA_MODO_JUEGO_ESPERA='0.1')
        r = subprocess.run([sys.executable, '-I', str(herramienta)], env=entorno, capture_output=True, text=True, timeout=30)
        hilo.join()
        assert r.returncode == 0, r.stderr
        llamadas = reg.read_text().split()
        # entrar, salir, entrar, salir, entrar y, al cerrarse Hyprland, salir.
        assert llamadas == ['juego', 'entrar', 'juego', 'salir'] * 3, llamadas
        # Firma rara: no se construye una ruta con ella.
        r = subprocess.run([sys.executable, '-I', str(herramienta)], env=dict(entorno, HYPRLAND_INSTANCE_SIGNATURE='../x'),
                           capture_output=True, text=True, timeout=10)
        assert r.returncode == 1 and 'sin sesion' in r.stderr
    print('VERDE: modo juego entra con Proton, .exe y gamescope en pantalla completa y sale al cambiar o cerrar')


if __name__ == '__main__':
    main()
