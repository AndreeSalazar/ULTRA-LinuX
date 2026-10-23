"""Read-only interpretation of bounded ULTRA resource snapshots, never tuning."""
import re
import io

MAX_SAMPLES = 512
MAX_COUNTER = (1 << 64) - 1


def integer(value):
    if not re.fullmatch(r'[0-9]{1,20}', value):
        return None
    number = int(value)
    return number if number <= MAX_COUNTER else None


def uptime_us(value):
    match = re.fullmatch(r'([0-9]{1,12})(?:\.([0-9]{1,6}))?', value)
    if not match:
        return None
    return int(match[1])*1_000_000 + int((match[2] or '').ljust(6, '0') or '0')


def put_unique(mapping, key, value, limit):
    if key in mapping:
        mapping[key] = None  # ambiguous input is unavailable
    elif len(mapping) < limit:
        mapping[key] = value


def parse(text):
    samples, current, session, in_resources = [], None, 0, False
    for line in io.StringIO(text):
        line = line.rstrip('\r\n')
        if len(line) > 4096:
            if current is not None:
                current['malformado'] = True
            continue
        if line == 'ULTRA MEDICION v1' or line.startswith('fin_medicion '):
            session += 1
            current, in_resources = None, False
        elif line.startswith('muestra_utc='):
            if len(samples) >= MAX_SAMPLES:
                raise ValueError('demasiadas muestras: limite 512')
            current = dict(session=session, fecha_utc=line[12:], uptime=None,
                           vm={}, psi={}, blocks={}, memory={}, completo=False,
                           malformado=False, bloques_recursos=0, reloj_visto=False, vm_na=False, mem_na=False)
            samples.append(current)
            in_resources = False
        elif current is None:
            continue
        elif line.startswith('ULTRA RECURSOS v1 '):
            current['bloques_recursos'] += 1
            current['malformado'] |= current['bloques_recursos'] != 1
            in_resources = True
        elif line.startswith('fin_recursos '):
            current['completo'] = in_resources
            in_resources = False
        elif not in_resources:
            continue
        elif line.startswith('RELOJ uptime_idle_s='):
            tokens = line[len('RELOJ uptime_idle_s='):].split()
            if current['reloj_visto']:
                current['malformado'] = True
            current['reloj_visto'] = True
            current['uptime'] = uptime_us(tokens[0]) if tokens else None
        elif line.startswith('VM '):
            match = re.fullmatch(r'VM ([a-zA-Z0-9_]{1,80})=([0-9]+)', line)
            if match:
                current['malformado'] |= current['vm_na']
                put_unique(current['vm'], match[1], integer(match[2]), 256)
            elif line != 'VM NA':
                current['malformado'] = True
            else:
                current['malformado'] |= bool(current['vm'])
                current['vm_na'] = True
                current['vm'].clear()
        elif line.startswith('MEM '):
            match = re.fullmatch(r'MEM ([a-zA-Z0-9_]{1,80}): ([0-9]+) *(kB)?', line)
            if match:
                current['malformado'] |= current['mem_na']
                put_unique(current['memory'], match[1],
                           dict(valor=integer(match[2]), unidad=match[3]), 64)
            elif line == 'MEM NA':
                current['malformado'] |= bool(current['memory'])
                current['mem_na'] = True
                current['memory'].clear()
            else:
                current['malformado'] = True
        elif line.startswith('PSI '):
            match = re.fullmatch(r'PSI recurso=(cpu|memory|io) (some|full) '
                r'avg10=[0-9]{1,6}\.[0-9]{1,6} avg60=[0-9]{1,6}\.[0-9]{1,6} '
                r'avg300=[0-9]{1,6}\.[0-9]{1,6} total=([0-9]+)', line)
            if match:
                put_unique(current['psi'], match[1]+'.'+match[2], integer(match[3]), 6)
            else:
                absent = re.fullmatch(r'PSI recurso=(cpu|memory|io) NA', line)
                if absent:
                    for kind in ('some', 'full'):
                        put_unique(current['psi'], absent[1]+'.'+kind, None, 6)
                else:
                    current['malformado'] = True
        elif line.startswith('BLOQUE dispositivo='):
            match = re.fullmatch(r'BLOQUE dispositivo=([a-zA-Z0-9_.-]{1,80}) stat=(.+)', line)
            if match:
                values = [integer(v) for v in match[2].split()]
                stat = values if len(values) in (11, 15, 17) and None not in values else None
                put_unique(current['blocks'], match[1], stat, 32)
    return samples


