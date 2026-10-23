#!/usr/bin/python
# SPDX-License-Identifier: GPL-2.0-only
# Only used by probar_arranque.py in its disposable QEMU desktop/profile.
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import sys
import time

assert os.getuid() == 1000 and os.environ.get('HYPRLAND_INSTANCE_SIGNATURE')
loader = importlib.machinery.SourceFileLoader('screens_native', '/usr/local/bin/ultra-pantallas')
spec = importlib.util.spec_from_loader(loader.name, loader)
app = importlib.util.module_from_spec(spec)
loader.exec_module(app)
runtime = Path(os.environ['XDG_RUNTIME_DIR'])/'ultra-pantallas'
runtime.mkdir(mode=0o700, exist_ok=True)
profile = runtime/'perfil-prueba.json'
original = app.monitors()[0]
old = app.snapshot(original)
alternatives = [mode for mode in app.supported(original)
                if mode.split('@')[0] != old['mode'].split('@')[0]
                and int(mode.split('x')[0]) <= 1920
                and int(mode.split('x')[1].split('@')[0]) <= 1080]
# Virtio advertises 5K modes too; this small VM tests desktop controls, not
# framebuffer capacity. Exercise a bounded mode instead of exhausting scanout.
new = dict(old)
if alternatives:
    new['mode'] = alternatives[0]
else:
    new['scale'] = 1.25
try:
    app.choose = lambda *args, **kwargs: 1
    assert app.change(old, new, runtime, profile) is False
    time.sleep(.5)
    assert any(app.matches(old, output) for output in app.monitors())
    assert not profile.exists()
    print('PANTALLAS_CANCEL_OK', flush=True)
    app.choose = lambda *args, **kwargs: 0
    assert app.change(old, new, runtime, profile) is True
    assert any(app.matches(new, output) for output in app.monitors())
    assert app.profiles(profile)[new['output']] == new
    print('PANTALLAS_CONFIRM_OK '+json.dumps(new), flush=True)
    app.apply(old)
    time.sleep(.5)
    scale = dict(old, scale=1.25)
    assert app.change(old, scale, runtime, profile) is True
    assert any(app.matches(scale, output) for output in app.monitors())
    print('PANTALLAS_ESCALA_OK', flush=True)
    # Exercise the actual restore command without touching the user's real config.
    app.apply(old)
    time.sleep(.5)
    config = runtime/'config-prueba'
    os.environ['XDG_CONFIG_HOME'] = str(config)
    restore = dict(new, scale=1)
    app.atomic_json(config/'ultra/pantalla.json', dict(version=1, outputs={restore['output']:restore}))
    previous_args = sys.argv
    sys.argv = ['ultra-pantallas', 'restaurar']
    try: app.main()
    finally: sys.argv = previous_args
    assert any(app.matches(restore, output) for output in app.monitors())
    (config/'ultra/pantalla.json').unlink()
    (config/'ultra').rmdir()
    config.rmdir()
    print('PANTALLAS_RESTORE_OK', flush=True)
finally:
    app.apply(old)
    time.sleep(.5)
    profile.unlink(missing_ok=True)
assert any(app.matches(old, output) for output in app.monitors())
print('PANTALLAS_PRUEBA_OK '+json.dumps(old), flush=True)
