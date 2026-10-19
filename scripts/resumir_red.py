#!/usr/bin/env python3
"""Rates from monotonic sysfs samples; never bridge resets or interface recreation."""
import json
from decimal import Decimal, InvalidOperation
from pathlib import Path
import sys

COUNTERS = ('rx_bytes', 'tx_bytes', 'rx_packets', 'tx_packets',
            'rx_errors', 'tx_errors', 'rx_dropped', 'tx_dropped')


def summarize(text):
    # The report prints its final Ethernet snapshot before the historical log.
    # Only use session samples for rates; do not invent a backwards clock jump.
    if '[MEDICION DURANTE LA SESION:' in text:
        text = text.split('[MEDICION DURANTE LA SESION:', 1)[1].split('[KERNEL:', 1)[0]
    previous = {}
    intervals = []
    rejected = 0
    for line in text.splitlines():
        if not line.startswith('RED iface='):
            continue
        fields = dict(item.split('=', 1) for item in line.split()[1:] if '=' in item)
        try:
            key = (fields['iface'], int(fields['indice']))
            stamp = Decimal(fields['monotonic_s'])
            if not stamp.is_finite() or stamp < 0:
                raise ValueError('clock')
            values = {name: int(fields[name]) for name in COUNTERS}
            if any(v < 0 or v > 2**64-1 for v in values.values()):
                raise ValueError('counter')
        except (KeyError, ValueError, InvalidOperation):
            rejected += 1
            # An unavailable sample breaks continuity; do not hide a reset in it.
            previous.clear()
            continue
        old = previous.get(key)
        previous[key] = (stamp, values)
        if old is None:
            continue
        before, counters = old
        elapsed = stamp - before
        if elapsed <= 0 or any(values[k] < counters[k] for k in COUNTERS):
            rejected += 1
            continue
        delta = {k: values[k] - counters[k] for k in COUNTERS}
        interval = {'interfaz': key[0], 'indice': key[1], 'segundos': float(elapsed),
                    'delta': delta}
        for direction in ('rx', 'tx'):
            interval[direction + '_Mbit_s'] = float(Decimal(delta[direction + '_bytes']) * 8 / elapsed / 1000000)
            interval[direction + '_paquetes_s'] = float(Decimal(delta[direction + '_packets']) / elapsed)
        intervals.append(interval)
    return {'intervalos': intervals, 'discontinuidades_o_muestras_invalidas': rejected,
            'nota': 'Contadores de interfaz, no bytes fisicos en cable ni capacidad maxima. '
                    'GRO/TSO y el driver afectan las cuentas; errores/descartes dependen del punto de medida. '
                    'Muestras no atomicas. En informes se usa solo el historial de la sesion. '
                    'Sin dos muestras compatibles no se calcula tasa.'}


if __name__ == '__main__':
    if len(sys.argv) < 2:
        raise SystemExit('Uso: resumir_red.py informe.txt [...]')
    print(json.dumps({str(p): summarize(p.read_text(encoding='utf-8-sig'))
                      for p in map(Path, sys.argv[1:])}, indent=2, ensure_ascii=False))
