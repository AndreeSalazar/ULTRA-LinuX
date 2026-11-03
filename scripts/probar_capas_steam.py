#!/usr/bin/env python3
"""Retiro de la capa Steam mas antigua sin montajes ni discos reales."""
import importlib.machinery
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SOURCE = Path(__file__).resolve().parents[1]/'src/rootfs/bin/ultra-bibliotecas.py'
loader = importlib.machinery.SourceFileLoader('bibliotecas', str(SOURCE))
spec = importlib.util.spec_from_loader(loader.name, loader)
mod = importlib.util.module_from_spec(spec); loader.exec_module(mod)

RUNTIME = '"AppState"\n{\n\t"appid"\t"1493710"\n\t"name"\t"Proton Experimental"\n}\n'
GAME = '"AppState"\n{\n\t"appid"\t"230410"\n\t"name"\t"Warframe"\n}\n'


class Capas(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.parent = Path(self.temp.name).resolve()
        self.store = self.parent/'lib'; self.store.mkdir()
        self.layers = []
        for stamp in (300, 100, 200):
            layer = self.parent/f'lib.anterior-{stamp}'
            apps = layer/'upper/steamapps'; (apps/'common/Warframe').mkdir(parents=True)
            (apps/'common/Warframe/Warframe.x64.exe').write_text('viejo')
            (apps/'appmanifest_230410.acf').write_text(GAME)
            self.layers.append(layer)
        self.oldest = self.parent/'lib.anterior-100'
        self.patch = patch.object(mod, 'mount_points', return_value=[])
        self.patch.start()
        # Linux rmtree has this protection; Windows (host test) lacks it.
        self.safe = patch.object(mod.shutil.rmtree, 'avoids_symlink_attacks', True)
        self.safe.start()

    def tearDown(self):
        self.safe.stop(); self.patch.stop(); self.temp.cleanup()

    def test_only_the_oldest_stale_layer_goes(self):
        self.assertEqual(mod.prune_oldest_generation(self.store), self.oldest)
        self.assertFalse(self.oldest.exists())
        self.assertEqual(sorted(p.name for p in self.parent.glob('lib.anterior-*')), ['lib.anterior-200', 'lib.anterior-300'])
        self.assertTrue(self.store.is_dir())

    def test_layer_with_saves_or_cache_is_kept(self):
        for name in ('compatdata', 'shadercache'):
            (self.oldest/'upper/steamapps'/name/'1091500').mkdir(parents=True)
            with self.assertRaisesRegex(ValueError, name): mod.prune_oldest_generation(self.store)
            self.assertTrue(self.oldest.exists())
            import shutil; shutil.rmtree(self.oldest/'upper/steamapps'/name)

    def test_layer_with_proton_runtime_is_kept(self):
        (self.oldest/'upper/steamapps/appmanifest_1493710.acf').write_text(RUNTIME)
        with self.assertRaisesRegex(ValueError, 'Proton'): mod.prune_oldest_generation(self.store)
        self.assertTrue(self.oldest.exists())

    def test_mounted_layer_is_kept(self):
        with patch.object(mod, 'mount_points', return_value=[self.oldest/'upper']):
            with self.assertRaisesRegex(ValueError, 'montada'): mod.prune_oldest_generation(self.store)
        self.assertTrue(self.oldest.exists())

    def test_unexpected_names_never_deleted(self):
        for layer in self.layers: layer.rename(layer.with_name(layer.name.replace('anterior-', 'anterior-x')))
        with self.assertRaisesRegex(ValueError, 'inesperados'): mod.prune_oldest_generation(self.store)


if __name__ == '__main__': unittest.main(verbosity=2)
