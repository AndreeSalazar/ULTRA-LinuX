#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Paired before/after process runs; raw samples and separate allocator counts."""
import json
from pathlib import Path
import statistics
import subprocess
import sys

out, sandbox = Path(sys.argv[1]), Path(sys.argv[2])
cases = [('hot', 64, 1000000), ('hot', 4096, 200000), ('cycle', 64, 100000)]
samples, counts, summaries = [], [], []
def run(route, optimization, mode, size, rounds, counter=False):
    name = route+'-'+optimization+('-count' if counter else '')
    p = subprocess.run(['bash', str(sandbox), str(out/name), mode, str(size), str(rounds)],
                       capture_output=True, text=True, timeout=20)
    if p.returncode:
        raise RuntimeError(name+': '+p.stdout+p.stderr)
    line = next(line for line in p.stdout.splitlines() if line.startswith('BENCH '))
    fields = dict(item.split('=', 1) for item in line.split()[1:])
    for key in fields:
        if key != 'mode': fields[key] = int(fields[key])
    if (fields['mode'], fields['size'], fields['rounds']) != (mode, size, rounds):
        raise ValueError('case mismatch')
    fields.update(route=route, optimization=optimization)
    if counter:
        line = next(line for line in p.stdout.splitlines() if line.startswith('ALLOC '))
        fields.update({k: int(v) for k, v in (item.split('=', 1) for item in line.split()[1:])})
    return fields

for optimization in ('O2', 'O3'):
    for mode, size, rounds in cases:
        before_count = run('before', optimization, mode, size, rounds, True)
        after_count = run('after', optimization, mode, size, rounds, True)
        if before_count['checksum'] != after_count['checksum']:
            raise ValueError('counter content mismatch')
        expected_before = rounds+4 if mode == 'hot' else rounds*5
        expected_after = 4 if mode == 'hot' else rounds*4
        if (before_count['calloc'], after_count['calloc']) != (expected_before, expected_after):
            raise ValueError('allocation mechanism differs from hypothesis')
        if (before_count['free_nonnull'], after_count['free_nonnull']) != (expected_before, expected_after):
            raise ValueError('release counts differ from hypothesis')
        counts += [before_count, after_count]
        # Separate-process warmup; not retained as a measured trial.
        for route in ('before', 'after'): run(route, optimization, mode, size, rounds)
        paired = []
        for trial in range(12):
            pair = {}
            for route in (('before', 'after') if trial % 2 == 0 else ('after', 'before')):
                row = run(route, optimization, mode, size, rounds)
                row['trial'] = trial
                if row['wall_ns'] <= 0 or row['cpu_ns'] <= 0:
                    raise ValueError('nonpositive time')
                samples.append(row); pair[route] = row
            if pair['before']['checksum'] != pair['after']['checksum']:
                raise ValueError('paired content mismatch')
            paired.append(100*(1-pair['after']['wall_ns']/pair['before']['wall_ns']))
        summary = dict(optimization=optimization, mode=mode, size=size, rounds=rounds,
                       pairs=12, paired_wall_reduction_pct_median=statistics.median(paired),
                       paired_wall_reduction_pct_min=min(paired), paired_wall_reduction_pct_max=max(paired))
        for route in ('before', 'after'):
            rows = [r for r in samples if (r['route'], r['optimization'], r['mode'], r['size']) == (route, optimization, mode, size)]
            summary[route] = dict(wall_ns_median=statistics.median(r['wall_ns'] for r in rows),
                wall_ns_min=min(r['wall_ns'] for r in rows), wall_ns_max=max(r['wall_ns'] for r in rows),
                cpu_ns_median=statistics.median(r['cpu_ns'] for r in rows),
                rss_peak_kib_max=max(r['rss_peak_kib'] for r in rows))
        summaries.append(summary)
        print(json.dumps(summary), flush=True)

(out/'muestras.json').write_text(json.dumps(samples, indent=2)+'\n')
(out/'asignaciones.json').write_text(json.dumps(counts, indent=2)+'\n')
(out/'resumen.json').write_text(json.dumps(dict(Baseline='cf52e2f', Samples=summaries,
    Scope='QEMU process benchmark including checks and init/teardown, not CPU cycles, IRQ ticks, FPS, native results or statistical proof; RSS peaks not exclusive route memory.'), indent=2)+'\n')
print('BENCH-CUSTODIA-OK: 12 pares alternados/caso; contenido y asignaciones comprobados; conteo separado del tiempo')
