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


class Suspension(unittest.TestCase):
    """NVIDIA-safe S3 on a fake /sys/power and /proc/driver/nvidia."""
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();root=Path(self.tmp.name)
        self.power=root/'power';self.power.mkdir();(self.power/'state').write_text('freeze mem\n')
        (self.power/'mem_sleep').write_text('[s2idle] deep\n')
        self.gpu=root/'nvidia';self.gpu.mkdir();(self.gpu/'suspend').write_text('')
        (self.gpu/'params').write_text('ResmanDebugLevel: 4294967295\nPreserveVideoMemoryAllocations: 1\n')
        self.writes=[];original=Path.write_text
        def record(path,text,*a,**k):
            self.writes.append((path.name,text));return original(path,text,*a,**k)
        self.patches=[patch.object(app,'POWER',self.power),patch.object(app,'NVIDIA',self.gpu),
                      patch.object(Path,'write_text',record),
                      patch.object(app.os,'sync',lambda:None,create=True)]
        for p in self.patches: p.start()
    def tearDown(self):
        for p in reversed(self.patches): p.stop()
        self.tmp.cleanup()
    def test_deep_and_nvidia_order(self):
        self.assertEqual(app.perform('suspender'),'reanudado')
        self.assertEqual(self.writes,[('mem_sleep','deep'),('suspend','suspend'),('state','mem'),('suspend','resume')])
    def test_nvidia_notifier_handles_gpu_without_proc_writes(self):
        with (self.gpu/'params').open('a') as params:
            params.write('UseKernelSuspendNotifiers: 1\n')
        self.assertEqual(app.perform('suspender'), 'reanudado')
        self.assertEqual(self.writes, [('mem_sleep', 'deep'), ('state', 'mem')])
    def test_gpu_resumes_even_if_sleep_fails(self):
        original=Path.write_text
        def fail_state(path,text,*a,**k):
            self.writes.append((path.name,text))
            if path.name=='state': raise OSError('rechazado')
            return original(path,text,*a,**k)
        with patch.object(Path,'write_text',fail_state):
            with self.assertRaises(OSError): app.perform('suspender')
        self.assertEqual(self.writes[-1],('suspend','resume'))
    def test_refuses_without_preserved_vram(self):
        (self.gpu/'params').write_text('PreserveVideoMemoryAllocations: 0\n')
        self.writes.clear()
        with self.assertRaisesRegex(ValueError,'memoria de video'): app.perform('suspender')
        self.assertNotIn(('state','mem'),self.writes)
    def test_without_nvidia_plain_s3(self):
        (self.gpu/'suspend').unlink();(self.gpu/'params').unlink()
        (self.power/'mem_sleep').write_text('s2idle [deep]\n')
        self.writes.clear()
        app.perform('suspender')
        self.assertEqual(self.writes,[('state','mem')])


if __name__=='__main__': unittest.main(verbosity=2)
