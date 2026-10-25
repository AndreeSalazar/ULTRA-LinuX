#!/usr/bin/env python3
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

root=Path(__file__).resolve().parents[1]
loader=importlib.machinery.SourceFileLoader('hud_heronite',str(root/'src/rootfs/bin/ultra-hud-heronite'))
spec=importlib.util.spec_from_loader(loader.name,loader)
app=importlib.util.module_from_spec(spec);loader.exec_module(app)


@unittest.skipUnless(os.name=='posix','banco Linux: permisos y enlaces reales')
class Hud(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.home=Path(self.tmp.name).resolve()
        self.global_=self.home/'.local/share/heronite/config.json';self.global_.parent.mkdir(parents=True)
        self.doc={'defaultSettings':{'showMangohud':False,'heroniteFpsLimit':'60'},'version':'v0'}
        self.global_.write_text(json.dumps(self.doc))
        self.game=self.global_.parent/'GamesConfig/1423049311.json';self.game.parent.mkdir()
        self.gdoc={'1423049311':{'showMangohud':False,'winePrefix':'/same/prefix','wineVersion':{'bin':'/same/proton'}}}
        self.game.write_text(json.dumps(self.gdoc))
        self.patch=patch.object(app,'running',return_value=False);self.patch.start()
    def tearDown(self):self.patch.stop();self.tmp.cleanup()
    def test_enables_native_hud_but_preserves_limits_prefix_and_runner(self):
        out=app.synchronize(self.home,initial=True)
        self.assertTrue(out['activado']);self.assertEqual(out['configuraciones_actualizadas'],2)
        settings=json.loads(self.game.read_text())['1423049311']
        self.assertTrue(settings.pop('showMangohud'))
        expected=dict(self.gdoc['1423049311']);expected.pop('showMangohud')
        self.assertEqual(settings,expected)
        self.assertEqual(json.loads(self.global_.read_text())['defaultSettings']['heroniteFpsLimit'],'60')
        before=self.game.stat().st_mtime_ns
        self.assertEqual(app.synchronize(self.home,initial=True)['configuraciones_actualizadas'],0)
        self.assertEqual(before,self.game.stat().st_mtime_ns)
    def test_explicit_disable_survives_initialization(self):
        config=self.home/'.config/ultra';config.mkdir(parents=True)
        (config/'hud-desactivado').touch()
        app.synchronize(self.home,initial=False)
        self.assertFalse(app.synchronize(self.home,initial=True)['activado'])
        self.assertFalse(json.loads(self.game.read_text())['1423049311']['showMangohud'])
    def test_malformed_config_causes_no_partial_game_write(self):
        self.game.write_text('{bad')
        before=self.global_.read_bytes()
        with self.assertRaises(ValueError):app.synchronize(self.home)
        self.assertEqual(self.global_.read_bytes(),before)
    def test_running_ui_defers_native_writes(self):
        before=self.global_.read_bytes()
        with patch.object(app,'running',return_value=True):
            self.assertIn('pendiente',app.synchronize(self.home))
        self.assertEqual(self.global_.read_bytes(),before)
    def test_symlink_rejected(self):
        self.game.unlink();self.game.symlink_to(self.global_)
        with self.assertRaises(ValueError):app.synchronize(self.home)


if __name__=='__main__':unittest.main(verbosity=2)
