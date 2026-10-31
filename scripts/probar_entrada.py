#!/usr/bin/env python3
"""Input settings contracts and rollback; no physical devices or user profiles."""
import copy
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT=Path(__file__).resolve().parents[1]
loader=importlib.machinery.SourceFileLoader('entrada',str(ROOT/'src/rootfs/bin/ultra-entrada'))
spec=importlib.util.spec_from_loader(loader.name,loader);app=importlib.util.module_from_spec(spec);loader.exec_module(app)


class Input(unittest.TestCase):
    def test_ranges_types_and_injection(self):
        app.valid(app.DEFAULT)
        for key,value in [('sensitivity',float('nan')),('sensitivity',True),('scroll_factor',0),
                          ('repeat_rate',10000),('repeat_delay',True),('left_handed',1),
                          ('kb_layout','us"});os.execute("bad")'),('kb_options','x\n'),
                          ('accel_profile','custom'),('extra',1)]:
            with self.subTest(key=key),self.assertRaises(ValueError):app.valid({**app.DEFAULT,key:value})
    def test_fixed_lua_no_shell(self):
        with patch.object(app,'run',return_value='ok') as run:
            app.apply({**app.DEFAULT,'kb_layout':'us,latam','kb_options':'grp:alt_shift_toggle'})
        args=run.call_args.args[0]
        self.assertEqual(args[:2],['hyprctl','eval'])
        self.assertIn('natural_scroll=false',args[2]);self.assertIn('kb_layout="us,latam"',args[2])
    def test_snapshot_requires_all_fields(self):
        def run(args):
            k=args[-1].split(':')[1];v=app.DEFAULT[k]
            t='bool' if type(v)is bool else 'int' if type(v)is int else 'float' if type(v)is float else 'str'
            return json.dumps({t:('[[EMPTY]]' if v == '' else v)})
        with patch.object(app,'run',side_effect=run):self.assertEqual(app.snapshot(),app.DEFAULT)
        with patch.object(app,'run',return_value='{}'),self.assertRaises(ValueError):app.snapshot()
    @unittest.skipUnless(os.name=='posix','Linux permissions')
    def test_atomic_persistence_and_restore_failure(self):
        with tempfile.TemporaryDirectory() as tmp:
            path=app.folder(Path(tmp).resolve())/'entrada.json'
            new={**app.DEFAULT,'sensitivity':.6,'accel_profile':'flat'}
            with patch.object(app,'snapshot',side_effect=[app.DEFAULT,new]),patch.object(app,'apply'):
                app.commit(path,new,True)
            before=path.read_bytes();self.assertEqual(app.read(path),new)
            self.assertEqual(path.stat().st_mode & 0o777,0o600)
            with patch.object(app,'snapshot',side_effect=[new,new]),patch.object(app,'apply') as apply:
                with self.assertRaises(ValueError):app.commit(path,app.DEFAULT,True)
            self.assertEqual(apply.call_args_list[-1].args[0],new)
            self.assertEqual(path.read_bytes(),before)
            with patch.object(app,'snapshot',side_effect=[new,app.DEFAULT]),patch.object(app,'apply') as apply,patch.object(app,'save',side_effect=OSError('disk full')):
                with self.assertRaises(OSError):app.commit(path,app.DEFAULT,True)
            self.assertEqual(apply.call_args_list[-1].args[0],new)
            self.assertEqual(path.read_bytes(),before)
    @unittest.skipUnless(os.name=='posix','Linux links')
    def test_links_and_oversized_profile_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp).resolve();target=root/'outside';target.mkdir();(root/'.config').symlink_to(target)
            with self.assertRaises(ValueError):app.folder(root)
            self.assertFalse((target/'ultra').exists())
            p=root/'large';p.write_text('x'*8193)
            with self.assertRaises(ValueError):app.read(p)
            link=root/'link';link.symlink_to(p)
            with self.assertRaises(OSError):app.read(link)
    def test_invalid_never_changes_compositor(self):
        with patch.object(app,'snapshot') as snap,patch.object(app,'apply') as apply:
            with self.assertRaises(ValueError):app.commit(Path('/no-write'),{'bad':1},True)
        snap.assert_not_called();apply.assert_not_called()


if __name__=='__main__':unittest.main(verbosity=2)
