#!/usr/bin/env python3
"""Writable launcher boundaries, without accounts or real games."""
import hashlib
import importlib.machinery
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
loader = importlib.machinery.SourceFileLoader('image_launcher', str(ROOT/'src/rootfs/bin/ultra-heronite-appimage'))
spec = importlib.util.spec_from_loader(loader.name, loader)
app = importlib.util.module_from_spec(spec); loader.exec_module(app)


class Launcher(unittest.TestCase):
    def test_initial_seed_verified_and_self_update_preserved(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            source = root/'official'; source.write_bytes(b'official test seed')
            with patch.object(app, 'SEED', source), patch.object(app, 'SHA256', hashlib.sha256(source.read_bytes()).hexdigest()):
                target = app.seed(root/'app')
                self.assertEqual(target.read_bytes(), source.read_bytes())
                target.write_bytes(b'new self-updated version')
                self.assertEqual(app.seed(root/'app').read_bytes(), b'new self-updated version')

    def test_bad_seed_never_published(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve(); source = root/'bad'; source.write_bytes(b'bad')
            with patch.object(app, 'SEED', source):
                with self.assertRaises(ValueError): app.seed(root/'app')
            self.assertFalse((root/'app/Heronite.AppImage').exists())
            self.assertEqual(list((root/'app').glob('.semilla-*')), [])

    def test_appimage_symlink_rejected_without_changing_target(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve(); directory = root/'app'; directory.mkdir()
            other = root/'other'; other.write_text('preserve')
            (directory/'Heronite.AppImage').symlink_to(other)
            with self.assertRaises(ValueError): app.seed(directory)
            self.assertEqual(other.read_text(), 'preserve')


if __name__ == '__main__': unittest.main(verbosity=2)
