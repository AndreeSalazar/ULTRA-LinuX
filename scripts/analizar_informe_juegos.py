#!/usr/bin/env python3
"""Summarize exported measurements without exporting accounts or raw logs."""
import argparse
import csv
import hashlib
import json
from pathlib import Path
import re
import shutil


def analyze(raw):
    text = raw.decode('utf-8-sig', errors='replace')
    ram, gpu, network, games, processes = [], [], [], [], set()
    traffic = {}
    storage = None
    for line in text.splitlines():
        if line.startswith('RAM total_kB='):
            values = dict(re.findall(r'(\w+)=(\d+)', line))
            if 'usada_estimada_kB' in values:
                ram.append(int(values['usada_estimada_kB']))
        elif line.startswith('GPU GPU-'):
            row = next(csv.reader([line[4:]], skipinitialspace=True))
            try:
                if len(row) == 7:
                    gpu.append({'vram_MiB':float(row[3]), 'uso_pct':float(row[4]),
                                'temperatura_C':float(row[5])})
            except ValueError:
                pass
        elif line.startswith('RED iface='):
            values = dict(re.findall(r'(\w+)=([^ ]+)', line))
            if values.get('speed_Mbps', '').isdigit():
                network.append(int(values['speed_Mbps']))
            try:
                sample = (float(values['monotonic_s']), int(values['rx_bytes']), int(values['tx_bytes']))
                traffic.setdefault(values['iface'], []).append(sample)
            except (KeyError, ValueError):
                pass
        elif line.startswith('PROCESO '):
            match = re.search(r' nombre=([^ ]+)', line)
            if match:
                processes.add(match[1])
        match = re.match(r'^/\S+\s+(\S+)\s+(\S+)\s+(\S+)\s+(\d+%)\s+/juegos\s*$', line)
        if match:
            storage = dict(zip(('capacidad','usado','disponible','porcentaje'), match.groups()))
    section = text.split('[REGISTRO DE BIBLIOTECAS STEAM: sin cuentas o sesiones]', 1)
    if len(section) == 2:
        active = False
        for line in section[1].splitlines():
            if line.startswith('['):
                break
            if line.startswith('STEAM: biblioteca '):
                active = True
            elif line.startswith('STEAM:'):
                active = False
            elif active and line.startswith('  '):
                name = line.strip()
                if name and name not in games:
                    games.append(name)
    transfers = {}
    for interface, samples in traffic.items():
        samples = sorted(set(samples))
        rx, tx, seconds, peak = 0, 0, 0.0, 0.0
        for before, after in zip(samples, samples[1:]):
            elapsed = after[0]-before[0]
            incoming, outgoing = after[1]-before[1], after[2]-before[2]
            if elapsed <= 0 or incoming < 0 or outgoing < 0:
                continue
            rx += incoming; tx += outgoing; seconds += elapsed
            peak = max(peak, incoming/elapsed/1048576)
        transfers[interface] = {'recibido_intervalos_GiB':round(rx/1073741824, 3),
                                'enviado_intervalos_GiB':round(tx/1073741824, 3),
                                'intervalos_s':round(seconds, 3), 'pico_rx_intervalo_MiB_s':round(peak, 3)}
    return {'informe_sha256':hashlib.sha256(raw).hexdigest(), 'almacenamiento':storage,
            'juegos_en_registro':games, 'muestras_RAM':len(ram), 'muestras_GPU':len(gpu),
            'RAM_max_GiB':round(max(ram)/1048576, 3) if ram else None,
            'VRAM_max_MiB':max((g['vram_MiB'] for g in gpu), default=None),
            'GPU_uso_max_pct':max((g['uso_pct'] for g in gpu), default=None),
            'GPU_temperatura_max_C':max((g['temperatura_C'] for g in gpu), default=None),
            'enlaces_Mbps_observados':sorted(set(network)),
            'trafico_por_interfaz':transfers,
            'ejecutable_warframe_observado':any(p.lower().startswith('warframe.x64') for p in processes),
            'wine_observado':any(p.lower().startswith(('wine','wineserver')) for p in processes),
            'avisos':{'pixman_rectangulo_invalido':'In pixman_region32_init_rect: Invalid rectangle passed' in text,
                      'kernel_Xid_observado':bool(re.search(r'NVRM: Xid', text)),
                      'sin_espacio_observado':'No space left on device' in text},
            'limites':['Picos de muestras; no maximos continuos ni FPS.',
                       'Proceso presente no acredita partida, renderizado correcto o estabilidad.',
                       'El informe no desglosa que carpetas consumieron el espacio.',
                       'El trafico es de la interfaz completa, no se atribuye a Warframe.',
                       'Ausencia de error en este informe no prueba ausencia global de fallos.']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('informe', type=Path)
    parser.add_argument('--salida', type=Path, required=True)
    parser.add_argument('--imagen-host', type=Path)
    args = parser.parse_args()
    with args.informe.open('rb') as stream:
        raw = stream.read(16*1024*1024+1)
    if len(raw) > 16*1024*1024:
        raise ValueError('informe demasiado grande')
    result = analyze(raw)
    if args.imagen_host:
        result['imagen_host_GiB'] = args.imagen_host.stat().st_size/1073741824
        result['host_libre_GiB'] = round(shutil.disk_usage(args.imagen_host.parent).free/1073741824, 3)
    args.salida.mkdir(parents=True, exist_ok=True)
    (args.salida/'resumen.json').write_text(json.dumps(result, ensure_ascii=False, indent=2)+'\n', encoding='utf-8', newline='\n')
    disk = result['almacenamiento'] or {}
    lines = ['# Sesion fisica: Steam y recursos', '',
             f'Biblioteca registrada: {len(result["juegos_en_registro"])} juegos encontrados.',
             'Juegos: '+', '.join(result['juegos_en_registro'])+'.', '',
             '| Medida observada | Valor |', '|---|---|',
             f'| RAM maxima muestreada del sistema | {result["RAM_max_GiB"]} GiB |',
             f'| VRAM maxima muestreada | {result["VRAM_max_MiB"]} MiB |',
             f'| Uso GPU maximo muestreado | {result["GPU_uso_max_pct"]}% |',
             f'| Temperatura GPU maxima muestreada | {result["GPU_temperatura_max_C"]} C |',
             f'| Espacio Linux al cerrar | {disk.get("disponible", "NA")} libres / {disk.get("capacidad", "NA")} |',
             f'| Velocidades de enlace observadas | {result["enlaces_Mbps_observados"]} Mbps |', '',
             'Warframe aparece como proceso: '+str(result['ejecutable_warframe_observado'])+'.',
             'Esto acredita inicio del ejecutable; no confirma partida ni estabilidad.', '',
             '## Lo que falta', '',
             '1. Revisar el desglose de almacenamiento antes de borrar caches o ampliar el ext4; ver seguimiento.md.',
             '2. Confirmar una partida y verificar los archivos desde Steam con la compatibilidad elegida.',
             '3. Reproducir el aviso de rectangulo invalido de pixman y distinguirlo del cierre normal del escritorio.',
             '4. Medir tiempos de fotograma y una carga repetible antes de cambiar compilacion o graficos.', '']
    for interface, transfer in result['trafico_por_interfaz'].items():
        lines += [f'Red {interface}: {transfer["recibido_intervalos_GiB"]} GiB recibidos en '
                  f'{transfer["intervalos_s"]} s medidos; pico medio por intervalo '
                  f'{transfer["pico_rx_intervalo_MiB_s"]} MiB/s. No se atribuye a una aplicacion concreta.', '']
    if 'host_libre_GiB' in result:
        lines += [f'Imagen persistente actual: {result["imagen_host_GiB"]} GiB; libres en su disco de Windows: {result["host_libre_GiB"]} GiB.',
                  'Ampliarla de 16 a 32 GiB requiere 16 GiB adicionales, mas margen. No se amplio ni se borro nada.', '']
    lines += ['## Limites de la medida', '']+[f'- {v}' for v in result['limites']]
    (args.salida/'README.md').write_text('\n'.join(lines)+'\n', encoding='utf-8', newline='\n')
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    main()
