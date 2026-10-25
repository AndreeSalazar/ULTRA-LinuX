#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Prueba ultra-personalizar en un HOME temporal con el tema real y comandos de sesion falsos."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

RAIZ = Path(__file__).resolve().parents[1]
FALSOS = ('hyprctl', 'swaybg', 'waybar', 'mako', 'makoctl', 'pkill', 'notify-send', 'pw-play', 'ultra-ambiente', 'ultra-fondo-vivo', 'ultra-tarjetas')


def main():
    fuente = (RAIZ / 'src/rootfs/bin/ultra-personalizar').read_text()
    with tempfile.TemporaryDirectory(prefix='ultra-personalizar-') as tmp:
        tmp = Path(tmp); home = tmp / 'home'; bin_ = tmp / 'bin'; llamadas = tmp / 'llamadas'
        (home / 'Imágenes').mkdir(parents=True); bin_.mkdir()
        (home / '.config/gtk-3.0').mkdir(parents=True)
        (home / '.config/gtk-3.0/settings.ini').write_text('[Settings]\ngtk-cursor-theme-size=28\n')
        foot = home / '.config/foot/foot.ini'; foot.parent.mkdir(parents=True)
        foot.write_text((RAIZ / 'src/rootfs/desktop/foot.ini').read_text())
        for nombre in FALSOS:
            falso = bin_ / nombre
            # Una sola escritura por llamada: varios falsos en segundo plano no mezclan sus lineas.
            falso.write_text(f'#!/bin/bash\nprintf "%s\\0" {nombre} "$@" $\'\\n\' >> {llamadas}\n')
            falso.chmod(0o755)
        herramienta = tmp / 'ultra-personalizar'
        herramienta.write_text(fuente
            .replace("ETC = Path('/etc/ultra')", f"ETC = Path({str(RAIZ / 'src/rootfs/desktop')!r})")
            .replace("CREAR_FONDO = Path('/usr/local/libexec/ultra-crear-fondo')", f"CREAR_FONDO = Path({str(RAIZ / 'src/rootfs/bin/ultra-crear-fondo')!r})")
            .replace("['/usr/bin/python', str(CREAR_FONDO)", f"[{sys.executable!r}, '-I', str(CREAR_FONDO)")
            .replace('if os.geteuid() != 1000:', 'if False:')
            # Hora simulada para el fondo auto y vigilar sin esperas.
            .replace('import time\n', "import time\nHORAS = [int(h) for h in os.environ.get('HORAS_PRUEBA', '12').split(',')]\n", 1)
            .replace('hora = time.localtime().tm_hour if hora is None else hora', 'hora = HORAS.pop(0) if hora is None else hora')
            .replace("elif accion == 'vigilar': vigilar()", "elif accion == 'vigilar': vigilar(intervalo=0, vueltas=3)"))
        runtime = tmp / 'run'; runtime.mkdir(mode=0o700)
        entorno = dict(os.environ, HOME=str(home), PATH=f'{bin_}:{os.environ["PATH"]}', XDG_RUNTIME_DIR=str(runtime),
                       HYPRLAND_INSTANCE_SIGNATURE='prueba', WAYLAND_DISPLAY='wayland-prueba')
        entorno.pop('ULTRA_EFECTOS', None)

        def correr(*args, ok=True, **extra):
            llamadas.write_text('')
            r = subprocess.run([sys.executable, '-I', str(herramienta), *args], env=dict(entorno, **extra), capture_output=True, text=True, timeout=120)
            # Lo lanzado en segundo plano (swaybg, pw-play, tarjetas) anota tarde: esperar a que el registro se quede quieto.
            previo, quieto = None, 0
            for _ in range(60):
                time.sleep(.05)
                actual = llamadas.read_text()
                quieto = quieto + 1 if actual == previo else 0
                if quieto >= 4: break
                previo = actual
            assert (r.returncode == 0) == ok, (args, r.returncode, r.stderr)
            return r.stdout, [l.lstrip('\0').split('\0')[:-1] for l in llamadas.read_text().splitlines() if l.strip('\0')]

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
        # Perfiles: de fabrica, propios, nombres invalidos y borrado.
        _, cmds = correr('perfil', 'aplicar', 'demonio')
        estado = json.loads((home / '.config/ultra/personal.json').read_text())
        assert estado['acento'] == 'rojo' and estado['fondo'] == 'demonio' and estado['dock'] == 'visible'
        assert ['swaybg', '-i', str(RAIZ / 'src/rootfs/desktop/wallpaper-demonio.png'), '-m', 'fill'] in cmds
        _, cmds = correr('perfil', 'aplicar', 'ciudad-3d')
        estado = json.loads((home / '.config/ultra/personal.json').read_text())
        assert estado['fondo'] == 'ciudad-3d' and estado['tarjetas'] == 'visibles' and estado['sonidos'] == 'si'
        assert ['ultra-fondo-vivo', 'poner', 'ciudad3d', '#5ad8ff'] in cmds
        correr('perfil', 'aplicar', 'minimal')
        assert [b['name'] for b in json.loads((tema / 'waybar.jsonc').read_text())] == ['hud']
        correr('perfil', 'guardar', 'mi-noche')
        assert 'mi-noche' in correr('perfiles')[0].split()
        correr('perfil', 'aplicar', 'neon-hielo')
        correr('perfil', 'aplicar', 'mi-noche')
        assert json.loads((home / '.config/ultra/personal.json').read_text())['dock'] == 'oculto'
        for args in (('perfil', 'guardar', 'Mal Nombre'), ('perfil', 'guardar', '../fuera'), ('perfil', 'guardar', 'demonio'),
                     ('perfil', 'aplicar', 'no-existe'), ('perfil', 'borrar', 'demonio')):
            correr(*args, ok=False)
        (home / '.config/ultra/perfiles/roto.json').write_text('{"acento": "$(rm)", "dock": "oculto"}')
        correr('perfil', 'aplicar', 'roto')
        assert json.loads((home / '.config/ultra/personal.json').read_text())['acento'] == 'hielo'
        correr('perfil', 'borrar', 'mi-noche'); assert 'mi-noche' not in correr('perfiles')[0].split()
        correr('perfil', 'aplicar', 'neon-hielo')
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
        # Terminal: seleccion y cursor siguen al acento; la paleta del usuario no se toca.
        correr('poner', 'acento', 'rojo')
        texto = foot.read_text()
        assert 'selection-background=ff2a4f' in texto and 'cursor=000000 ff2a4f' in texto and 'bright4=5ad8ff' in texto
        # Fondo auto: Neon de dia (con el acento), Demonio de noche; vigilar cambia al pasar la hora.
        correr('poner', 'fondo', 'auto')
        assert f'FONDO={tema / "fondo.svg"}' in correr('rutas', HORAS_PRUEBA='12')[0]
        assert f'FONDO={RAIZ / "src/rootfs/desktop/wallpaper-demonio.png"}' in correr('rutas', HORAS_PRUEBA='22')[0]
        _, cmds = correr('vigilar', HORAS_PRUEBA='18,18,19,19')
        assert ['swaybg', '-i', str(RAIZ / 'src/rootfs/desktop/wallpaper-demonio.png'), '-m', 'fill'] in cmds
        _, cmds = correr('vigilar', HORAS_PRUEBA='12,12,12')
        assert not any(c[0] == 'swaybg' for c in cmds)
        (runtime / 'ultra-modo-juego').write_text('activo\n')
        _, cmds = correr('vigilar', HORAS_PRUEBA='18,18,19,19')
        assert not any(c[0] == 'swaybg' for c in cmds), 'en juego el fondo espera'
        (runtime / 'ultra-modo-juego').unlink()
        # Fondo vivo: arranca el estatico y pide el animado con tu acento; otro fondo lo para.
        _, cmds = correr('poner', 'fondo', 'vivo')
        assert ['ultra-fondo-vivo', 'poner', 'neon', '#ff2a4f'] in cmds, cmds
        rutas = correr('rutas')[0]
        assert 'VIVO=neon' in rutas and 'VIVO_ACENTO=#ff2a4f' in rutas
        _, cmds = correr('poner', 'fondo', 'vivo-demonio')
        assert ['ultra-fondo-vivo', 'poner', 'demonio', '#ff2a4f'] in cmds
        _, cmds = correr('poner', 'fondo', 'ciudad-3d')
        assert ['ultra-fondo-vivo', 'poner', 'ciudad3d', '#ff2a4f'] in cmds
        assert 'VIVO=ciudad3d' in correr('rutas')[0]
        _, cmds = correr('poner', 'fondo', 'neon')
        assert ['ultra-fondo-vivo', 'parar'] in cmds
        # Tu imagen animada: primero quieta y luego con lluvia y escaneo.
        propia = home / 'Imágenes/ciudad.png'; propia.write_bytes(b'\x89PNG\r\n')
        _, cmds = correr('poner', 'imagen-viva', str(propia))
        assert ['ultra-fondo-vivo', 'poner', 'imagen', '#ff2a4f', str(propia)] in cmds, cmds
        assert f'VIVO_IMAGEN={propia}' in correr('rutas')[0] and f'FONDO={propia}' in correr('rutas')[0]
        correr('poner', 'imagen-viva', '/etc/passwd', ok=False)
        # Tarjetas eww: copia con tu acento; ocultarlas las para.
        _, cmds = correr('poner', 'tarjetas', 'visibles')
        assert ['ultra-tarjetas', 'recargar'] in cmds
        scss = (tema / 'eww/eww.scss').read_text()
        assert '$acento: #ff2a4f' in scss and '#5ad8ff' not in scss and (tema / 'eww/eww.yuck').is_file()
        _, cmds = correr('poner', 'tarjetas', 'ocultas')
        assert ['ultra-tarjetas', 'parar'] in cmds
        correr('poner', 'tarjetas', 'visibles')
        # Efectos reducidos: la barra deja de latir; las alertas siguen.
        correr('poner', 'efectos', 'reducidos')
        css = (tema / 'waybar.css').read_text()
        assert 'animation: latido' not in css and 'animation: linea' not in css and 'animation: alerta' in css
        correr('poner', 'efectos', 'vivos')
        assert 'animation: latido' in (tema / 'waybar.css').read_text()
        # Cada elemento tiene su sonido y su volumen; 'basicos' calla los de ventanas.
        _, cmds = correr('sonar', 'ventana-abrir')
        assert ['pw-play', '--volume=0.35', str(RAIZ / 'src/rootfs/desktop/sonido-ventana-abrir.wav')] in cmds
        correr('poner', 'sonidos', 'basicos')
        assert not any(c[0] == 'pw-play' for c in correr('sonar', 'ventana-abrir')[1])
        assert any(c[0] == 'pw-play' for c in correr('sonar', 'aviso')[1])
        correr('poner', 'sonidos', 'si')
        # En juego callan ventanas y avisos; lo critico suena.
        (runtime / 'ultra-modo-juego').write_text('activo\n')
        assert not any(c[0] == 'pw-play' for c in correr('sonar', 'escritorio')[1] + correr('sonar', 'aviso')[1])
        assert any(c[0] == 'pw-play' for c in correr('sonar', 'critico')[1])
        (runtime / 'ultra-modo-juego').unlink()
        # Sonidos propios: solo eventos conocidos y se pueden apagar.
        _, cmds = correr('sonar', 'captura')
        assert ['pw-play', '--volume=0.6', str(RAIZ / 'src/rootfs/desktop/sonido-captura.wav')] in cmds
        correr('sonar', '../../etc/passwd', ok=False)
        correr('poner', 'sonidos', 'no')
        assert not any(c[0] == 'pw-play' for c in correr('sonar', 'captura')[1])
        # Modo juego: todo quieto y avisos en no molestar; al salir vuelve al modo guardado.
        (runtime / 'ultra-efectos').write_text('vivo\n')
        _, cmds = correr('juego', 'entrar')
        lua = next(c for c in cmds if c[:2] == ['hyprctl', 'eval'])[2]
        assert 'animations={enabled=false}' in lua and 'blur={enabled=false' in lua and 'leaf="borderangle",enabled=false' in lua
        assert ['makoctl', 'mode', '-a', 'do-not-disturb'] in cmds and (runtime / 'ultra-modo-juego').exists()
        assert ['ultra-fondo-vivo', 'pausar'] in cmds
        assert (runtime / 'ultra-efectos').read_text().strip() == 'vivo'
        _, cmds = correr('juego', 'salir')
        lua = next(c for c in cmds if c[:2] == ['hyprctl', 'eval'])[2]
        assert 'animations={enabled=true}' in lua and ['makoctl', 'mode', '-r', 'do-not-disturb'] in cmds
        assert ['ultra-fondo-vivo', 'seguir'] in cmds
        assert not (runtime / 'ultra-modo-juego').exists()
        correr('poner', 'modojuego', 'no')
        assert not any(c[:2] == ['hyprctl', 'eval'] for c in correr('juego', 'entrar')[1])
        correr('juego', 'otro', ok=False)
        # Restablecer: borra solo lo propio y vuelve al tema del sistema.
        _, cmds = correr('restablecer')
        assert not (home / '.config/ultra/personal.json').exists() and not (tema / 'waybar.css').exists()
        assert ['waybar', '-c', str(RAIZ / 'src/rootfs/desktop/waybar.jsonc'), '-s', str(RAIZ / 'src/rootfs/desktop/waybar.css')] in cmds, cmds
        assert not (tema / 'waybar.jsonc').exists()
        assert 'selection-background=5ad8ff' in foot.read_text() and 'cursor=000000 5ad8ff' in foot.read_text()
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
