#!/usr/bin/env python3
"""VIGILANTE fixtures: PID reuse, deltas, unavailable HUD and bounded private output."""
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

path = Path(__file__).resolve().parents[1]/'src/rootfs/bin/ultra-vigilante'
loader = importlib.machinery.SourceFileLoader('vigilante_test', str(path))
spec = importlib.util.spec_from_loader(loader.name, loader)
app = importlib.util.module_from_spec(spec); loader.exec_module(app)


class Vigilante(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(); self.root = Path(self.tmp.name)
        self.proc = self.root/'proc'; self.proc.mkdir()
        self.cg = self.root/'cgroup'; self.cg.mkdir()
        self.energy = self.root/'energy.json'
        self.patches = [patch.object(app, 'CGROUP', self.cg), patch.object(app, 'ENERGY', self.energy)]
        for p in self.patches: p.start()
        self.process()
    def tearDown(self):
        for p in self.patches: p.stop()
        self.tmp.cleanup()
    def process(self, start=100, major=2, switches=10):
        p = self.proc/'42'; p.mkdir(exist_ok=True)
        fields = ['0']*22; fields[19] = str(start)
        (p/'stat').write_text('42 (game ) name) '+' '.join(fields))
        (p/'status').write_text(f'voluntary_ctxt_switches:\t{switches}\nnonvoluntary_ctxt_switches:\t3\n')
        (p/'maps').write_text('00000 r-xp 000 00:01 12 /usr/lib/mangohud/libMangoHud.so\n')
        (p/'environ').write_bytes(b'MANGOHUD=1\0TOKEN=NEVER-EXPORT\0MANGOHUD_CONFIG=fps,no_display\0')
        return {'pid':42,'start':start,'name':'game','game_id':'steam:22','cpu_pct':100,
                'major_faults':major,'minor_faults':20,'io_read_Bps':30,'io_write_Bps':0}
    def sample(self, before=None, now=10, **kw):
        state = {'monotonic':now,'processes':[self.process(**kw)]}
        app.enrich(state, before, self.proc)
        return state
    def test_first_sample_unknown_then_deltas(self):
        one = self.sample(); two = self.sample(one, now=15, major=5, switches=20)
        self.assertIsNone(one['vigilante']['processes'][0]['deltas']['major_faults'])
        row = two['vigilante']['processes'][0]
        self.assertEqual(row['deltas']['major_faults'],3)
        self.assertEqual(row['deltas']['voluntary_ctxt_switches'],10)
        self.assertTrue(row['hud']['mapped']); self.assertTrue(row['hud']['env_hidden'])
        self.assertNotIn('NEVER-EXPORT',json.dumps(two))
        self.assertIsNone(two['vigilante']['fps']); self.assertFalse(two['vigilante']['automatic_tuning'])
    def test_reused_pid_has_no_delta(self):
        one = self.sample(); two = self.sample(one, now=15, start=200, major=999)
        self.assertIsNone(two['vigilante']['processes'][0]['deltas']['major_faults'])
    def test_pid_changes_during_extra_reads_are_rejected(self):
        state = {'monotonic':10,'processes':[self.process()]}
        with patch.object(app,'stat_start',return_value=999): app.enrich(state,None,self.proc)
        self.assertEqual(state['vigilante']['processes'],[])
    def test_missing_and_oversized_maps_not_false(self):
        p = self.proc/'42'; (p/'maps').unlink()
        self.assertIsNone(app.hud(p)['mapped'])
        (p/'maps').write_bytes(b'x'*1048577)
        self.assertIsNone(app.hud(p)['mapped'])
        (p/'maps').write_text('000 /usr/lib/libc.so.6\n')
        self.assertFalse(app.hud(p)['mapped'])
    def test_counter_reset_and_stale_interval_unknown(self):
        self.assertIsNone(app.delta(1,99,5)); self.assertIsNone(app.delta(9,1,121))
        self.assertIsNone(app.delta(9,1,0)); self.assertEqual(app.delta(9,1,5),8)
    def test_cgroup_delta_is_not_lifetime_total(self):
        file = self.cg/'memory.events'; file.write_text('high 23000\noom 0\n')
        one = self.sample(); file.write_text('high 23002\noom 0\n')
        two = self.sample(one,now=15)
        self.assertIsNone(one['vigilante']['memory_events_delta'])
        self.assertEqual(two['vigilante']['memory_events_delta']['high'],2)
        one['vigilante']['memory_identity']=[-1,-1]
        self.assertIsNone(self.sample(one,now=15)['vigilante']['memory_events_delta'])
    def test_stale_energy_is_not_current(self):
        self.energy.write_text(json.dumps({'monotonic_s':1,'cpu_paquete_W':99,'termica':{'cpu_C':91}}))
        self.assertIsNone(self.sample(now=10)['vigilante']['cpu_energy'])
        self.energy.write_text(json.dumps({'monotonic_s':9,'cpu_paquete_W':42,'termica':{'cpu_C':75}}))
        self.assertEqual(self.sample(now=10)['vigilante']['cpu_energy']['package_W'],42)
    def test_bounded_history_and_selection(self):
        self.assertEqual(len(app.history_document([{}]*150)['samples']),120)
        self.assertLessEqual(len(json.dumps(app.history_document([{'large':'x'*20000}]*120)).encode()),app.MAX_HISTORY_BYTES)
        state = {'monotonic':10,'processes':[self.process() for _ in range(30)]}
        app.enrich(state,None,self.proc)
        self.assertTrue(state['vigilante']['limited']); self.assertEqual(len(state['vigilante']['processes']),16)
    def test_previous_session_is_not_exported(self):
        doc = app.history_document([{'monotonic_s':10},{'monotonic_s':20}])
        self.assertEqual(app.session_history(doc,15)['samples'],[{'monotonic_s':20}])
        with self.assertRaises(ValueError): app.session_history(doc,30)
        with self.assertRaises(ValueError): app.session_history(doc,float('nan'))
        doc['boot_id']='previous-boot'
        with self.assertRaises(ValueError): app.session_history(doc,0)
    @unittest.skipUnless(os.name=='posix','Linux FIFO')
    def test_fifo_does_not_block(self):
        p=self.root/'fifo'; os.mkfifo(p)
        with self.assertRaises(ValueError): app.read(p)


if __name__=='__main__': unittest.main(verbosity=2)
