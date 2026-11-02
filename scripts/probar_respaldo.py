#!/usr/bin/env python3
"""ultra-respaldo on a disposable home: selection, archive, rotation, restore."""
import importlib.machinery
import importlib.util
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch

SOURCE = Path(__file__).resolve().parents[1]/'src/rootfs/bin/ultra-respaldo'
loader = importlib.machinery.SourceFileLoader('ultra_respaldo', str(SOURCE))
spec = importlib.util.spec_from_loader(loader.name, loader)
app = importlib.util.module_from_spec(spec); loader.exec_module(app)

TAR = shutil.which('tar')
GNU = bool(TAR) and 'GNU tar' in subprocess.run([TAR, '--version'], capture_output=True, text=True).stdout


class Respaldo(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.home = Path(self.temp.name)/'home'
        files = {
            '.config/hypr/ajustes.conf': 'tema',
            '.config/BraveSoftware/Default/Cache/data_0': 'x'*100,
            '.config/BraveSoftware/Default/Bookmarks': '{}',
            '.local/share/keyrings/login.keyring': 'secreto',
            '.local/share/ultra/apps/heronite/Heronite.AppImage': 'binario',
            '.local/share/ultra/portapapeles/pin.json': '[]',
            '.local/share/heronite/config.json': '{}',
            '.local/share/heronite/images/arte.png': 'png',
            '.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/save.dat': 'partida',
            '.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/AppData/Local/NVIDIA/DXCache/x.bin': 'cache',
            '.local/share/Steam/steamapps/common/Juego/enorme.pak': 'no',
            'Descargas/video.mkv': 'no',
            'Documentos/notas.txt': 'hola',
        }
        for rel, text in files.items():
            path = self.home/rel; path.parent.mkdir(parents=True, exist_ok=True); path.write_text(text)
        self.local = self.home/'.local/share/ultra/respaldos'
        self.patches = [patch.object(app, 'HOME', self.home), patch.object(app, 'LOCAL', self.local)]
        if GNU: self.patches.append(patch.object(app, 'TAR', TAR))
        for p in self.patches: p.start()

    def tearDown(self):
        for p in reversed(self.patches): p.stop()
        self.temp.cleanup()

    def selected(self):
        files, state = app.collect()
        return {p.relative_to(self.home).as_posix() for p in files}, state

    def test_selection_keeps_settings_and_saves_without_caches_or_games(self):
        chosen, _ = self.selected()
        for wanted in ('.config/hypr/ajustes.conf', '.config/BraveSoftware/Default/Bookmarks',
                       '.local/share/keyrings/login.keyring', '.local/share/ultra/portapapeles/pin.json',
                       '.local/share/heronite/config.json', 'Documentos/notas.txt',
                       '.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/save.dat'):
            self.assertIn(wanted, chosen)
        for unwanted in ('Cache/data_0', 'Heronite.AppImage', 'images/arte.png', 'DXCache', 'enorme.pak', 'video.mkv'):
            self.assertFalse(any(unwanted in c for c in chosen), unwanted)

    def test_size_limits_skip_large_files_and_budget(self):
        with patch.object(app, 'FILE_LIMIT', 3):
            chosen, state = self.selected()
        self.assertNotIn('.local/share/keyrings/login.keyring', chosen)
        self.assertGreater(state['omitidos'], 0)
        with patch.object(app, 'BUDGET', 0):
            chosen, state = self.selected()
        self.assertEqual(chosen, set())

    @unittest.skipUnless(GNU, 'requiere GNU tar')
    def test_archive_rotation_and_restore_into_new_folder(self):
        stamps = iter(f'20261102-00000{i}' for i in range(9))
        with patch.object(app.time, 'strftime', lambda fmt: next(stamps)):
            for _ in range(3):
                final, manifest = app.create(self.local, keep=2)
            kept = app.backups(self.local)
            self.assertEqual([p.name for p in kept], [p.name for p in kept][-2:])
            self.assertEqual(len(kept), 2)
            self.assertEqual(len(list(self.local.glob('*.json'))), 2)
            self.assertEqual(list(self.local.glob('.parcial-*')), [])
            self.assertGreaterEqual(manifest['archivos'], 7)
            (self.home/'Documentos/notas.txt').write_text('cambiado')
            target = app.restore(final)
        self.assertEqual((target/'Documentos/notas.txt').read_text(), 'hola')
        self.assertEqual((self.home/'Documentos/notas.txt').read_text(), 'cambiado', 'live file untouched')
        self.assertTrue((target/'.local/share/keyrings/login.keyring').is_file())

    def test_restore_rejects_foreign_files(self):
        other = self.home/'otro.tar.gz'; other.write_text('x')
        with self.assertRaises(ValueError): app.restore(other)


if __name__ == '__main__': unittest.main(verbosity=2)
