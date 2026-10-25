#!/usr/bin/env python3
"""Adapter boundaries: no account, network, real game or prefix."""
import importlib.machinery
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
loader = importlib.machinery.SourceFileLoader('heronite_adapter', str(ROOT/'src/rootfs/bin/ultra-heronite'))
spec = importlib.util.spec_from_loader(loader.name, loader)
app = importlib.util.module_from_spec(spec); loader.exec_module(app)


class Prepare(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name).resolve()
        self.data = self.root/'data'; self.data.mkdir()
        self.config = self.data/'config.json'
        self.patch = patch.object(app, 'DATA', self.data); self.patch.start()

    def tearDown(self):
        self.patch.stop(); self.tmp.cleanup()

    def test_only_obsolete_bundled_tools_are_changed_not_prefix_or_custom_runner(self):
        original = {'defaultSettings': {'altGogdlBin': '/opt/heroic/resources/gogdl',
                    'altLegendaryBin': '/my/custom/legendary', 'winePrefix': '/same/prefix',
                    'wineVersion': {'bin': '/same/proton'}}}
        self.config.write_text(json.dumps(original))
        with patch.object(app.subprocess, 'run') as run:
            app.prepare()
        self.assertEqual(run.call_args.args[0], [app.BIN, '--help'])
        new = json.loads(self.config.read_text())['defaultSettings']
        self.assertNotIn('/opt/heroic', new['altGogdlBin'])
        for key in ('altLegendaryBin', 'winePrefix', 'wineVersion'):
            self.assertEqual(new[key], original['defaultSettings'][key])

    def test_no_write_when_config_is_current(self):
        self.config.write_text('{"defaultSettings":{"winePrefix":"/same/prefix"}}')
        before = self.config.read_bytes(), self.config.stat().st_mtime_ns
        with patch.object(app.subprocess, 'run'):
            app.prepare()
        self.assertEqual(before, (self.config.read_bytes(), self.config.stat().st_mtime_ns))

    def test_symlink_data_rejected_before_native_migration(self):
        link = self.root/'link'; link.symlink_to(self.data, target_is_directory=True)
        with patch.object(app, 'DATA', link), patch.object(app.subprocess, 'run') as run:
            with self.assertRaises(ValueError): app.prepare()
        run.assert_not_called()

    def test_bad_config_preserved(self):
        self.config.write_text('{broken')
        with patch.object(app.subprocess, 'run'):
            with self.assertRaises(ValueError): app.prepare()
        self.assertEqual(self.config.read_text(), '{broken')


if __name__ == '__main__': unittest.main(verbosity=2)
