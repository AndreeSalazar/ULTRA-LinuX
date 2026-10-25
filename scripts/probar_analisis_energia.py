#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
import unittest
from analizar_energia_informe import analizar


class EnergiaInforme(unittest.TestCase):
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
