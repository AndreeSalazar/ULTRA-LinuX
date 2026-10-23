#!/usr/bin/env python3
"""Independent known-delta fixtures: pressure, VM, disk units and discontinuities."""
import copy
from recursos_intervalos import MAX_SAMPLES, summarize_resources


def sample(time, *, vm=100, psi=1000, stat=None, complete=True, extra=''):
    if stat is None:
        stat = [10, 0, 8, 20, 4, 0, 16, 40, 7, 100, 200]
    return (f'muestra_utc=2026-10-23T00:00:{time:02}Z\n'
        'ULTRA RECURSOS v1 solo_lectura=1 contadores=acumulados\n'
        f'RELOJ uptime_idle_s={time}.00 0.00\n'
        'MEM MemAvailable: 1024 kB\n'
        f'VM pgmajfault={vm}\nVM pswpout=0\n'
        f'PSI recurso=memory some avg10=0.00 avg60=0.00 avg300=0.00 total={psi}\n'
        'PSI recurso=cpu full avg10=0.00 avg60=0.00 avg300=0.00 total=0\n'
        'BLOQUE dispositivo=nvme0n1 stat='+' '.join(map(str, stat))+'\n'+extra+
        ('fin_recursos snapshot_no_atomico=1 faltante_no_es_cero=1\n' if complete else ''))


first = [10, 0, 8, 20, 4, 0, 16, 40, 7, 100, 200]
last = [12, 0, 16, 30, 5, 0, 32, 60, 0, 600, 2200]
base = 'ULTRA MEDICION v1\n'+sample(10, stat=first)
text = base+sample(15, vm=103, psi=501000, stat=last)
result = summarize_resources(text)
row = result['intervalos'][0]
assert result['muestras'] == result['capturas_completas'] == 2
assert row['duracion_s'] == 5 and row['vm_delta'] == {'pgmajfault': 3, 'pswpout': 0}
assert row['psi_pct'] == {'cpu.full': None, 'memory.some': 10}
disk = row['bloques']['nvme0n1']
assert disk['lectura_bytes_s'] == 819.2 and disk['escritura_bytes_s'] == 1638.4
assert disk['espera_media_lecturas_ms'] == 5 and disk['espera_media_escrituras_ms'] == 20
assert disk['actividad_aproximada_pct'] == 10
assert disk['solicitudes_pendientes_promedio_temporal'] == .4
assert disk['en_vuelo_al_final'] == 0  # decrease of the gauge is valid
assert row['memoria_final']['MemAvailable'] == {'valor': 1024, 'unidad': 'kB'}
print('VERDE: deltas conocidos, sectores de 512 bytes, esperas medias, PSI y gauge decreciente')

row = summarize_resources(base+sample(15, vm=1, psi=0, stat=[0]*11))['intervalos'][0]
assert row['vm_delta']['pgmajfault'] is None and row['vm_delta']['pswpout'] == 0
assert row['psi_pct']['memory.some'] is None
assert row['bloques']['nvme0n1']['estado'] == 'contador_reiniciado_o_dispositivo_cambiado'
for end in (sample(10), sample(9), sample(15, complete=False)):
    assert summarize_resources(base+end)['intervalos'][0]['estado'] != 'calculado'
assert summarize_resources(base+'fin_medicion muestras=1\nULTRA MEDICION v1\n'+sample(15))['intervalos'][0]['estado'] == 'sesiones_distintas'
assert summarize_resources('kernel=legacy\nRAM total_kB=10\n')['muestras'] == 0
print('VERDE: reinicios, cero real, reloj detenido/inverso, captura incompleta y sesiones separadas')

row = summarize_resources(base+sample(15).replace('VM pgmajfault=100\n', ''))['intervalos'][0]
assert row['vm_delta']['pgmajfault'] is None
row = summarize_resources(base+sample(15, extra='VM pgmajfault=123\n'))['intervalos'][0]
assert row['vm_delta']['pgmajfault'] is None
row = summarize_resources(base+sample(15, stat=first+[0]*4))['intervalos'][0]
assert row['bloques']['nvme0n1']['estado'] == 'ausente_o_formato_cambiado'
row = summarize_resources(base+sample(15, stat=first, psi=6_001_000))['intervalos'][0]
assert row['psi_pct']['memory.some'] is None
zero = summarize_resources(base+sample(15, stat=first))['intervalos'][0]['bloques']['nvme0n1']
assert zero['lectura_bytes_s'] == 0 and zero['espera_media_lecturas_ms'] is None
changed = copy.copy(last); changed[9] = 99999
assert summarize_resources(base+sample(15, stat=changed))['intervalos'][0]['bloques']['nvme0n1']['actividad_aproximada_pct'] is None
for count in (15, 17):
    old = first+[0]*(count-11); new = last+[0]*(count-11)
    assert summarize_resources(sample(10, stat=old)+sample(15, stat=new))['intervalos'][0]['bloques']['nvme0n1']['lectura_bytes_s'] == 819.2
print('VERDE: campos ausentes/duplicados, formatos 11/15/17, PSI fuera de intervalo, sin falsa espera cero')
for extra in ('VM NA\nVM pgmajfault=123\n', 'RELOJ uptime_idle_s=15.00 0\n', 'MEM NA\n',
              'PSI recurso=memory some avg10=0.00 avg60=0.00 avg300=0.00 total=5 total=6\n'):
    assert summarize_resources(base+sample(15, extra=extra))['intervalos'][0]['estado'] == 'captura_incompleta_o_ambigua'
row = summarize_resources(base+sample(15, extra='PSI recurso=memory NA\n'))['intervalos'][0]
assert row['psi_pct']['memory.some'] is None

text = base+sample(15, extra='x'*4097+'\n')
assert summarize_resources(text)['intervalos'][0]['estado'] == 'captura_incompleta_o_ambigua'
try:
    summarize_resources('muestra_utc=x\n'*(MAX_SAMPLES+1))
except ValueError:
    pass
else:
    raise AssertionError('sample flood accepted')
print('VERDE: lineas excesivas y limite de muestras; analisis no modifica archivos ni parametros')
