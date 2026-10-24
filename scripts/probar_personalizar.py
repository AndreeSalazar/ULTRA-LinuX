#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Prueba ultra-personalizar en un HOME temporal con el tema real y comandos de sesion falsos."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

RAIZ = Path(__file__).resolve().parents[1]
FALSOS = ('hyprctl', 'swaybg', 'waybar', 'mako', 'pkill', 'notify-send', 'ultra-ambiente')


def main():
    fuente = (RAIZ / 'src/rootfs/bin/ultra-personalizar').read_text()
    with tempfile.TemporaryDirectory(prefix='ultra-personalizar-') as tmp:
        tmp = Path(tmp); home = tmp / 'home'; bin_ = tmp / 'bin'; llamadas = tmp / 'llamadas'
        (home / 'Imágenes').mkdir(parents=True); bin_.mkdir()
        (home / '.config/gtk-3.0').mkdir(parents=True)
        (home / '.config/gtk-3.0/settings.ini').write_text('[Settings]\ngtk-cursor-theme-size=28\n')
        for nombre in FALSOS:
            falso = bin_ / nombre
            falso.write_text(f'#!/bin/bash\nprintf "%s\\0" {nombre} "$@" >> {llamadas}; printf "\\n" >> {llamadas}\n')
            falso.chmod(0o755)
        herramienta = tmp / 'ultra-personalizar'
        herramienta.write_text(fuente
            .replace("ETC = Path('/etc/ultra')", f"ETC = Path({str(RAIZ / 'src/rootfs/desktop')!r})")
            .replace("CREAR_FONDO = Path('/usr/local/libexec/ultra-crear-fondo')", f"CREAR_FONDO = Path({str(RAIZ / 'src/rootfs/bin/ultra-crear-fondo')!r})")
            .replace("['/usr/bin/python', str(CREAR_FONDO)", f"[{sys.executable!r}, '-I', str(CREAR_FONDO)")
            .replace('if os.geteuid() != 1000:', 'if False:'))
        runtime = tmp / 'run'; runtime.mkdir(mode=0o700)
        entorno = dict(os.environ, HOME=str(home), PATH=f'{bin_}:{os.environ["PATH"]}', XDG_RUNTIME_DIR=str(runtime),
                       HYPRLAND_INSTANCE_SIGNATURE='prueba', WAYLAND_DISPLAY='wayland-prueba')
        entorno.pop('ULTRA_EFECTOS', None)

        def correr(*args, ok=True):
            llamadas.write_text('')
            r = subprocess.run([sys.executable, '-I', str(herramienta), *args], env=entorno, capture_output=True, text=True, timeout=120)
            assert (r.returncode == 0) == ok, (args, r.returncode, r.stderr)
            return r.stdout, [l.split('\0')[:-1] for l in llamadas.read_text().splitlines() if l]

        tema = home / '.config/ultra/tema-personal'
        # Sin elecciones: no genera nada y el escritorio usa /etc/ultra.
        assert correr('rutas')[0] == '' and correr('acento')[0] == '' and not tema.exists()
        # Acento amarillo: todo el tema cambia y nada queda en hielo.
        _, cmds = correr('poner', 'acento', 'amarillo')
        css = (tema / 'waybar.css').read_text()
        assert '#fcee0a' in css and '#5ad8ff' not in css and 'rgba(252, 238, 10,' in css
        assert 'fcee0aff' in (tema / 'mako.conf').read_text() and 'fcee0aff' in (tema / 'fuzzel.ini').read_text()
        fondo = (tema / 'fondo.svg').read_text()
        assert fondo.startswith('<svg') and '#fcee0a' in fondo
        lua = next(c for c in cmds if c[:2] == ['hyprctl', 'eval'])[2]
        assert 'rgba(fcee0aff)' in lua and 'gaps_in=6' in lua and 'enabled=true' in lua
        assert ['waybar', '-c', str(tema / 'waybar.jsonc'), '-s', str(tema / 'waybar.css')] in cmds
        assert (runtime / 'ultra-efectos').read_text().strip() == 'vivo'
        assert ['mako', '--config', str(tema / 'mako.conf')] in cmds
        assert ['swaybg', '-i', str(tema / 'fondo.svg'), '-m', 'fill'] in cmds
        assert correr('acento')[0].strip() == 'fcee0aff'
        # Opciones de forma: huecos, borde quieto, cursor, barra, ventanas inactivas.
        for clave, valor in (('huecos', 'amplio'), ('borde', 'quieto'), ('cursor', '48'), ('barra', 'grande'), ('inactivas', '85')):
            _, cmds = correr('poner', clave, valor)
        lua = next(c for c in cmds if c[:2] == ['hyprctl', 'eval'])[2]
        assert 'gaps_in=10,gaps_out=24' in lua and 'leaf="borderangle",enabled=false' in lua and 'inactive_opacity=0.85' in lua
        assert ['hyprctl', 'setcursor', 'ULTRA-Nucleo', '48'] in cmds
        assert 'gtk-cursor-theme-size=48' in (home / '.config/gtk-3.0/settings.ini').read_text()
        assert 'font-size: 17px' in (tema / 'waybar.css').read_text() and 'font-size: 27px' in (tema / 'waybar.css').read_text()
        # Barra abajo sin dock, reloj de 12 h, avisos abajo y letra mono.
        for clave, valor in (('posicion', 'abajo'), ('dock', 'oculto'), ('reloj', '12h'), ('avisos', 'abajo-derecha'), ('letra', 'mono')):
            correr('poner', clave, valor)
        barras = json.loads((tema / 'waybar.jsonc').read_text())
        assert [b['name'] for b in barras] == ['hud'] and barras[0]['position'] == 'bottom'
        assert barras[0]['clock']['format'] == '{:%I:%M %p}'
        hud = next(l for l in (tema / 'waybar.css').read_text().splitlines() if l.startswith('window#waybar.hud > box'))
        assert 'border-top: 2px solid' in hud and 'border-bottom' not in hud
        assert '"JetBrains Mono", "DejaVu Sans Mono"' in (tema / 'waybar.css').read_text()
        mako = (tema / 'mako.conf').read_text()
        assert 'anchor=bottom-right' in mako and 'outer-margin=12,0,52' in mako and 'font=JetBrains Mono 11' in mako
        # Dock visible con barra abajo: el dock sube y los avisos lo esquivan.
        correr('poner', 'dock', 'visible'); correr('poner', 'avisos', 'arriba-derecha')
        barras = json.loads((tema / 'waybar.jsonc').read_text())
        assert barras[1]['name'] == 'dock' and barras[1]['position'] == 'top' and 'margin-bottom' not in barras[1]
        assert 'outer-margin=90,0,52' in (tema / 'mako.conf').read_text()
        # Esquinas, animacion, velocidad, desenfoque y efectos.
        for clave, valor in (('esquinas', 'redondas'), ('animacion', 'deslizar'), ('velocidad', 'lenta'), ('desenfoque', 'no')):
            _, cmds = correr('poner', clave, valor)
        lua = next(c for c in cmds if c[:2] == ['hyprctl', 'eval'])[2]
        assert 'rounding=12' in lua and 'blur={enabled=false' in lua and 'animations={enabled=true}' in lua
        assert 'leaf="windows",enabled=true,speed=6.4,bezier="ultraEase",style="slide"' in lua
        _, cmds = correr('poner', 'efectos', 'reducidos')
        lua = next(c for c in cmds if c[:2] == ['hyprctl', 'eval'])[2]
        assert 'animations={enabled=false}' in lua and 'shadow={enabled=false' in lua
        assert (runtime / 'ultra-efectos').read_text().strip() == 'sobrio'
        # ultra-ambiente pide un modo: se respeta sin tocar el estado guardado.
        (runtime / 'ultra-efectos').write_text('sobrio\n')
        _, cmds = correr('hyprland', 'vivo')
        lua = next(c for c in cmds if c[:2] == ['hyprctl', 'eval'])[2]
        assert 'animations={enabled=true}' in lua and 'blur={enabled=false' in lua
        assert (runtime / 'ultra-efectos').read_text().strip() == 'sobrio'
        correr('poner', 'efectos', 'vivos')
        # Color propio en hex y fondos Nucleo, liso e imagen propia.
        correr('poner', 'acento', '#12AB34')
        assert '#12ab34' in (tema / 'waybar.css').read_text()
        correr('poner', 'fondo', 'nucleo')
        assert f'FONDO={RAIZ / "src/rootfs/desktop/wallpaper-nucleo.svg"}' in correr('rutas')[0]
        _, cmds = correr('poner', 'fondo', 'liso')
        assert ['swaybg', '-c', '#06050a'] in cmds
        imagen = home / 'Imágenes/mia.png'; imagen.write_bytes(b'\x89PNG\r\n')
        correr('poner', 'imagen', str(imagen))
        assert f'FONDO={imagen}' in correr('rutas')[0]
        # Rechazos: valores desconocidos, hex mal formado, imagen fuera de carpeta o enlazada.
        for args in (('poner', 'acento', 'rosa'), ('poner', 'acento', '#12345'), ('poner', 'huecos', '999'),
                     ('poner', 'imagen', '/etc/passwd'), ('poner', 'nada', 'x')):
            correr(*args, ok=False)
        enlace = home / 'Imágenes/enlace.png'; enlace.symlink_to(imagen)
        correr('poner', 'imagen', str(enlace), ok=False)
        # Un personal.json corrupto o manipulado vuelve a valores seguros.
        (home / '.config/ultra/personal.json').write_text(json.dumps(dict(acento='$(rm -rf ~)', huecos='9999', fondo='imagen', imagen='/etc/shadow')))
        rutas = correr('rutas')[0]
        assert '#5ad8ff' in (tema / 'waybar.css').read_text() and f'FONDO={RAIZ / "src/rootfs/desktop/wallpaper.svg"}' in rutas
        # Restablecer: borra solo lo propio y vuelve al tema del sistema.
        _, cmds = correr('restablecer')
        assert not (home / '.config/ultra/personal.json').exists() and not (tema / 'waybar.css').exists()
        assert ['waybar', '-c', str(RAIZ / 'src/rootfs/desktop/waybar.jsonc'), '-s', str(RAIZ / 'src/rootfs/desktop/waybar.css')] in cmds
        assert not (tema / 'waybar.jsonc').exists()
        assert imagen.exists() and correr('rutas')[0] == ''
        # Menu: un fuzzel falso elige Acento > verde, Huecos > compacto y Cerrar.
        cola = tmp / 'cola'; cola.write_text('\n'.join(['Colores y fondo', 'Acento: hielo', '  verde', '← volver',
                                                     'Ventanas', 'Huecos entre ventanas: normal', '  compacto', '← volver', 'Cerrar']) + '\n')
        fuzzel = bin_ / 'fuzzel'
        fuzzel.write_text(f'#!/bin/bash\ncat > /dev/null\nlinea=$(head -n1 {cola}); sed -i 1d {cola}\n[[ -n $linea ]] || exit 1\nprintf "%s\\n" "$linea"\n')
        fuzzel.chmod(0o755)
        correr('menu')
        guardado = json.loads((home / '.config/ultra/personal.json').read_text())
        assert guardado['acento'] == 'verde' and guardado['huecos'] == 'compacto', guardado
        assert '#3dff9a' in (tema / 'waybar.css').read_text()
    print('VERDE: personalizar genera, aplica, rechaza entradas invalidas y restablece')


if __name__ == '__main__':
    main()
