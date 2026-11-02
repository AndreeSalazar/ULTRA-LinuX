#!/usr/bin/env python3
"""Disposable GOG metadata and PE stub; no accounts, real games or network."""
import importlib.machinery
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SOURCE = Path(__file__).resolve().parents[1]/'src/rootfs/bin/ultra-gog-cyberpunk'
loader = importlib.machinery.SourceFileLoader('gog_cyberpunk', str(SOURCE))
spec = importlib.util.spec_from_loader(loader.name, loader)
app = importlib.util.module_from_spec(spec); loader.exec_module(app)


class Import(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name).resolve()
        self.game = self.root/'Cyberpunk2077'; self.game.mkdir()
        exe = self.game/'bin/x64/Cyberpunk2077.exe'
        exe.parent.mkdir(parents=True); exe.write_bytes(b'MZ-test-fixture')
        self.info = dict(gameId=app.APP,rootGameId=app.APP,buildId='12345',name='Cyberpunk 2077',languages=['en-US'])
        (self.game/f'goggame-{app.APP}.info').write_text(json.dumps(self.info))
        self.home = self.root/'home'; self.home.mkdir()
        self.store = self.home/'.config/heroic/gog_store'; self.store.mkdir(parents=True)
        self.file = self.store/'installed.json'
        self.patches = [patch.object(app,'HOME',self.home),patch.object(app,'GAME',self.game),
                        patch.object(app,'HERONITE_VERSION',self.root/'version-ausente'),
                        patch.object(app,'heroic_running',return_value=False)]
        for p in self.patches: p.start()
    def tearDown(self):
        for p in reversed(self.patches): p.stop()
        self.temp.cleanup()
    def test_verified_base_and_dlc_import_without_auth_or_copy(self):
        (self.game/'goggame-999.info').write_text(json.dumps(dict(gameId='999',rootGameId=app.APP)))
        self.assertIn('registrado',app.register())
        data=json.loads(self.file.read_text())['installed'][0]
        self.assertEqual(data['install_path'],str(self.game))
        self.assertEqual(data['buildId'],'12345')
        self.assertEqual(data['installedDLCs'],['999'])
        self.assertFalse((self.store/'auth.json').exists())
        self.assertFalse((self.home/'Juegos').exists())
    def test_repair_windows_path_and_preserve_other_entries_settings(self):
        other=dict(appName='999',install_path='/other',custom=True)
        self.file.write_text(json.dumps(dict(installed=[other,dict(appName=app.APP,install_path='D:\\Cyberpunk 2077',custom='keep')],extra=42)))
        before=self.file.read_bytes(); app.register()
        data=json.loads(self.file.read_text())
        self.assertEqual(data['installed'][0],other)
        self.assertEqual(data['installed'][1]['custom'],'keep')
        self.assertEqual(data['extra'],42)
        self.assertEqual(self.file.with_name('installed.json.ultra-antes-cyberpunk').read_bytes().strip(),before.strip())
    def test_idempotent_registration_does_not_rewrite(self):
        app.register(); before=(self.file.read_bytes(),self.file.stat().st_mtime_ns)
        self.assertIn('ya reconocido',app.register())
        self.assertEqual(before,(self.file.read_bytes(),self.file.stat().st_mtime_ns))
    def test_running_client_blocks_writes(self):
        with patch.object(app,'heroic_running',return_value=True),self.assertRaises(ValueError): app.register()
        self.assertFalse(self.file.exists())
    def test_running_client_can_launch_already_registered_without_writes(self):
        app.register(); before=(self.file.read_bytes(),self.file.stat().st_mtime_ns)
        with patch.object(app,'heroic_running',return_value=True):
            self.assertIn('ya reconocido',app.register())
        self.assertEqual(before,(self.file.read_bytes(),self.file.stat().st_mtime_ns))
    def test_malformed_store_untouched(self):
        self.file.write_text('{')
        with self.assertRaises(ValueError): app.register()
        self.assertEqual(self.file.read_text(),'{')
    def test_unmatched_product_rejected(self):
        self.info['rootGameId']='wrong'
        (self.game/f'goggame-{app.APP}.info').write_text(json.dumps(self.info))
        with self.assertRaises(ValueError): app.register()
        self.assertFalse(self.file.exists())
    def test_symlink_executable_or_store_rejected(self):
        exe=self.game/'bin/x64/Cyberpunk2077.exe'; exe.unlink()
        outside=self.root/'other.exe'; outside.write_bytes(b'MZ-test')
        exe.symlink_to(outside)
        with self.assertRaises(ValueError): app.register()
        self.assertFalse(self.file.exists())
    def test_non_pe_stub_rejected(self):
        (self.game/'bin/x64/Cyberpunk2077.exe').write_bytes(b'wrong')
        with self.assertRaises(ValueError): app.register()
        self.assertFalse(self.file.exists())

    def test_heronite_uses_separate_store_and_preserves_heroic(self):
        version = self.root/'heronite-version'; version.write_text('0.15.0')
        self.file.write_text('{"installed": [], "sentinel": "original"}')
        before = self.file.read_bytes()
        with patch.object(app, 'HERONITE_VERSION', version):
            self.assertIn('registrado', app.register())
            target = self.home/'.local/share/heronite/gog_store/installed.json'
            self.assertEqual(json.loads(target.read_text())['installed'][0]['appName'], app.APP)
            self.assertIn('ya reconocido', app.register())
        self.assertEqual(self.file.read_bytes(), before)

    def test_heronite_1_store_uses_upstream_executable_contract(self):
        version = self.root/'heronite-version'; version.write_text('1.0.0\n')
        with patch.object(app, 'HERONITE_VERSION', version):
            self.assertIn('registrado', app.register())
            target = self.home/'.local/share/heronite/gog_store/installed.json'
            entry = json.loads(target.read_text())['installed'][0]
            # Heronite writes executable="" itself; a different value would
            # force a rewrite on every launch.
            self.assertEqual((entry['executable'], entry['install_path']), ('', str(self.game)))
            before = (target.read_bytes(), target.stat().st_mtime_ns)
            self.assertIn('ya reconocido', app.register())
            self.assertEqual(before, (target.read_bytes(), target.stat().st_mtime_ns))

    def test_heronite_version_policy(self):
        for good in ('0.15.0', '1.0.0', '1.2', '1.10.3'):
            self.assertTrue(app.HERONITE_COMPATIBLE.fullmatch(good), good)
        for bad in ('0.14.9', '2.0.0', '1', '1.0.0-rc1', '', '1.0.0 ; x'):
            self.assertFalse(app.HERONITE_COMPATIBLE.fullmatch(bad), bad)

    def test_appimage_process_counts_as_running(self):
        self.patches[-1].stop()
        try:
            proc = self.root/'proc'; (proc/'41').mkdir(parents=True)
            fake = self.root/'heronite-extract/appimage_extracted_x/usr/bin/heronite-rs'
            fake.parent.mkdir(parents=True); fake.write_bytes(b'')
            real_path = app.Path
            class Stat:
                st_uid = 1000
            def fake_path(*args):
                return proc if args == ('/proc',) else real_path(*args)
            def resolve(self, *a, **k):
                return fake if self.name == 'exe' else real_path.resolve(self, *a, **k)
            with patch.object(app, 'Path', side_effect=fake_path), \
                 patch.object(real_path, 'stat', lambda self, *a, **k: Stat()), \
                 patch.object(real_path, 'resolve', resolve):
                self.assertTrue(app.heroic_running())
        finally:
            self.patches[-1].start()


if __name__=='__main__': unittest.main(verbosity=2)
