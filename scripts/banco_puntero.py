#!/usr/bin/python
# SPDX-License-Identifier: GPL-2.0-only
"""Hyprland real: politica por perfil y ventanas XWayland ficticias, sin juegos."""
import ctypes as C
import importlib.machinery
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
adapter = importlib.machinery.SourceFileLoader('puntero_red_banco','/usr/local/bin/ultra-puntero-red').load_module()
xx,fix,dd,hint_type,callback_keep = adapter.connect()
class Color(C.Structure):
    _fields_=[('pixel',C.c_ulong),('red',C.c_ushort),('green',C.c_ushort),('blue',C.c_ushort),('flags',C.c_char),('pad',C.c_char)]
x.XCreateBitmapFromData.argtypes=[C.c_void_p,C.c_ulong,C.c_char_p,C.c_uint,C.c_uint]
x.XCreateBitmapFromData.restype=C.c_ulong
x.XCreatePixmapCursor.argtypes=[C.c_void_p,C.c_ulong,C.c_ulong,C.POINTER(Color),C.POINTER(Color),C.c_uint,C.c_uint]
x.XCreatePixmapCursor.restype=C.c_ulong
x.XDefineCursor.argtypes=[C.c_void_p,C.c_ulong,C.c_ulong]
x.XFreePixmap.argtypes=[C.c_void_p,C.c_ulong]
x.XFreeCursor.argtypes=[C.c_void_p,C.c_ulong]
def visible():
    image=fix.XFixesGetCursorImage(dd)
    assert image
    try:
        v=image.contents
        return any((v.pixels[i]>>24)&255 for i in range(v.width*v.height))
    finally:xx.XFree(image)
def transparent_fixture(window,repair):
    pix=x.XCreateBitmapFromData(d,x.XDefaultRootWindow(d),b'\x00',1,1)
    color=Color();cursor=x.XCreatePixmapCursor(d,pix,pix,C.byref(color),C.byref(color),0,0)
    assert cursor
    x.XDefineCursor(d,window,cursor);x.XFlush(d)
    try:
        if repair:
            deadline=time.monotonic()+5
            while time.monotonic()<deadline and not visible():time.sleep(.05)
            assert visible(),'cursor transparente RED no recuperado'
            print('PUNTERO-TRANSPARENTE-REPARADO-OK',flush=True)
        else:
            time.sleep(.4)
            assert not visible(),'el reparador cambio el cursor del juego'
            print('PUNTERO-JUEGO-NO-ALTERADO-OK',flush=True)
    finally:
        x.XFreeCursor(d,cursor);x.XFreePixmap(d,pix)
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
        if not floating:
            move = 'hl.dispatch(hl.dsp.cursor.move({x=' + str(int(app['at'][0]+app['size'][0]/2)) + ',y=' + str(int(app['at'][1]+app['size'][1]/2)) + '}))'
            ctl('eval',move);time.sleep(.1)
        transparent_fixture(w,floating)
        print('PUNTERO-XWAYLAND-' + ('LAUNCHER' if floating else 'JUEGO') + '-OK', flush=True)
        x.XDestroyWindow(d, w); x.XFlush(d); windows.remove(w)
    print('PUNTERO-COMPATIBLE-OK', flush=True)
finally:
    for w in windows:
        x.XDestroyWindow(d, w)
    x.XFlush(d); x.XCloseDisplay(d)
    xx.XCloseDisplay(dd)
    ctl('eval', 'hl.config({cursor={no_hardware_cursors=' + str(old) + '}})')
print('PUNTERO-RC-0', flush=True)
