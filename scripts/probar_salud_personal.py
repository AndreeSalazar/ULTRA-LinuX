#!/usr/bin/env python3
"""Semaforo del informe, disco Personal y salud ext4, sin discos reales."""
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]


def load(name):
    loader = importlib.machinery.SourceFileLoader(name.replace('-', '_'), str(ROOT/'src/rootfs/bin'/name))
    spec = importlib.util.spec_from_loader(loader.name, loader)
    module = importlib.util.module_from_spec(spec); loader.exec_module(module)
    return module


salud, personal, datos, respaldo = (load(n) for n in ('ultra-salud', 'ultra-personal', 'ultra-comprobar-datos', 'ultra-respaldo'))

REPORT = """ULTRA GPU - ultimo intento
kernel=7.3.0
modo=nvidia
compositor=hyprland
salida=0

[DRIVER]
Model: \t\t NVIDIA GeForce RTX 3060

[RED: preparacion de esta sesion]
VERDE [RED]: enlace listo
RED iface=enp37s0 speed_Mbps=1000

[AUDIO: tarjetas]
Audio
 ├─ Sinks:
 │  *   52. USB Audio Device Analog Stereo      [vol: 1.00]
 │
[E][17:02:11.590989] mod.rt       | algo
VERDE [AUDIO-RT]: rtprio=88

[JUEGOS: montaje]
VERDE [JUEGOS]: listo

[HERONITE: ultimo inicio]
{"fase": "interfaz", "resultado": "entregado", "error": "", "advertencias": []}

[CPU: temperatura]
[{"cpu_max_C": 61.5}]

[LIMITES]
oom_kill 0

[DATOS Y RESPALDOS: superbloques]
DISCOS [{"punto": "/datos", "montado": true, "sistema": "ext4", "estado": "clean", "errores": 0}, {"punto": "/personal", "montado": false}]
/personal/ultra/respaldos/ULTRA-respaldo-20261102-090000.tar.zst 1234

[LANZAMIENTO DEL CLIENTE]
listo
"""
NOW = time.mktime(time.strptime('20261102-120000', '%Y%m%d-%H%M%S'))


class Salud(unittest.TestCase):
    def lines(self, text):
        return [l for l in salud.summary(text, NOW).splitlines() if l[:4] in ('VERD', 'AMAR', 'ROJO')]

    def test_healthy_session_is_all_green(self):
        lines = self.lines(REPORT)
        self.assertTrue(all(l.startswith('VERDE') for l in lines), lines)
        self.assertIn('total: 11 verde, 0 amarillo, 0 rojo', salud.summary(REPORT, NOW))

    def test_problems_sorted_red_first_with_advice(self):
        bad = (REPORT.replace('salida=0', 'salida=1').replace('[vol: 1.00]', '[vol: 3.00]')
               .replace('speed_Mbps=1000', 'speed_Mbps=100\n[INTERFAZ x]\n 1000baseT/Full')
               .replace('"estado": "clean", "errores": 0', '"estado": "clean with errors", "errores": 3')
               .replace('VERDE [AUDIO-RT]: rtprio=88', 'Realtime scheduling disabled')
               .replace('oom_kill 0', 'oom_kill 2'))
        lines = self.lines(bad)
        self.assertTrue(lines[0].startswith('ROJO'))
        text = '\n'.join(lines)
        for expected in ('salida 1', 'al 300%', '100 Mb/s', 'ext4 con errores (3)', 'sin tiempo real', '2 procesos'):
            self.assertIn(expected, text)

    def test_backup_age_and_place(self):
        local = REPORT.replace('/personal/ultra/respaldos/', '/juegos/usuarios/ultra/.local/share/ultra/respaldos/')
        self.assertIn('prepara D:', '\n'.join(self.lines(local)))
        old = REPORT.replace('20261102-090000', '20261020-090000')
        self.assertIn('13 dias', '\n'.join(self.lines(old)))

    def test_real_report_when_available(self):
        real = Path('A:/ULTRA/informe/gpu-7.3.0-rc7-pc-g32c52ec87d62.txt')
        if not real.is_file(): self.skipTest('informe real no disponible')
        text = '\n'.join(self.lines(real.read_text(encoding='utf-8', errors='replace')))
        for expected in ('300%', '100 Mb/s', 'RTX 3060', 'Audio RT'):
            self.assertIn(expected, text)


class Personal(unittest.TestCase):
    def part(self, **change):
        row = dict(name='/dev/sdb1', type='part', fstype='exfat', label='PERSONAL', tran='sata', mountpoints=[None])
        row.update(change)
        return row

    def test_finds_internal_exfat_personal_only(self):
        self.assertEqual(personal.find_personal([self.part()])['name'], '/dev/sdb1')
        self.assertEqual(personal.find_personal([self.part(fstype='ext4', label='ULTRA-PERSONAL')])['name'], '/dev/sdb1')
        for other in (dict(tran='usb'), dict(label='Personal'), dict(fstype='ntfs'), dict(type='disk')):
            self.assertIsNone(personal.find_personal([self.part(**other)]), other)
        with self.assertRaises(ValueError): personal.find_personal([self.part(), self.part(name='/dev/sdc1')])

    def test_exfat_mount_is_private_and_never_formats(self):
        calls = []
        free = type('V', (), {'f_bavail': 1, 'f_frsize': 1})()
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(personal, 'TARGET', Path(tmp)/'personal'), \
                 patch.object(personal, 'USER_DIR', Path(tmp)/'personal/ultra'), \
                 patch.object(personal, 'blocks', lambda: [self.part()]), \
                 patch.object(personal.os, 'statvfs', lambda p: free, create=True), \
                 patch.object(personal, 'run', lambda *a, **k: calls.append(a) or ''):
                personal.mount_personal()
        (call,) = calls
        self.assertEqual(call[:3], ('/usr/bin/mount', '-t', 'exfat'))
        self.assertTrue({'uid=1000', 'dmask=0077', 'fmask=0177', 'noexec', 'nosuid'} <= set(call[4].split(',')))
        self.assertFalse(any('mkfs' in a[0] or 'sfdisk' in a[0] for a in calls))

class Datos(unittest.TestCase):
    def test_superblock_fields(self):
        text = ('Filesystem volume name:   ULTRA-DATOS\nFilesystem state:         clean with errors\n'
                'FS Error count:           3\nMount count:              41\nLast checked:             Sat Oct 10\n')
        doc = datos.parse(text)
        self.assertEqual((doc['estado'], doc['errores'], doc['etiqueta']), ('clean with errors', 3, 'ULTRA-DATOS'))


class Destino(unittest.TestCase):
    def test_local_when_no_personal_disk(self):
        with patch.object(respaldo.os.path, 'ismount', return_value=False):
            self.assertEqual(respaldo.destination(), (respaldo.LOCAL, respaldo.KEEP_LOCAL))

    def test_personal_when_mounted_and_owned(self):
        with tempfile.TemporaryDirectory() as tmp:
            folder = Path(tmp)/'ultra'; folder.mkdir()
            with patch.object(respaldo, 'PERSONAL', folder), patch.object(respaldo.os.path, 'ismount', return_value=True), \
                 patch.object(respaldo.os, 'geteuid', return_value=folder.stat().st_uid, create=True):
                self.assertEqual(respaldo.destination(), (folder/'respaldos', respaldo.KEEP_PERSONAL))


if __name__ == '__main__': unittest.main(verbosity=2)
