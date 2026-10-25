#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
import unittest
import json
from analizar_energia_informe import analizar


class EnergiaInforme(unittest.TestCase):
    def test_lector_nuevo_con_cpu_rapl_y_temperatura_etiquetada(self):
        samples = [{'monotonic_s':100,'cpu_paquete_W':30.5,'termica':{'cpu_C':80},'perfil':'eficiente'},
                   {'monotonic_s':102,'cpu_paquete_W':None,'termica':{'cpu_C':91},'perfil':'fresco'}]
        result=analizar('[CPU: temperatura etiquetada, perfil y RAPL; ultimas 300 muestras]\n'+json.dumps(samples)+'\n[/CPU ENERGIA]\n')
        self.assertEqual(result['cpu_W']['media'],30.5)
        self.assertEqual(result['cpu_temperatura_C']['max'],91)
        self.assertEqual(result['perfiles_energia_observados'],['eficiente','fresco'])
        self.assertEqual(result['ventana_energia_cpu_monotonic_s'],[100,102])
    def test_delta_no_suma_guest_otra_vez_y_detecta_salto_hora(self):
        data = ('muestra_utc=2026-10-25T00:00:00Z\n'
                'CPU_contadores cpu 10 0 10 80 0 0 0 0 1000 0\n'
                'RELOJ uptime_idle_s=1.00 0\n'
                'muestra_utc=2026-10-25T05:00:01Z\n'
                'CPU_contadores cpu 40 0 30 130 0 0 0 0 3000 0\n'
                'RELOJ uptime_idle_s=2.00 0\n'
                'GPU GPU-ficticia, 0000:00:00.0, 100, 20, 50, 60, 25.5\n')
        result = analizar(data)
        self.assertEqual(result['cpu_ocupado_pct_periodo'], 50)
        self.assertEqual(result['duracion_segun_uptime_s'], 1)
        self.assertEqual(result['saltos_reloj_civil_s'], [18000])
        self.assertEqual(result['gpu_W']['media'], 25.5)
        self.assertIsNone(result['cpu_W'])

    def test_contador_reiniciado_no_inventa_porcentaje(self):
        result = analizar('muestra_utc=a\nCPU_contadores cpu 20 0 20 60 0 0 0 0\n'
                          'muestra_utc=b\nCPU_contadores cpu 1 0 1 2 0 0 0 0\n')
        self.assertEqual(result['intervalos_cpu_rechazados'], 1)
        self.assertIsNone(result['cpu_ocupado_pct_maquina'])

    def test_faltantes_no_son_cero_y_gpu_no_finita_se_rechaza(self):
        result = analizar('muestra_utc=a\nGPU GPU-ficticia, x, 100, 20, 50, 60, NaN\n')
        self.assertIsNone(result['gpu_W'])
        self.assertIsNone(result['cpu_W'])
        self.assertIsNone(result['cpu_ocupado_pct_maquina'])

    def test_media_periodo_pondera_contadores_y_no_promedia_porcentajes(self):
        result = analizar('muestra_utc=a\nCPU_contadores cpu 0 0 0 0 0 0 0 0\n'
                          'muestra_utc=b\nCPU_contadores cpu 1 0 0 0 0 0 0 0\n'
                          'muestra_utc=c\nCPU_contadores cpu 1 0 0 9 0 0 0 0\n')
        self.assertEqual(result['cpu_ocupado_pct_maquina']['media'], 50)
        self.assertEqual(result['cpu_ocupado_pct_periodo'], 10)


if __name__ == '__main__':
    unittest.main(verbosity=2)
