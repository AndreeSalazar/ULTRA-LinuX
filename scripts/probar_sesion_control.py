#!/usr/bin/env python3
"""Fixed actions, stale requests and GUI cancellation; no real power operation."""
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

root=Path(__file__).resolve().parents[1]
loader=importlib.machinery.SourceFileLoader('session_control',str(root/'src/rootfs/bin/ultra-sesion-control'))
spec=importlib.util.spec_from_loader(loader.name,loader);app=importlib.util.module_from_spec(spec);loader.exec_module(app)


class Control(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.root=Path(self.tmp.name).resolve();self.path=self.root/'request'
    def tearDown(self): self.tmp.cleanup()
    def write(self,**changes):
        doc={'accion':'apagar','id':'a'*32,'monotonic':100.0}|changes
        self.path.write_text(json.dumps(doc));self.path.chmod(0o600)
    def test_valid_action_and_expiration(self):
        self.write();self.assertEqual(app.decode_request(self.path,now=101)['accion'],'apagar')
        with self.assertRaises(ValueError): app.decode_request(self.path,now=131)
    def test_unknown_action_nan_extra_fields_and_future_rejected(self):
        for changes in ({'accion':'apagar; reboot'},{'monotonic':float('nan')},{'otro':'dato'},{'monotonic':120.0}):
            self.write(**changes)
            with self.assertRaises(ValueError): app.decode_request(self.path,now=101)
    def test_symlink_public_and_oversized_requests_rejected(self):
        self.write();self.path.chmod(0o644)
        with self.assertRaises(ValueError): app.decode_request(self.path,now=101)
        self.path.write_text('x'*513);self.path.chmod(0o600)
        with self.assertRaises(ValueError): app.decode_request(self.path,now=101)
        self.path.unlink();self.path.symlink_to(self.root/'absent')
        with self.assertRaises(OSError): app.decode_request(self.path,now=101)
    def test_cancel_never_submits(self):
        for choices in ([None],[4],[0,0]):
            with patch.object(app,'choose',side_effect=choices),patch.object(app,'request') as request:
                app.menu();request.assert_not_called()
    def test_restart_uses_fixed_dispatch_and_pending_value(self):
        with patch.object(app,'PENDING',self.root/'pending'),patch.object(app.subprocess,'run') as run:
            self.assertEqual(app.perform('reiniciar'),'cerrando_sesion')
            self.assertEqual((self.root/'pending').read_text(),'reiniciar\n')
            self.assertEqual(run.call_args.args[0][-2:],['dispatch','hl.dsp.exit()'])
    def test_failed_close_does_not_leave_power_action(self):
        with patch.object(app,'PENDING',self.root/'pending'),patch.object(app.subprocess,'run',side_effect=OSError('test')):
            with self.assertRaises(OSError): app.perform('apagar')
            self.assertFalse((self.root/'pending').exists())
    def test_unsupported_suspend_never_writes(self):
        with patch.object(app,'supports_suspend',return_value=False),patch.object(app.os,'sync') as sync:
            with self.assertRaises(ValueError): app.perform('suspender')
            sync.assert_not_called()


if __name__=='__main__': unittest.main(verbosity=2)
