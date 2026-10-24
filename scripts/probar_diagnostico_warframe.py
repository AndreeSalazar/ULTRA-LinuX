#!/usr/bin/env python3
"""Synthetic counters/CSV and bounded recorder; no actual games or accounts."""
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

BASE=Path(__file__).resolve().parents[1]/'src/rootfs/bin'
def load(name):
    loader=importlib.machinery.SourceFileLoader(name,str(BASE/name))
    spec=importlib.util.spec_from_loader(name,loader)
    app=importlib.util.module_from_spec(spec); loader.exec_module(app); return app
diag=load('ultra-diagnostico-juego'); live=load('ultra-juegos-vivo'); kernel=load('ultra-eventos-kernel')


class Diagnostic(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(); self.root=Path(self.temp.name).resolve()
        self.home=self.root/'home'; self.home.mkdir()
        self.runtime=self.root/'runtime'; self.archive=self.root/'archive'; self.frames=self.root/'frames'; self.frames.mkdir()
        self.proc=self.root/'proc'; self.proc.mkdir()
        (self.proc/'stat').write_text('cpu 10 0 10 80 0 0 0 0\n')
        (self.proc/'meminfo').write_text('MemTotal: 4096 kB\nMemAvailable: 2048 kB\n')
        self.task=self.proc/'42'; self.task.mkdir()
        self.fields=['0']*22; self.fields[0]='D'; self.fields[7]='5'; self.fields[9]='2'; self.fields[11]='10'; self.fields[19]='100'; self.fields[21]='100'
        self.write_stat(); (self.task/'environ').write_bytes(b'SteamAppId=230410\0PASSWORD=SECRET\0')
        (self.task/'io').write_text('read_bytes: 100\nwrite_bytes: 50\n')
        thread=self.task/'task/42'; thread.mkdir(parents=True); (thread/'stat').write_text((self.task/'stat').read_text())
        self.patches=[patch.object(diag,'HOME',self.home),patch.object(diag,'RUNTIME',self.runtime),
                      patch.object(diag,'ARCHIVE',self.archive),patch.object(diag,'FRAMES',self.frames),
                      patch.object(diag,'PROC',self.proc),patch.object(live,'PROC',self.proc),
                      patch.object(live,'gpu_sample',return_value=None),patch.object(diag,'KERNEL',self.root/'no-kernel')]
        for p in self.patches: p.start()
    def tearDown(self):
        for p in reversed(self.patches): p.stop()
        self.temp.cleanup()
    def write_stat(self): (self.task/'stat').write_text('42 (GameThread) '+' '.join(self.fields))
    def test_game_cpu_io_faults_and_pid_reuse(self):
        one=live.sample(now=10,measure_pss=False)
        self.fields[11]='110'; self.fields[9]='4'; self.write_stat()
        (self.task/'io').write_text('read_bytes: 1100\nwrite_bytes: 250\n')
        two=live.sample(one,now=12,measure_pss=False)
        rows,game=diag.game_rates(two,one,2)
        self.assertEqual(game['read_Bps'],500); self.assertEqual(game['major_faults_delta'],2)
        self.assertIsNone(game['pss_bytes']); self.assertNotIn('SECRET',json.dumps(two))
        self.fields[19]='999'; self.write_stat()
        reused=live.sample(two,now=14,measure_pss=False)
        self.assertIsNone(diag.game_rates(reused,two,2)[1]['major_faults_delta'])
        self.assertIsNone(diag.game_rates(reused,two,2)[1]['read_Bps'])
    def test_global_rates_units_and_missing_values(self):
        old=dict(network={'eth0':dict(rx_bytes=1000,tx_bytes=0,rx_errors=0,tx_errors=0,rx_dropped=0,tx_dropped=0)},disks={'8:0:sda':[2,0,8,10,1,0,4,4,0,10,12]},pressure={'io.some':0},vm={'pswpin':0})
        cur=dict(network={'eth0':dict(rx_bytes=2000,tx_bytes=100,rx_errors=0,tx_errors=0,rx_dropped=0,tx_dropped=0)},disks={'8:0:sda':[4,0,16,20,2,0,8,10,1,20,30]},pressure={'io.some':1000000},vm={'pswpin':1})
        result=diag.context_rates(cur,old,2)
        self.assertEqual(result['network_global']['eth0']['rx_Bps'],500)
        self.assertEqual(result['disks_global']['8:0:sda']['read_Bps'],2048)
        self.assertEqual(result['disks_global']['8:0:sda']['read_mean_ms'],5)
        self.assertEqual(result['pressure_global']['io.some'],50)
        self.assertIsNone(diag.context_rates(cur,None,0)['network_global']['eth0']['rx_Bps'])
        self.assertIsNone(diag.rate(1,10,2))
    def test_thread_cpu_survives_missing_wait_channel(self):
        sample=live.sample(now=1,measure_pss=False)
        before,info=diag.threads(live,sample['processes'],{},1)
        self.assertEqual(info['observed'],1); self.assertEqual(info['uninterruptible'],1)
        self.assertIsNone(info['top'][0]['wait_channel'])
    def test_frame_parser_ignores_system_identity_and_invalid_numbers(self):
        path=self.frames/'Warframe-test.csv'
        path.write_text('os,cpu,gpu\nSECRET,private,private\nfps,frametime,elapsed\n60,16,1000000000\n12,80,2000000000\n60,nan,3000000000\n')
        result=diag.frame_summary(path)
        self.assertEqual(result['frametime_ms']['p99'],80); self.assertEqual(result['rows'],2)
        self.assertEqual(result['invalid_rows'],1); self.assertNotIn('SECRET',json.dumps(result))
    def test_kernel_events_do_not_export_text_or_addresses(self):
        row=kernel.event('[ 12.50] NVRM: Xid (PCI:0000:29:00): 109, pid=42, name=SECRET, channel 0xabc')
        self.assertEqual(row,dict(kernel_s=12.5,type='nvidia_xid',pid=42,code=109))
        self.assertNotIn('SECRET',json.dumps(row)); self.assertNotIn('PCI',json.dumps(row))
        self.assertIsNone(kernel.event('[ 1.0] normal message SECRET'))
        self.assertEqual(kernel.event('[ 2.0] audit: type=1326 pid=42 syscall=16 args=SECRET')['type'],'seccomp_audit')
    def test_record_bounded_and_no_privacy_leak(self):
        diag.record(live,duration=1.1)
        state=json.loads((self.runtime/'estado.json').read_text())
        self.assertFalse(state['running']); self.assertGreaterEqual(state['samples'],1)
        saved=(self.archive/state['session']/'recursos.jsonl').read_text()
        self.assertNotIn('SECRET',saved); self.assertIn('steam:230410',saved)
        self.assertLess(len(saved),diag.MAX_RESOURCE)
        self.assertIsNone(state['gpu_global'])
    def test_three_captures_limit_preserves_existing_data(self):
        self.archive.mkdir()
        for i in range(3): (self.archive/f'sesion-{i}').mkdir()
        with self.assertRaises(ValueError): diag.record(live,duration=1)
        self.assertEqual(len(list(self.archive.iterdir())),3)


if __name__=='__main__': unittest.main(verbosity=2)
