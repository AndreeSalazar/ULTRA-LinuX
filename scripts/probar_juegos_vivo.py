#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Disposable /proc and game metadata, never accounts or real games."""
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

BASE = Path(__file__).resolve().parents[1]/'src/rootfs/bin'


def load(name):
    loader = importlib.machinery.SourceFileLoader(name, str(BASE/name))
    spec = importlib.util.spec_from_loader(name, loader)
    mod = importlib.util.module_from_spec(spec)
    loader.exec_module(mod)
    return mod


app = load('ultra-juegos-vivo')
lib = load('ultra-bibliotecas.py')
disks = load('ultra-discos')


class Live(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.home = self.root/'home'; self.home.mkdir()
        self.catalog = self.root/'catalog.json'; self.catalog.write_text('{"folders":[]}')
        self.targets = self.root/'targets'; self.targets.mkdir()
        self.patches = [patch.object(app,'HOME',self.home),patch.object(lib,'CATALOG',self.catalog),patch.object(lib,'TARGETS',self.targets)]
        for p in self.patches: p.start()
    def tearDown(self):
        for p in self.patches: p.stop()
        self.temp.cleanup()
    def manifest(self, flags=4):
        folder = self.home/'.local/share/Steam/steamapps'
        (folder/'common/Test').mkdir(parents=True,exist_ok=True)
        path = folder/'appmanifest_22.acf'
        path.write_text('"AppState" { "appid" "22" "name" "Test" "installdir" "Test" "SizeOnDisk" "4096" "StateFlags" "'+str(flags)+'" }')
        return path
    def test_steam_install_update_remove_live(self):
        path = self.manifest()
        self.assertEqual(app.catalogue(lib)['games'][0]['state'],'instalado')
        self.manifest(flags=6)
        self.assertEqual(app.catalogue(lib)['games'][0]['state'],'instalado')
        path.unlink()
        self.assertEqual(app.catalogue(lib)['games'],[])
    def test_gog_registered_and_discovered_are_distinct(self):
        record = lib.selection(dict(uuid='ABCD-1234',relative='Games',kind='folder'))
        folder = self.targets/record['id']; folder.mkdir()
        (folder/'goggame-123.info').write_text(json.dumps(dict(gameId='123',name='GOG test')))
        self.catalog.write_text(json.dumps(dict(folders=[dict(record,status='activa',path=str(folder))])))
        game = app.catalogue(lib)['games'][0]
        self.assertEqual(game['state'],'importar en Heronite')
        store = self.home/'.config/heroic/gog_store'; store.mkdir(parents=True)
        path = store/'installed.json'
        path.write_text(json.dumps(dict(installed=[dict(appName='123',title='GOG test',install_path=str(folder),access_token='NO-EXPORTAR')],refresh_token='SECRETO')))
        before = path.read_bytes()
        games = app.catalogue(lib)
        self.assertEqual(games['games'][0]['state'],'importado')
        self.assertNotIn('SECRETO',json.dumps(games)); self.assertNotIn('NO-EXPORTAR',json.dumps(games))
        self.assertEqual(path.read_bytes(),before)
        path.write_text('{')
        self.assertTrue(app.catalogue(lib)['errors'])
    def fake_proc(self, ticks=10, start=100):
        proc = self.root/'proc'; proc.mkdir(exist_ok=True)
        (proc/'stat').write_text('cpu  10 0 10 80 0 0 0 0\n')
        (proc/'meminfo').write_text('MemTotal: 4096 kB\nMemAvailable: 2048 kB\n')
        task=proc/'42';task.mkdir(exist_ok=True)
        fields=['0']*22
        fields[0]='S';fields[11]=str(ticks);fields[12]='0';fields[19]=str(start);fields[21]='100'
        (task/'stat').write_text('42 (game with ) name) '+' '.join(fields))
        (task/'environ').write_bytes(b'SteamAppId=22\0PASSWORD=NO-EXPORTAR\0')
        (task/'smaps_rollup').write_text('Pss: 400 kB\n')
        (task/'io').write_text('read_bytes: '+str(ticks*100)+'\nwrite_bytes: 0\n')
        return proc
    def test_rates_memory_and_pid_reuse_not_fps_guesses(self):
        proc=self.fake_proc()
        with patch.object(app,'PROC',proc):
            one=app.sample(now=10)
            self.fake_proc(ticks=110)
            two=app.sample(one,now=12)
            expected=100*100/os.sysconf('SC_CLK_TCK')/2
            self.assertEqual(two['games'][0]['cpu_pct'],expected)
            self.assertEqual(two['games'][0]['pss_bytes'],400*1024)
            self.assertEqual(two['processes'][0]['io_read_Bps'],5000)
            self.assertEqual(two['ram_used_MiB'],2)
            self.assertIsNone(two['fps']);self.assertIsNone(two['games'][0]['fps'])
            self.assertNotIn('NO-EXPORTAR',json.dumps(two))
            self.fake_proc(ticks=200,start=200)
            reused=app.sample(two,now=14)
            self.assertIsNone(reused['games'][0]['cpu_pct'])
            (proc/'42/smaps_rollup').unlink()
            unavailable=app.sample(reused,now=16)
            self.assertIsNone(unavailable['games'][0]['pss_bytes'])
    def test_registration_fingerprint_changes_with_manifests(self):
        path=self.manifest()
        row=dict(id='fixture',status='activa',kind='steam',path=str(self.home/'.local/share/Steam'))
        with patch.object(disks,'snapshot',return_value=dict(folders=[row])),patch.object(disks,'steam_running',return_value=False),patch.object(lib,'register_steam',return_value=True) as register:
            _,key=disks.tick(lib)
            disks.tick(lib,key)
            self.assertEqual(register.call_count,1)
            path.write_text(path.read_text()+'\n// cambio\n')
            disks.tick(lib,key)
            self.assertEqual(register.call_count,2)
    def test_identity_allowlist(self):
        self.assertEqual(app.game_identity(b'SteamAppId=0\0TOKEN=secret\0'),(None,''))
        self.assertEqual(app.game_identity(b'HEROIC_APP_NAME=123\0HEROIC_APP_RUNNER=gog\0')[0],'gog:123')
        self.assertEqual(app.game_identity(b'ULTRA_GAME_ID=carpeta:abc\0ULTRA_GAME_NAME=Test\nTitle\0'),('carpeta:abc','Test Title'))


if __name__=='__main__': unittest.main(verbosity=2)
