#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Resume contadores de un informe existente; no estima W de CPU desde su uso."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import statistics
import math
from datetime import datetime


def resumen(values):
    if not values:
        return None
    return {"min": round(min(values), 3), "media": round(statistics.mean(values), 3),
            "max": round(max(values), 3), "n": len(values)}


def analizar(text):
    potencia_cpu = []
    temperatura_cpu = []
    perfiles = set()
    reloj_energia = []
    block = re.search(r'^\[CPU: temperatura etiquetada, perfil y RAPL; ultimas 300 muestras\]\s*\n(.*?)^\[/CPU ENERGIA\]', text, re.M | re.S)
    if block:
        try:
            data = json.loads(block[1])
            if isinstance(data, list) and len(data) <= 300:
                for sample in data:
                    if not isinstance(sample, dict): continue
                    t = sample.get('monotonic_s')
                    if isinstance(t, (int,float)) and math.isfinite(t): reloj_energia.append(t)
                    value = sample.get('cpu_paquete_W')
                    if not isinstance(value, bool) and isinstance(value, (int,float)) and math.isfinite(value) and 0 <= value < 10000:
                        potencia_cpu.append(value)
                    thermal = sample.get('termica')
                    value = thermal.get('cpu_C') if isinstance(thermal, dict) else None
                    if not isinstance(value, bool) and isinstance(value, (int,float)) and math.isfinite(value) and -40 <= value <= 150:
                        temperatura_cpu.append(value)
                    if sample.get('perfil') in ('original','eficiente','fresco','sin_soporte'):
                        perfiles.add(sample['perfil'])
        except (ValueError, TypeError): pass
    samples = []
    current = None
    processes = set()
    for line in text.splitlines():
        if line.startswith('muestra_utc='):
            current = {"utc": line.split('=', 1)[1]}
            samples.append(current)
        elif current is not None and line.startswith('CPU_contadores cpu '):
            values = [int(n) for n in line.split()[2:10]]
            if len(values) == 8:
                current['cpu'] = values
        elif current is not None and line.startswith('RELOJ uptime_idle_s='):
            try:
                current['uptime'] = float(line.split('=', 1)[1].split()[0])
            except (ValueError, IndexError):
                pass
        elif current is not None and line.startswith('GPU GPU-'):
            fields = line.split(',')
            if len(fields) == 7:
                for key, index in [('gpu_W', 6), ('gpu_pct', 4), ('gpu_C', 5), ('vram_MiB', 3)]:
                    try:
                        value = float(fields[index])
                        if value >= 0 and value < float('inf'):
                            current[key] = value
                    except ValueError:
                        pass
        match = re.match(r'PROCESO pid=\d+ nombre=(\S+)', line)
        if match:
            processes.add(match[1])
    cpu = []
    rejected = 0
    busy_total = 0
    ticks_total = 0
    clock_jumps = []
    for previous, current in zip(samples, samples[1:]):
        if 'cpu' not in previous or 'cpu' not in current:
            rejected += 1
            continue
        delta = [b-a for a, b in zip(previous['cpu'], current['cpu'])]
        if min(delta) < 0 or sum(delta) <= 0:
            rejected += 1
            continue
        cpu.append(100 * (sum(delta)-delta[3]-delta[4]) / sum(delta))
        busy_total += sum(delta)-delta[3]-delta[4]
        ticks_total += sum(delta)
        if 'uptime' in previous and 'uptime' in current:
            monotonic_delta = current['uptime']-previous['uptime']
            try:
                wall_delta = (datetime.fromisoformat(current['utc'])-datetime.fromisoformat(previous['utc'])).total_seconds()
                if abs(wall_delta-monotonic_delta) > 2:
                    clock_jumps.append(round(wall_delta-monotonic_delta, 2))
            except ValueError:
                pass
    policies = {}
    for policy, key, value in re.findall(r'^CPU politica=(\S+) (scaling_driver|scaling_governor|energy_performance_preference)=(\S+)', text, re.M):
        policies.setdefault(policy, {}).setdefault(key, set()).add(value)
    elapsed = samples[-1]['uptime']-samples[0]['uptime'] if samples and all('uptime' in s for s in (samples[0], samples[-1])) else None
    return {"muestras": len(samples), "duracion_segun_uptime_s": round(elapsed, 2) if elapsed is not None else None,
            "saltos_reloj_civil_s": clock_jumps,
            "cpu_ocupado_pct_maquina": resumen(cpu),
            "cpu_ocupado_pct_periodo": round(100*busy_total/ticks_total, 3) if ticks_total else None,
            "intervalos_cpu_rechazados": rejected,
            "gpu_W": resumen([s['gpu_W'] for s in samples if 'gpu_W' in s]),
            "gpu_pct": resumen([s['gpu_pct'] for s in samples if 'gpu_pct' in s]),
            "gpu_C": resumen([s['gpu_C'] for s in samples if 'gpu_C' in s]),
            "vram_MiB": resumen([s['vram_MiB'] for s in samples if 'vram_MiB' in s]),
            "cpu_W": resumen(potencia_cpu),
            "cpu_temperatura_C": resumen(temperatura_cpu), "perfiles_energia_observados": sorted(perfiles),
            "ventana_energia_cpu_monotonic_s": [min(reloj_energia),max(reloj_energia)] if reloj_energia else None,
            "motivo_cpu_W": "Muestras RAPL del paquete; media de observaciones, no energia total integrada." if potencia_cpu else "No hay muestras RAPL CPU validas; no se infiere desde porcentaje, frecuencia ni GPU.",
            "politicas": {p: {k: sorted(v) for k, v in values.items()} for p, values in policies.items()},
            "procesos_observados": sorted(processes),
            "primera_muestra_utc": samples[0]['utc'] if samples else None,
            "ultima_muestra_utc": samples[-1]['utc'] if samples else None,
            "limites": ["Uso CPU agregado de la maquina: 100% representa todos sus hilos.",
                        "Se excluyen idle e iowait; guest ya esta incluido en user/nice y no se suma otra vez.",
                        "GPU W corresponde a la tarjeta; no al procesador ni al enchufe.",
                        "Intervalos con contadores faltantes, reiniciados o sin avance se rechazan.",
                        "Estas muestras no identifican por si mismas causas ni consumo por proceso."]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('informe', type=Path)
    parser.add_argument('salida', type=Path)
    args = parser.parse_args()
    raw = args.informe.read_bytes()
    result = analizar(raw.decode('utf-8-sig', errors='replace'))
    result['informe_sha256'] = hashlib.sha256(raw).hexdigest()
    args.salida.parent.mkdir(parents=True, exist_ok=True)
    args.salida.write_text(json.dumps(result, ensure_ascii=False, indent=2)+'\n', encoding='utf-8', newline='\n')
    print(json.dumps({k:v for k,v in result.items() if k not in ('procesos_observados','politicas')}, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    main()
