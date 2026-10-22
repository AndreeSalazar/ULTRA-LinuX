#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Summarize the bounded software benchmark; reject partial/misordered evidence."""
import argparse
import json
import math
import statistics
from pathlib import Path


def summarize(text):
    groups = {}
    declared = {}
    execution = None
    executions = []
    for line in text.splitlines():
        columns = line.split(',')
        if columns[0] == 'BENCH_EJECUCION':
            execution = int(columns[1])
            executions.append(execution)
        elif columns[0] == 'BENCH':
            if execution is None or len(columns) != 7:
                raise ValueError('Muestra sin ejecucion o columnas incorrectas')
            _, api, round_number, route, wall, cpu, rss = columns
            if api not in ('GLES', 'Vulkan') or route not in ('recrear', 'reutilizar'):
                raise ValueError('API o ruta desconocida')
            sample = dict(ronda=int(round_number), ruta=route, wall_ns=int(wall),
                          cpu_us=float(cpu), rss_pico_kib=int(rss))
            if (sample['wall_ns'] <= 0 or not math.isfinite(sample['cpu_us']) or
                    sample['cpu_us'] <= 0 or sample['rss_pico_kib'] <= 0):
                raise ValueError('Tiempo o RSS no valido')
            groups.setdefault((execution, api), []).append(sample)
        elif columns[0] == 'BENCH_RESUMEN':
            key = (execution, columns[1], columns[2])
            if key in declared:
                raise ValueError('Resumen duplicado')
            declared[key] = [int(field.split('=')[1]) for field in columns[3:]]
    if executions != [1, 2, 3] or len(groups) != 6 or len(declared) != 12:
        raise ValueError('Se requieren tres procesos completos por API')
    result = {'backend': 'llvmpipe en QEMU', 'operaciones_por_muestra': 2048,
              'pares_por_proceso': 7, 'apis': {}}
    for api in ('GLES', 'Vulkan'):
        runs = []
        for execution in (1, 2, 3):
            samples = groups[(execution, api)]
            expected = [(r, ('recrear', 'reutilizar')[(r + p) % 2])
                        for r in range(7) for p in range(2)]
            if [(s['ronda'], s['ruta']) for s in samples] != expected:
                raise ValueError('Rondas incompletas o sin orden alternado')
            run = {'ejecucion': execution}
            for route in ('recrear', 'reutilizar'):
                rows = [s for s in samples if s['ruta'] == route]
                wall = sorted(s['wall_ns'] for s in rows)
                if declared[(execution, api, route)] != [wall[0], wall[3], wall[-1]]:
                    raise ValueError('El resumen no coincide con las muestras')
                run[route] = dict(wall_mediana_ms=wall[3] / 1e6,
                                  wall_min_ms=wall[0] / 1e6, wall_max_ms=wall[-1] / 1e6,
                                  cpu_mediana_ms=statistics.median(s['cpu_us'] for s in rows) / 1000,
                                  rss_pico_kib=max(s['rss_pico_kib'] for s in rows))
            runs.append(run)
        fresh = statistics.median(run['recrear']['wall_mediana_ms'] for run in runs)
        reused = statistics.median(run['reutilizar']['wall_mediana_ms'] for run in runs)
        result['apis'][api] = dict(ejecuciones=runs, recrear_ms=fresh, reutilizar_ms=reused,
                                   reduccion_wall_porcentaje=100 * (1 - reused / fresh))
    result['limites'] = ('Tiempo incluye comprobacion y finalizacion. RSS es pico acumulado del proceso, '
                        'no ahorro por ruta. No mide VRAM ni FPS ni rendimiento de NVIDIA.')
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('informe', type=Path)
    args = parser.parse_args()
    print(json.dumps(summarize(args.informe.read_text(encoding='utf-8')), indent=2,
                     ensure_ascii=True, allow_nan=False))
