#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

base=Path(sys.argv.pop(1))
def load(name, path):
    loader=importlib.machinery.SourceFileLoader(name,str(path))
    spec=importlib.util.spec_from_loader(name,loader)
    module=importlib.util.module_from_spec(spec); loader.exec_module(module)
    return module
app=load('discos',base/'ultra-discos')
lib=load('bibliotecas',base/'ultra-bibliotecas.py')

class Discos(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory()
        self.root=Path(self.temp.name)
        self.home=self.root/'home'; self.home.mkdir()
        self.catalog=self.root/'catalog.json'
        self.item=lib.selection(dict(uuid='1234-ABCD',relative='SteamLibrary',kind='steam'))
        self.fake=Mock(HOME=self.home,CATALOG=self.catalog,read_json=lib.read_json,
                       atomic_json=lib.atomic_json,user_config=Mock(return_value=[self.item]))
        self.catalog.write_text(json.dumps(dict(volumes=[dict(status='lectura')],folders=[])))
    def tearDown(self): self.temp.cleanup()
    def ready(self):
        self.catalog.write_text(json.dumps(dict(volumes=[],folders=[dict(self.item,status='activa',path='/prepared')])) )
    def test_pending_is_not_reported_ready(self):
        with patch.object(app,'steam_running',return_value=False):
            state,_=app.tick(self.fake)
        self.assertIn('pendiente',state['folders'][0]['status'])
        self.fake.register_steam.assert_not_called()
    def test_steam_open_waits_then_registers_once(self):
        self.ready()
        with patch.object(app,'steam_running',return_value=True):
            state,key=app.tick(self.fake)
        self.assertIn('Esperando',state['steam']); self.assertIsNone(key)
        self.fake.register_steam.assert_not_called()
        with patch.object(app,'steam_running',return_value=False):
            _,key=app.tick(self.fake,key)
            app.tick(self.fake,key)
        self.fake.register_steam.assert_called_once()
    def test_removed_folder_is_not_registered(self):
        self.ready(); self.fake.user_config.return_value=[]
        state,_=app.tick(self.fake)
        self.assertEqual(state['folders'],[])
        self.fake.register_steam.assert_not_called()
    def test_automatic_final_state_private(self):
        app.automatic(self.fake,duration=0)
        status=self.home/'.cache/ultra/discos/automatico.json'
        data=json.loads(status.read_text())
        self.assertFalse(data['running']); self.assertEqual(data['remaining'],0)
        self.assertEqual(status.stat().st_mode & 0o777,0o600)
    def test_registration_failure_visible(self):
        self.ready(); self.fake.register_steam.side_effect=ValueError('metadatos invalidos')
        with patch.object(app,'steam_running',return_value=False): app.automatic(self.fake,duration=0)
        state=json.loads((self.home/'.cache/ultra/discos/automatico.json').read_text())
        self.assertIn('metadatos invalidos',state['error'])
    def test_sixty_second_progress(self):
        states=[]
        self.fake.atomic_json=Mock(side_effect=lambda path,state:states.append(dict(state)))
        with patch.object(app.time,'monotonic',side_effect=[0,0,30,60]), patch.object(app.time,'sleep'):
            app.automatic(self.fake)
        self.assertEqual([s['remaining'] for s in states],[60,30,0])
        self.assertEqual([s['running'] for s in states],[True,True,False])
    def test_steam_launch_race_not_reported_ready(self):
        self.ready(); self.fake.register_steam.return_value=False
        with patch.object(app,'steam_running',return_value=False): state,key=app.tick(self.fake)
        self.assertIsNone(key); self.assertIn('Cierra Steam',state['steam'])
    def test_missing_storage_has_no_write(self):
        with patch.dict(os.environ,HOME='/temporary'):
            text=app.lines(self.fake)
        self.assertIn('almacenamiento no disponible',text[0])
        self.assertFalse((self.home/'.cache').exists())
    def test_navigation_remembers_folder_after_cancel(self):
        volume=self.root/'disk'; folder=volume/'Games'; folder.mkdir(parents=True)
        catalog=dict(volumes=[dict(uuid='1234-ABCD',label='Datos',path=str(volume),status='lectura')])
        config=self.home/'.config/ultra/bibliotecas.json'
        replies=iter([catalog['volumes'][0],folder,None])
        with patch.object(lib,'CONFIG',config), patch.object(lib,'choose',side_effect=lambda *args:next(replies)):
            self.assertIsNone(lib.pick_folder(catalog))
        saved=json.loads(config.with_name('navegacion.json').read_text())
        self.assertEqual(saved['relative'],'Games')
        seen=[]
        with patch.object(lib,'CONFIG',config),patch.object(lib,'choose',side_effect=lambda title,rows:seen.append(rows)):
            lib.pick_folder(catalog)
        self.assertEqual(seen[0][0][0],'Continuar en Games')

if __name__=='__main__': unittest.main(verbosity=2)
