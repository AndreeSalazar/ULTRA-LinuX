#!/usr/bin/env python3
"""Retiro de la capa Steam mas antigua con rescate de partidas; sin discos reales."""
import importlib.machinery
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SOURCE = Path(__file__).resolve().parents[1]/'src/rootfs/bin/ultra-bibliotecas.py'
loader = importlib.machinery.SourceFileLoader('bibliotecas', str(SOURCE))
spec = importlib.util.spec_from_loader(loader.name, loader)
mod = importlib.util.module_from_spec(spec); loader.exec_module(mod)

GAME = '"AppState"\n{\n\t"appid"\t"230410"\n\t"name"\t"Warframe"\n}\n'


class Capas(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        root = Path(self.temp.name).resolve()
        self.parent = root/'privado'; self.parent.mkdir()
        self.home = root/'home'; self.home.mkdir()
        self.store = self.parent/'lib'; self.store.mkdir()
        for stamp in (300, 100, 200):
            apps = self.parent/f'lib.anterior-{stamp}'/'upper/steamapps'
            (apps/'common/Warframe').mkdir(parents=True)
            (apps/'common/Warframe/Warframe.x64.exe').write_text('viejo')
            (apps/'appmanifest_230410.acf').write_text(GAME)
        self.oldest = self.parent/'lib.anterior-100'
        uid = os.stat(root).st_uid  # the files' real owner on this host
        self.patches = [patch.object(mod, 'mount_points', return_value=[]),
                        # Linux rmtree has this protection; the Windows host test lacks it.
                        patch.object(mod.shutil.rmtree, 'avoids_symlink_attacks', True),
                        patch.object(mod, 'RESCUE', self.home/'Juegos'/'Partidas rescatadas'),
                        patch.object(mod, 'USER_UID', uid),
                        patch.object(mod.os, 'chown', lambda *a, **k: None, create=True)]
        for p in self.patches: p.start()

    def tearDown(self):
        for p in reversed(self.patches): p.stop()
        self.temp.cleanup()

    def test_only_the_oldest_stale_layer_goes(self):
        self.assertEqual(mod.prune_oldest_generation(self.store), self.oldest)
        self.assertFalse(self.oldest.exists())
        self.assertEqual(sorted(p.name for p in self.parent.glob('lib.anterior-*')), ['lib.anterior-200', 'lib.anterior-300'])
        self.assertFalse((self.home/'Juegos').exists(), 'nothing to rescue, nothing created')

    def test_saves_are_rescued_to_home_before_removal(self):
        save = self.oldest/'upper/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/partida.dat'
        save.parent.mkdir(parents=True); save.write_text('PARTIDA')
        (self.oldest/'upper/steamapps/shadercache/1091500').mkdir(parents=True)
        mod.prune_oldest_generation(self.store)
        rescued = self.home/'Juegos/Partidas rescatadas/lib.anterior-100'
        self.assertEqual((rescued/'compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/partida.dat').read_text(), 'PARTIDA')
        self.assertIn('Nada se borro', (rescued/'LEEME.txt').read_text())
        self.assertFalse(self.oldest.exists())

    def test_existing_rescue_is_never_overwritten(self):
        (self.oldest/'upper/steamapps/compatdata/1').mkdir(parents=True)
        (self.home/'Juegos/Partidas rescatadas/lib.anterior-100/compatdata').mkdir(parents=True)
        with self.assertRaisesRegex(ValueError, 'ya existe'): mod.prune_oldest_generation(self.store)
        self.assertTrue((self.oldest/'upper/steamapps/compatdata/1').is_dir())

    def test_redirected_rescue_folder_keeps_layer(self):
        (self.oldest/'upper/steamapps/compatdata/1').mkdir(parents=True)
        with patch.object(mod.Path, 'is_symlink', lambda self: self.name == 'Juegos'):
            with self.assertRaisesRegex(ValueError, 'redirigida'): mod.prune_oldest_generation(self.store)
        self.assertTrue(self.oldest.exists())

    def test_mounted_layer_is_kept(self):
        with patch.object(mod, 'mount_points', return_value=[self.oldest/'upper']):
            with self.assertRaisesRegex(ValueError, 'montada'): mod.prune_oldest_generation(self.store)
        self.assertTrue(self.oldest.exists())

    def test_unexpected_names_never_deleted(self):
        for layer in list(self.parent.glob('lib.anterior-*')):
            layer.rename(layer.with_name(layer.name.replace('anterior-', 'anterior-x')))
        with self.assertRaisesRegex(ValueError, 'inesperados'): mod.prune_oldest_generation(self.store)


if __name__ == '__main__': unittest.main(verbosity=2)
