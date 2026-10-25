#!/usr/bin/python
# SPDX-License-Identifier: GPL-2.0-only
"""Hyprland real: politica por perfil y ventanas XWayland ficticias, sin juegos."""
import ctypes as C
import json
import os
from pathlib import Path
import subprocess
import time

assert os.geteuid() == 1000


def ctl(*args):
    result = subprocess.run(['hyprctl', *args], text=True, capture_output=True, timeout=10)
    assert result.returncode == 0, result.stderr + result.stdout
    assert not result.stdout.lower().startswith('error'), result.stdout
    return result.stdout


# Ejecutar la politica del archivo entregado con hl simulado en el mismo Lua.
# No recargar atajos ni callbacks del escritorio durante esta comprobacion.
prefix = Path('/etc/ultra/hyprland.lua').read_text().split('-- Solo el lanzador:')[0]
for mode, expected in [('nvidia', 1), ('software', 2)]:
    lua = ('local cfg; local hl={monitor=function() end, config=function(c) cfg=c end}; '
           'local os={getenv=function(k) if k=="ULTRA_RENDER_MODE" then return "' + mode + '" end end}; '
           + prefix + '\nassert(cfg.cursor.no_hardware_cursors==' + str(expected) + '); '
           'assert(cfg.cursor.hide_on_key_press==false and cfg.cursor.hide_on_touch==false); '
           'assert(cfg.cursor.inactive_timeout==0); return "PUNTERO-POLITICA-OK"')
    # eval confirma con "ok"; no devuelve el valor retornado por Lua.
    assert ctl('eval', lua).strip() == 'ok'
print('PUNTERO-PERFILES-OK', flush=True)

x = C.CDLL('libX11.so.6')
x.XOpenDisplay.argtypes = [C.c_char_p]; x.XOpenDisplay.restype = C.c_void_p
x.XDefaultRootWindow.argtypes = [C.c_void_p]; x.XDefaultRootWindow.restype = C.c_ulong
x.XCreateSimpleWindow.argtypes = [C.c_void_p, C.c_ulong, C.c_int, C.c_int, C.c_uint, C.c_uint, C.c_uint, C.c_ulong, C.c_ulong]
x.XCreateSimpleWindow.restype = C.c_ulong


class Hint(C.Structure):
    _fields_ = [('res_name', C.c_char_p), ('res_class', C.c_char_p)]


x.XSetClassHint.argtypes = [C.c_void_p, C.c_ulong, C.POINTER(Hint)]
x.XStoreName.argtypes = [C.c_void_p, C.c_ulong, C.c_char_p]
for name in ('XMapWindow', 'XDestroyWindow'):
    getattr(x, name).argtypes = [C.c_void_p, C.c_ulong]
x.XFlush.argtypes = [C.c_void_p]
x.XCloseDisplay.argtypes = [C.c_void_p]
x.XSelectInput.argtypes = [C.c_void_p, C.c_ulong, C.c_long]
x.XPending.argtypes = [C.c_void_p]
x.XNextEvent.argtypes = [C.c_void_p, C.c_void_p]
d = x.XOpenDisplay(None)
assert d, 'XWayland no disponible'
windows = []
old = json.loads(ctl('-j', 'getoption', 'cursor:no_hardware_cursors'))['int']
try:
    ctl('eval', 'hl.config({cursor={no_hardware_cursors=1}})')
    assert json.loads(ctl('-j', 'getoption', 'cursor:no_hardware_cursors'))['int'] == 1
    assert not ctl('configerrors').strip(), 'configuracion Hyprland invalida'
    for classname, floating in [('REDlauncher.exe', True), ('Cyberpunk2077.exe', False)]:
        w = x.XCreateSimpleWindow(d, x.XDefaultRootWindow(d), 0, 0, 480, 280, 0, 0, 0x102030)
        assert w
        windows.append(w)
        raw = classname.encode()
        hint = Hint(raw, raw)
        x.XSetClassHint(d, w, C.byref(hint))
        x.XStoreName(d, w, b'ULTRA prueba XWayland sin juego')
        x.XSelectInput(d, w, 1 << 2)  # ButtonPressMask: raton virtual del banco.
        x.XMapWindow(d, w); x.XFlush(d)
        app = None
        for _ in range(40):
            app = next((a for a in json.loads(ctl('-j', 'clients')) if a.get('class') == classname), None)
            if app:
                break
            time.sleep(.1)
        assert app and app['xwayland'] and app['floating'] == floating, app
        if floating:
            monitor = next(m for m in json.loads(ctl('-j', 'monitors')) if m['id'] == app['monitor'])
            # Banco con una salida sin rotacion, escala 1.
            assert monitor['scale'] == 1 and monitor['transform'] == 0
            center = [app['at'][i] + app['size'][i] / 2 for i in range(2)]
            target = [monitor['x'] + monitor['width'] / 2, monitor['y'] + monitor['height'] / 2]
            assert all(abs(a-b) <= 80 for a, b in zip(center, target)), (center, target)
            move = 'hl.dispatch(hl.dsp.cursor.move({x=' + str(int(center[0])) + ',y=' + str(int(center[1])) + '}))'
            assert ctl('eval', move).strip() == 'ok'
            print('PUNTERO-ESPERA-CLICK', flush=True)
            clicked = False
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline:
                if x.XPending(d):
                    event = (C.c_long * 24)()
                    x.XNextEvent(d, C.byref(event))
                    if C.cast(event, C.POINTER(C.c_int))[0] == 4:
                        clicked = True
                        break
                time.sleep(.05)
            assert clicked, 'no llego ButtonPress a la ventana XWayland'
            print('PUNTERO-CLICK-XWAYLAND-OK', flush=True)
        print('PUNTERO-XWAYLAND-' + ('LAUNCHER' if floating else 'JUEGO') + '-OK', flush=True)
        x.XDestroyWindow(d, w); x.XFlush(d); windows.remove(w)
    print('PUNTERO-COMPATIBLE-OK', flush=True)
finally:
    for w in windows:
        x.XDestroyWindow(d, w)
    x.XFlush(d); x.XCloseDisplay(d)
    ctl('eval', 'hl.config({cursor={no_hardware_cursors=' + str(old) + '}})')
print('PUNTERO-RC-0', flush=True)
