#!/usr/bin/env python3
from resumir_red import COUNTERS, summarize


def sample(time, number, index=3, **changes):
    fields = dict.fromkeys(COUNTERS, number)
    fields.update(changes)
    return f'RED iface=eth0 indice={index} monotonic_s={time} ' + ' '.join(f'{k}={v}' for k, v in fields.items())


data = summarize(sample('10.5', 100) + '\n' + sample('12.5', 1100))
interval, = data['intervalos']
assert interval['rx_Mbit_s'] == 0.004 and interval['tx_paquetes_s'] == 500
assert interval['delta']['rx_dropped'] == 1000
assert not summarize(sample('1', 100))['intervalos']
assert not summarize(sample('1', 100) + '\n' + sample('1', 200))['intervalos']
assert not summarize(sample('1', 100) + '\n' + sample('2', 1))['intervalos']
assert not summarize(sample('1', 100) + '\n' + sample('2', 200, index=4))['intervalos']
for invalid in ('NA', '-1', str(2**64), 'no-numero'):
    result = summarize(sample('1', 100) + '\n' + sample('2', 200, rx_bytes=invalid) + '\n' + sample('3', 300))
    assert not result['intervalos'] and result['discontinuidades_o_muestras_invalidas'] == 1
assert not summarize(sample('NaN', 1))['intervalos']
report = (sample('99', 99999) + '\n[MEDICION DURANTE LA SESION: historial]\n' +
          sample('1', 100) + '\n' + sample('2', 200) + '\n[KERNEL: fin]\n' + sample('3', 300))
assert len(summarize(report)['intervalos']) == 1
assert summarize(report)['discontinuidades_o_muestras_invalidas'] == 0
print('VERDE [RED-RESUMEN]: tasas, tiempos, reset, recreacion de interfaz y NA')
