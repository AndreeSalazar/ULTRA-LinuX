#!/usr/bin/python
# SPDX-License-Identifier: GPL-2.0-only
"""Sensores etiquetados: nunca inferir temperatura CPU desde GPU o NVMe."""
from pathlib import Path


def leer(path):
    try:
        with path.open() as f:
            return f.read(128).strip()
    except (OSError, ValueError):
        return ''


def temperaturas(sysroot=Path('/sys')):
    grupos = {'cpu': [], 'gpu': [], 'disco': []}
    for chip in sorted((sysroot / 'class/hwmon').glob('hwmon*'))[:32]:
        nombre = leer(chip / 'name')
        grupo = ('cpu' if nombre in ('k10temp', 'coretemp', 'zenpower') else
                 'gpu' if nombre in ('amdgpu', 'nouveau') else
                 'disco' if nombre in ('nvme', 'drivetemp') else None)
        if grupo is None:
            continue
        for sensor in sorted(chip.glob('temp*_input'))[:16]:
            try:
                valor = int(leer(sensor)) / 1000
            except ValueError:
                continue
            if not -40 <= valor <= 150:
                continue
            etiqueta = leer(sensor.with_name(sensor.name.replace('_input', '_label'))) or sensor.stem
            grupos[grupo].append({'chip': nombre, 'sensor': etiqueta[:64], 'C': round(valor, 1)})
    # Tctl es la referencia de control AMD. Conservar tambien Tdie/CCD identificados.
    control = [s for s in grupos['cpu'] if s['sensor'] == 'Tctl'] or grupos['cpu']
    cpu = max(control, key=lambda s: s['C']) if control else None
    return {'cpu_C': cpu['C'] if cpu else None,
            'cpu_sensor': f"{cpu['chip']}/{cpu['sensor']}" if cpu else None,
            'gpu_C': max((s['C'] for s in grupos['gpu']), default=None),
            'disco_C': max((s['C'] for s in grupos['disco']), default=None),
            'sensores': [s for values in grupos.values() for s in values]}
