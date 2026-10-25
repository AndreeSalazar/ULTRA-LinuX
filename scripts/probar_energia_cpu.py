#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Banco sin hardware: rangos, restauracion, permisos, RAPL y sensores distintos."""
import importlib.machinery
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

BASE = Path(__file__).resolve().parents[1]/'src/rootfs/bin'


def cargar(name):
    loader = importlib.machinery.SourceFileLoader(name, str(BASE/name))
    module = importlib.util.module_from_spec(importlib.util.spec_from_loader(name, loader))
    loader.exec_module(module)
    return module


cpu = cargar('ultra-cpu-control')


class Energia(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.p = self.root/'policy0'; self.p.mkdir()
        for key, value in {'scaling_min_freq':'1700000','scaling_max_freq':'4600000',
                           'energy_performance_preference':'balance_performance',
                           'energy_performance_available_preferences':'performance balance_performance balance_power power'}.items():
            (self.p/key).write_text(value)

    def tearDown(self): self.tmp.cleanup()

    def test_perfiles_restauran_y_no_cambian_minimo(self):
        c=cpu.Control(self.root)
        self.assertEqual(c.aplicar('eficiente'), [])
        self.assertEqual((self.p/'scaling_max_freq').read_text(), '4140000')
        self.assertEqual((self.p/'energy_performance_preference').read_text(), 'balance_power')
        c.aplicar('fresco')
        self.assertEqual((self.p/'scaling_max_freq').read_text(), '3680000')
        self.assertEqual((self.p/'scaling_min_freq').read_text(), '1700000')
        self.assertEqual(c.restaurar(), [])
        self.assertEqual((self.p/'scaling_max_freq').read_text(), '4600000')
        self.assertEqual((self.p/'energy_performance_preference').read_text(), 'balance_performance')

    def test_sin_epp_disponible_no_inventa_soporte(self):
        (self.p/'energy_performance_available_preferences').write_text('performance')
        c=cpu.Control(self.root); self.assertEqual(c.aplicar('eficiente'), [])
        self.assertEqual((self.p/'energy_performance_preference').read_text(), 'balance_performance')
        c.restaurar()

    def test_rechaza_limite_inferior_al_minimo(self):
        (self.p/'scaling_min_freq').write_text('4400000')
        c=cpu.Control(self.root); self.assertTrue(c.aplicar('eficiente'))
        self.assertEqual((self.p/'scaling_max_freq').read_text(), '4600000')

    def test_otro_controlador_no_se_pisa_al_restaurar(self):
        c=cpu.Control(self.root); c.aplicar('eficiente')
        (self.p/'scaling_max_freq').write_text('3000000')
        self.assertTrue(c.restaurar())
        self.assertEqual((self.p/'scaling_max_freq').read_text(), '3000000')

    def test_fallo_parcial_revierte_cambios_previos(self):
        c=cpu.Control(self.root); original=Path.write_text
        def escribir(path, data, *args, **kwargs):
            if path.name=='energy_performance_preference' and data=='balance_power': raise OSError('fallo simulado')
            return original(path, data, *args, **kwargs)
        with patch.object(Path,'write_text',escribir): self.assertTrue(c.aplicar('eficiente'))
        self.assertEqual((self.p/'scaling_max_freq').read_text(), '4600000')
        self.assertEqual(c.perfil, 'original')

    def test_sin_politicas_y_perfil_invalido(self):
        c=cpu.Control(self.root/'ausente'); c.aplicar('eficiente')
        self.assertEqual(c.perfil,'sin_soporte')
        with self.assertRaises(ValueError): c.aplicar('../../etc')

    @unittest.skipUnless(hasattr(os,'O_NOFOLLOW'),'Requiere Linux O_NOFOLLOW')
    def test_peticion_rechaza_symlink_fifo_y_perfiles_desconocidos(self):
        p=self.root/'peticion'; p.write_text('eficiente\n')
        self.assertEqual(cpu.peticion(p,os.getuid()),'eficiente')
        self.assertIsNone(cpu.peticion(p,os.getuid()+1))
        p.write_text('eficiente; comando'); self.assertIsNone(cpu.peticion(p,os.getuid()))
        p.unlink(); p.symlink_to(self.p/'scaling_max_freq'); self.assertIsNone(cpu.peticion(p,os.getuid()))
        p.unlink(); os.mkfifo(p); self.assertIsNone(cpu.peticion(p,os.getuid()))

    def test_rapl_julios_del_intervalo_y_tiempo_monotonico(self):
        self.assertEqual(cpu.potencia_perf('1.0;30.0;Joules;power/energy-pkg/;1000000000;100.00;'),(1.0,30.0))
        self.assertEqual(cpu.potencia_perf('3.0;40.0;Joules;power/energy-pkg/;2000000000;100.00;',1),(3.0,20.0))
        for row in ('1;30;Watts;power/energy-pkg/;1;100', '1;nan;Joules;power/energy-pkg/;1;100',
                    '1;30;Joules;power/energy-pkg/;1;50','1;<not supported>;Joules;power/energy-pkg/;1;100'):
            with self.assertRaises(ValueError): cpu.potencia_perf(row)

    def test_cpu_no_confunde_nvme_gpu_ni_placa(self):
        chips=[('k10temp', [('Tctl',91000),('Tdie',90000)]),('nvme',[('Composite',99000)]),
               ('amdgpu',[('edge',77000)]),('desconocido',[('CPU',120000)])]
        for i,(name,sensors) in enumerate(chips):
            p=self.root/f'class/hwmon/hwmon{i}'; p.mkdir(parents=True)
            (p/'name').write_text(name)
            for n,(label,temp) in enumerate(sensors,1):
                (p/f'temp{n}_input').write_text(str(temp)); (p/f'temp{n}_label').write_text(label)
        result=cpu.temperaturas(self.root)
        self.assertEqual(result['cpu_C'],91)
        self.assertEqual(result['cpu_sensor'],'k10temp/Tctl')
        self.assertEqual(result['gpu_C'],77)
        self.assertEqual(result['disco_C'],99)


if __name__=='__main__': unittest.main(verbosity=2)
