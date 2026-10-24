#!/usr/bin/env python3
"""Safe synthetic retention/rotation and pidfd signal tests; no game files."""
import importlib.machinery,importlib.util,json,os,subprocess,sys,tempfile,time,unittest
from pathlib import Path
from unittest.mock import patch
BASE=Path(__file__).resolve().parents[1]/'src/rootfs/bin'
def load(name):
 loader=importlib.machinery.SourceFileLoader(name,str(BASE/name));spec=importlib.util.spec_from_loader(name,loader)
 module=importlib.util.module_from_spec(spec);loader.exec_module(module);return module
apps=load('ultra-aplicaciones');space=load('ultra-espacio');lib=load('ultra-bibliotecas.py')
class Manager(unittest.TestCase):
 def test_pidfd_identity_protected_self_and_close_fixture(self):
  self.assertIn(os.getpid(),apps.protected(apps.processes()))
  p=subprocess.Popen([sys.executable,'-c','import time; time.sleep(10)'])
  try:
   time.sleep(.1);row=next(r for r in apps.processes() if r['pid']==p.pid)
   with self.assertRaises(ValueError):apps.close(dict(row,start=row['start']+1))
   self.assertIsNone(p.poll());apps.close(row);self.assertEqual(p.wait(timeout=3),-15)
  finally:
   if p.poll() is None:p.kill();p.wait()
 def test_retention_keeps_open_recent_symlink_and_games(self):
  with tempfile.TemporaryDirectory() as d:
   home=Path(d).resolve();cache=home/'.cache/ultra';cache.mkdir(parents=True)
   def old(name):
    p=cache/name;p.write_bytes(b'x'*(1024**2+1));os.utime(p,(time.time()-9*86400,)*2);return p
   remove=old('steam.log');opened=old('brave.log');recent=cache/'heroic.log';recent.write_bytes(b'x'*(1024**2+1))
   game=home/'partida.log';game.write_text('partida');(cache/'cyberpunk.log').symlink_to(game)
   with opened.open() as handle,patch.object(space,'HOME',home):self.assertEqual(space.clean(),1024**2+1)
   self.assertFalse(remove.exists());self.assertTrue(opened.exists());self.assertTrue(recent.exists());self.assertEqual(game.read_text(),'partida')
 def test_rotation_refuses_second_archive_without_deleting_first(self):
  with tempfile.TemporaryDirectory() as d,patch.object(lib,'checked_dir',lambda *a:None),patch.object(lib.shutil,'disk_usage',return_value=type('D',(),{'free':10*1024**3})()):
   p=Path(d)/'library';p.mkdir();(p/'upper').mkdir();(p/'source.json').write_text(json.dumps({'id':'fixed','version':1}));(p/'save').write_text('preservar')
   lib.rotate_changed_store(p,{'id':'fixed','version':2})
   archive=next(Path(d).glob('library.anterior-*'));p.mkdir();(p/'source.json').write_text(json.dumps({'id':'fixed','version':2}))
   with self.assertRaises(ValueError):lib.rotate_changed_store(p,{'id':'fixed','version':3})
   self.assertEqual((archive/'save').read_text(),'preservar');self.assertTrue((p/'source.json').exists())
 def test_low_space_rotation_preserves_existing_store(self):
  with tempfile.TemporaryDirectory() as d,patch.object(lib,'checked_dir',lambda *a:None),patch.object(lib.shutil,'disk_usage',return_value=type('D',(),{'free':1})()):
   p=Path(d)/'library';p.mkdir();(p/'source.json').write_text(json.dumps({'id':'fixed','version':1}))
   with self.assertRaises(ValueError):lib.rotate_changed_store(p,{'id':'fixed','version':2})
   self.assertTrue(p.exists());self.assertEqual(len(list(Path(d).iterdir())),1)
 def test_existing_large_layer_needs_space_for_renewal(self):
  with tempfile.TemporaryDirectory() as d,patch.object(lib,'checked_dir',lambda *a:None),patch.object(lib.shutil,'disk_usage',return_value=type('D',(),{'free':10*1024**3})()),patch.object(lib,'run',return_value=str(7*1024**3)+' upper'):
   p=Path(d)/'library';p.mkdir();(p/'source.json').write_text(json.dumps({'id':'fixed','version':1}))
   with self.assertRaises(ValueError):lib.rotate_changed_store(p,{'id':'fixed','version':2})
   self.assertTrue((p/'source.json').exists())
if __name__=='__main__':unittest.main()