def delta(before, after):
    return after-before if before is not None and after is not None and after >= before else None


def block_interval(before, after, duration_us):
    if before is None or after is None or len(before) != len(after):
        return dict(estado='ausente_o_formato_cambiado')
    # in_flight (index 8) is a gauge, not a cumulative counter.
    changes = [delta(a, b) if i != 8 else None for i, (a, b) in enumerate(zip(before, after))]
    if any(d is None for i, d in enumerate(changes) if i != 8):
        return dict(estado='contador_reiniciado_o_dispositivo_cambiado')
    seconds = duration_us/1_000_000
    active = changes[9]*100_000/duration_us
    return dict(estado='calculado_identidad_estable_no_verificada',
        lectura_bytes_s=changes[2]*512/seconds, escritura_bytes_s=changes[6]*512/seconds,
        lecturas_completadas=changes[0], escrituras_completadas=changes[4],
        espera_media_lecturas_ms=changes[3]/changes[0] if changes[0] else None,
        espera_media_escrituras_ms=changes[7]/changes[4] if changes[4] else None,
        actividad_aproximada_pct=active if active <= 100 else None,
        actividad_fuera_de_intervalo=active > 100,
        solicitudes_pendientes_promedio_temporal=changes[10]*1000/duration_us,
        en_vuelo_al_final=after[8])


def summarize_resources(text):
    samples = parse(text)
    intervals = []
    for before, after in zip(samples, samples[1:]):
        row = dict(desde_utc=before['fecha_utc'], hasta_utc=after['fecha_utc'])
        intervals.append(row)
        if before['session'] != after['session']:
            row['estado'] = 'sesiones_distintas'
            continue
        if any(not s['completo'] or s['malformado'] for s in (before, after)):
            row['estado'] = 'captura_incompleta_o_ambigua'
            continue
        elapsed = delta(before['uptime'], after['uptime'])
        if not elapsed:
            row['estado'] = 'reloj_ausente_reiniciado_o_sin_avance'
            continue
        row.update(estado='calculado', duracion_s=elapsed/1_000_000,
                   vm_delta={}, psi_pct={}, bloques={}, memoria_final=after['memory'])
        for key in sorted(before['vm'].keys() | after['vm'].keys()):
            row['vm_delta'][key] = delta(before['vm'].get(key), after['vm'].get(key))
        for key in sorted(before['psi'].keys() | after['psi'].keys()):
            change = delta(before['psi'].get(key), after['psi'].get(key))
            # CPU full is undefined for system-wide PSI; never infer no CPU stalls.
            row['psi_pct'][key] = (100*change/elapsed
                if key != 'cpu.full' and change is not None and change <= elapsed else None)
        for key in sorted(before['blocks'].keys() | after['blocks'].keys()):
            row['bloques'][key] = block_interval(before['blocks'].get(key), after['blocks'].get(key), elapsed)
    return dict(schema=1, muestras=len(samples),
        capturas_completas=sum(s['completo'] and not s['malformado'] for s in samples),
        intervalos=intervals,
        alcance='Contadores de sistema entre capturas no atomicas; no sumar capas de disco. '
                'Sin atribucion a juegos, latencia por peticion, p99 ni prueba de saturacion. '
                'Faults no equivalen a UB. Ausente/reinicio es null, no cero.')
