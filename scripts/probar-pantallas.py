#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch, Mock

SOURCE = Path(sys.argv.pop(1)).resolve()
loader = importlib.machinery.SourceFileLoader('screens', str(SOURCE))
spec = importlib.util.spec_from_loader(loader.name, loader)
app = importlib.util.module_from_spec(spec)
loader.exec_module(app)

OUTPUT = dict(name='HEADLESS-1', width=1280, height=720, refreshRate=60,
              x=0, y=0, scale=1, transform=0, availableModes=['1280x720@60.00Hz'])


class Screens(unittest.TestCase):
    def test_modes_and_injection(self):
        self.assertEqual(app.supported(OUTPUT), ['1280x720@60.00000'])
        base = app.snapshot(OUTPUT)
        for key, value in [('output', 'DP-1"});os.execute("bad")'), ('mode', '90000x90000@nan'),
                           ('scale', float('nan')), ('scale', 0), ('scale', 10),
                           ('position', 'auto;bad'), ('transform', 8)]:
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                app.valid(dict(base, **{key: value}))

    def test_readback(self):
        expected = app.snapshot(OUTPUT)
        self.assertTrue(app.matches(expected, OUTPUT))
        self.assertFalse(app.matches(expected, dict(OUTPUT, width=1920)))
        self.assertFalse(app.matches(expected, dict(OUTPUT, scale=1.25)))
        self.assertFalse(app.matches(expected, dict(OUTPUT, refreshRate=75)))

    def test_profiles(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'profile.json'
            self.assertEqual(app.profiles(path), {})
            app.atomic_json(path, dict(version=1, outputs={'HEADLESS-1': app.snapshot(OUTPUT)}))
            self.assertEqual(set(app.profiles(path)), {'HEADLESS-1'})
            self.assertEqual(path.stat().st_mode & 0o777, 0o600)
            for data in [[], dict(version=2, outputs={}), dict(version=1, outputs={'bad':app.snapshot(OUTPUT)})]:
                app.atomic_json(path, data)
                with self.assertRaises(ValueError): app.profiles(path)

    def test_confirm_cancel_and_failed_readback(self):
        for choice, actual, raises in [(0, dict(OUTPUT, scale=1.25), False),
                                       (1, dict(OUTPUT, scale=1.25), False), (0, OUTPUT, True)]:
            with self.subTest(choice=choice, actual=actual), tempfile.TemporaryDirectory() as directory:
                runtime = Path(directory)
                profile = runtime / 'profile.json'
                old = app.snapshot(OUTPUT)
                new = dict(old, scale=1.25)
                watcher = Mock()
                watcher.poll.return_value = 0
                with patch.object(app, 'apply') as apply, patch.object(app, 'monitors', return_value=[actual]), \
                     patch.object(app, 'choose', return_value=choice), patch.object(app.subprocess, 'Popen', return_value=watcher), \
                     patch.object(app.time, 'sleep'):
                    if raises:
                        with self.assertRaises(ValueError): app.change(old, new, runtime, profile)
                    else:
                        self.assertEqual(app.change(old, new, runtime, profile), choice == 0)
                    self.assertEqual(apply.call_args_list[-1].args[0], new if choice == 0 and not raises else old)
                    self.assertEqual(profile.exists(), choice == 0 and not raises)
                    self.assertFalse(list(runtime.glob('cambio-*.json')))

    def test_save_preserves_other_monitor(self):
        with tempfile.TemporaryDirectory() as directory:
            runtime = Path(directory)
            profile = runtime/'profile.json'
            old = app.snapshot(OUTPUT)
            other = dict(old, output='DP-2', position='1280x0')
            app.atomic_json(profile, dict(version=1, outputs={'DP-2':other}))
            watcher = Mock()
            watcher.poll.return_value = 0
            with patch.object(app, 'apply'), patch.object(app, 'monitors', return_value=[dict(OUTPUT,scale=1.25)]), \
                 patch.object(app, 'choose', return_value=0), patch.object(app.subprocess,'Popen',return_value=watcher):
                self.assertTrue(app.change(old,dict(old,scale=1.25),runtime,profile))
            saved=app.profiles(profile)
            self.assertEqual(set(saved),{'HEADLESS-1','DP-2'})
            self.assertEqual(saved['DP-2'],other)

    def test_watchdog_survives_killed_parent(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'bin').mkdir()
            (root/'runtime').mkdir()
            (root/'state.json').write_text(json.dumps(OUTPUT))
            hyprctl = root/'bin/hyprctl'
            hyprctl.write_text('''#!/usr/bin/python
import os,json,re,sys
from pathlib import Path
root=Path(os.environ['CASE_ROOT']); file=root/'state.json'
data=json.loads(file.read_text())
if sys.argv[1]=='-j': print(json.dumps([data]))
else:
    match=re.search(r'scale=([0-9.]+)',sys.argv[2]); data['scale']=float(match[1])
    file.write_text(json.dumps(data)); print('ok')
''')
            fuzzel = root/'bin/fuzzel'
            fuzzel.write_text('''#!/usr/bin/python
import os,sys,time
from pathlib import Path
options=sys.stdin.read().splitlines()
if any('se ve bien' in arg.lower() for arg in sys.argv):
    Path(os.environ['CASE_ROOT'],'confirmando').touch(); time.sleep(60)
else: print('1.25x' if '1.25x' in options else options[0])
''')
            hyprctl.chmod(0o755); fuzzel.chmod(0o755)
            environment = dict(os.environ, PATH=str(root/'bin')+':/usr/bin', HOME=str(root),
                XDG_RUNTIME_DIR=str(root/'runtime'), XDG_CONFIG_HOME=str(root/'config'),
                HYPRLAND_INSTANCE_SIGNATURE='prueba', CASE_ROOT=str(root))
            parent = subprocess.Popen([sys.executable, str(SOURCE)], env=environment,
                                      stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
            try:
                end = time.monotonic()+8
                while not (root/'confirmando').exists() and time.monotonic()<end:
                    if parent.poll() is not None: break
                    time.sleep(.1)
                self.assertTrue((root/'confirmando').exists(), parent.poll())
                self.assertEqual(json.loads((root/'state.json').read_text())['scale'], 1.25)
                os.killpg(parent.pid, signal.SIGKILL)
                parent.wait(timeout=3)
                end = time.monotonic()+24
                while time.monotonic()<end:
                    if (json.loads((root/'state.json').read_text())['scale']==1
                        and not list((root/'runtime/ultra-pantallas').glob('cambio-*.json'))): break
                    time.sleep(.1)
                self.assertEqual(json.loads((root/'state.json').read_text())['scale'], 1)
                self.assertFalse(list((root/'runtime/ultra-pantallas').glob('cambio-*.json')))
                self.assertFalse((root/'config/ultra/pantalla.json').exists())
            finally:
                if parent.poll() is None: os.killpg(parent.pid, signal.SIGKILL)
                parent.communicate(timeout=3)


if __name__ == '__main__': unittest.main(verbosity=2)
