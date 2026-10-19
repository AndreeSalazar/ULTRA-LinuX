#!/usr/bin/env python3
"""Summarize live samples; do not convert missing measurements into zero."""
import csv
import json
import pathlib
import re
import sys

def summarize(path):
    text = path.read_text(errors='replace')
    def field(name):
        match = re.search(rf'^{name}=(.+)$', text, re.M)
        return match.group(1).strip() if match else None
    def bounds(values):
        return {'min': min(values), 'max_muestreado': max(values), 'muestras': len(values)} if values else None
    ram = [int(value) / 1024 for value in re.findall(r'^RAM .*usada_estimada_kB=(\d+)', text, re.M)]
    cpu = []
    previous = None
    for counters in re.findall(r'^CPU_contadores cpu\s+([0-9 ]+)$', text, re.M):
        values = [int(value) for value in counters.split()[:8]]
        if len(values) < 8:
            continue
        current = (sum(values), values[3] + values[4])
        if previous is not None:
            elapsed, idle = current[0] - previous[0], current[1] - previous[1]
            if elapsed > 0 and 0 <= idle <= elapsed:
                cpu.append(100 * (elapsed - idle) / elapsed)
        previous = current
    # Only complete UID snapshots: incomplete readings remain in raw evidence.
    pss = []
    for count, measured, truncated, value in re.findall(r'^UID1000 procesos=(\d+) medidos=(\d+) truncado=(\d+) .*pss_suma_parcial_kB=(\d+)', text, re.M):
        if count == measured and truncated == '0' and int(count) > 0:
            pss.append(int(value) / 1024)
    gpus = {}
    for line in text.splitlines():
        if not line.startswith('GPU GPU-'):
            continue
        row = next(csv.reader([line[4:]], skipinitialspace=True))
        if len(row) != 7:
            continue
        gpu = gpus.setdefault(row[0], {'pci': row[1], 'vram_MiB': [], 'uso_pct': [], 'temperatura_C': [], 'potencia_W': []})
        for name, index in [('vram_MiB', 3), ('uso_pct', 4), ('temperatura_C', 5), ('potencia_W', 6)]:
            try:
                value = float(row[index])
                if value >= 0 and value < float('inf'):
                    gpu[name].append(value)
            except ValueError:
                pass
    for gpu in gpus.values():
        for name in ('vram_MiB', 'uso_pct', 'temperatura_C', 'potencia_W'):
            gpu[name] = bounds(gpu[name])
    return {'archivo': path.name, 'kernel': field('kernel'), 'modo': field('modo'),
            'compositor': field('compositor'), 'fecha_utc': field('fecha_utc'),
            'ram_sistema_estimada_MiB': bounds(ram), 'pss_UID1000_MiB': bounds(pss),
            'cpu_sistema_ocupada_pct': bounds(cpu),
            'gpu': gpus or None,
            'alcance': 'Maximos muestreados; no picos absolutos ni memoria exclusiva de Hyprland. VRAM total del dispositivo.'}

if __name__ == '__main__':
    folder = pathlib.Path(sys.argv[1])
    # Prefer mode-specific copies to avoid counting the legacy alias twice.
    variants = sorted(folder.glob('gpu-*-*-hyprland.txt')) + sorted(folder.glob('gpu-*-*-cage.txt'))
    files = variants or sorted(p for p in folder.glob('gpu-*.txt') if not p.name.startswith('gpu-fallo-'))
    print(json.dumps([summarize(path) for path in files], ensure_ascii=False, indent=2))
