#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Validate and render a living UB catalog; never execute exhibit snippets."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re

CLASSES = {'UB', 'DEFINIDO', 'NO_ESPECIFICADO', 'IMPLEMENTACION'}

def inside(root, relative):
    path = (root / relative).resolve()
    if not path.is_relative_to(root) or not path.is_file():
        raise ValueError(f'archivo ausente o fuera del proyecto: {relative}')
    return path

def render(root, report_path):
    registry = inside(root, 'docs/museo-ub.json')
    data = json.loads(registry.read_text(encoding='utf-8'))
    if data['version'] != 1:
        raise ValueError('version de catalogo no soportada')
    source = inside(root, 'src/guardian/laboratorio/minas.c').read_text(encoding='utf-8')
    runtime = set(re.findall(r'^\s*\{"([^"\n]+)",\s*\'[DN]\'', source, re.M))
    if not runtime:
        raise ValueError('inventario de minas vacio')
    report = inside(root, report_path).read_text(encoding='utf-8')
    summaries = re.findall(r'^RESUMEN ejecucion:\s+VERDE=(\d+) NEUTRA=(\d+) AMARILLO=(\d+) ROJO=(\d+) \(de (\d+)\)$', report, re.M)
    observations = {}
    for state, name in re.findall(r'^(VERDE|NEUTRA|AMARILLO|ROJO)\s+([^:\n]+):', report, re.M):
        if name in runtime:
            if name in observations:
                raise ValueError(f'evidencia duplicada: {name}')
            observations[name] = state
    if len(summaries) != 1 or int(summaries[0][4]) != len(runtime) or set(observations) != runtime:
        raise ValueError('evidencia incompleta o desactualizada: repetir el campo de minas')
    actual_counts = Counter(observations.values())
    if [actual_counts[s] for s in ('VERDE', 'NEUTRA', 'AMARILLO', 'ROJO')] != [int(n) for n in summaries[0][:4]]:
        raise ValueError('resumen de ejecucion inconsistente con sus casos')
    fixtures = {f.stem for f in (root / 'src/guardian/laboratorio/minas-compilacion').glob('*.c')}
    compile_part = report.split('== Minas de ejecucion:', 1)[0]
    compile_pairs = re.findall(r'^(VERDE|ROJO)\s+([^:\n]+):', compile_part, re.M)
    compile_states = {name: state for state, name in compile_pairs}
    if set(compile_states) != fixtures or len(compile_pairs) != len(compile_states):
        raise ValueError('evidencia de compilacion incompleta o desactualizada')
    compile_summary = re.findall(r'^RESUMEN compilacion: VERDE=(\d+) ROJO=(\d+)$', report, re.M)
    compile_counts = Counter(compile_states.values())
    if len(compile_summary) != 1 or [int(n) for n in compile_summary[0]] != [compile_counts['VERDE'], compile_counts['ROJO']]:
        raise ValueError('resumen de compilacion inconsistente con sus casos')
    ids, mapped = set(), set()
    for card in data['piezas']:
        identifier = card['id']
        if not re.fullmatch(r'(?:UB|NOUB)-\d{3}', identifier) or identifier in ids:
            raise ValueError(f'identificador invalido o duplicado: {identifier}')
        ids.add(identifier)
        if card['clase'] not in CLASSES or (identifier.startswith('UB-') != (card['clase'] == 'UB')):
            raise ValueError(f'clasificacion inconsistente: {identifier}')
        if card['fuente'] not in data['fuentes']:
            raise ValueError(f'fuente desconocida: {identifier}')
        for key in ('titulo', 'sala', 'causa', 'mal', 'alternativa', 'regla', 'limite', 'clausula'):
            if not isinstance(card[key], str) or not card[key].strip():
                raise ValueError(f'falta {key} en {identifier}')
        for name in card['runtime']:
            if name not in runtime:
                raise ValueError(f'mina inexistente: {identifier}: {name}')
            mapped.add(name)
        for name in card['compilacion']:
            if not re.fullmatch(r'[a-z0-9-]+', name):
                raise ValueError('nombre de fixture invalido')
            inside(root, f'src/guardian/laboratorio/minas-compilacion/{name}.c')
        for evidence in card.get('evidencia_externa', []):
            inside(root, evidence)
        if card.get('fixture_externo'):
            inside(root, card['fixture_externo'])
    counts = Counter(c['clase'] for c in data['piezas'])
    lines = ['# Museo viviente de C y sus guardianes', '',
             'Generado desde `museo-ub.json`; las fichas se estudian y revisan, no se ejecutan desde este documento.', '',
             f"Dialectos: {data['dialecto']}.", '',
             f"Coleccion inicial: **{len(ids)} piezas** ({counts['UB']} de UB y {len(ids) - counts['UB']} de contraste). "
             f'Enlaza {len(mapped)} de las {len(runtime)} minas actuales; una pieza no equivale a una sola mina.', '',
             'Los fragmentos son originales y didacticos: algunos presuponen declaraciones o contexto. '
             'La alternativa puede ser una pauta de diseno, no un reemplazo completo compilable. '
             'Una alternativa valida puede rechazar una operacion imposible para el tipo; no implica obtener un resultado numerico.', '',
             '## Como leer el museo', '',
             '- ESTUDIADA / SIN MINA: hay explicacion y referencia; no hay deteccion acreditada.',
             '- VINCULADA: se muestran resultados de casos concretos, sin generalizarlos a toda la familia.',
             '- REVISION: el fixture existe; esta pagina no convierte su presencia en prueba de rechazo.',
             '- Las piezas NOUB distinguen reglas del lenguaje de politicas mas estrictas del proyecto.', '',
             'No es un catalogo completo de la norma ni un modelo que aprenda permisos automaticamente. '
             'Una extension de WG14 o GCC no se convierte por ello en una regla ISO C17. '
             'Los estados pertenecen al informe enlazado: generar esta pagina no vuelve a ejecutar las minas '
             'ni demuestra que un binario posterior conserve esos resultados.', '',
             '## Indice', '', '| Pieza | Sala | Clase | Vinculo |', '|---|---|---|---|']
    for c in data['piezas']:
        status = 'VINCULADA' if c['runtime'] or c['compilacion'] or c.get('evidencia_externa') else 'ESTUDIADA / SIN MINA'
        lines.append(f"| [{c['id']} — {c['titulo']}](#{c['id'].lower()}) | {c['sala']} | {c['clase']} | {status} |")
    for c in data['piezas']:
        ref = data['fuentes'][c['fuente']]
        lines += ['', f"## {c['id']}", '', f"**{c['titulo']}** — {c['clase']}", '', c['causa'], '',
                  'Fragmento de estudio (ver las precondiciones y la clase de la pieza):', '', '```c', c['mal'], '```', '',
                  'Alternativa o pauta valida:', '', '```c', c['alternativa'], '```', '',
                  f"Regla del guardian: {c['regla']}", '', f"Limite: {c['limite']}", '',
                  f"Referencia: [{ref['titulo']}]({ref['url']}), {c['clausula']}.", '']
        for name in c['runtime']:
            lines.append(f'- Caso `{name}`: **{observations[name]}** en [{report_path}](../{report_path}).')
        for name in c['compilacion']:
            lines.append(f'- REVISION: [{name}.c](../src/guardian/laboratorio/minas-compilacion/{name}.c): **{compile_states[name]}** en el informe; no equivale a deteccion en produccion.')
        for evidence in c.get('evidencia_externa', []):
            lines.append(f'- Evidencia externa (ver su alcance propio): [{evidence}](../{evidence}).')
        if not c['runtime'] and not c['compilacion'] and not c.get('evidencia_externa'):
            lines.append('**ESTUDIADA / SIN MINA:** no hay deteccion experimental acreditada para esta pieza.')
    lines += ['', '## Lo que aun no tiene vitrina propia', '',
              'Estas minas existen en el campo, pero esta coleccion inicial no las explica por separado:', '',
              ', '.join(f'`{name}`' for name in sorted(runtime - mapped)) + '.', '',
              '## Fuentes y mantenimiento', '']
    for ref in data['fuentes'].values():
        if not ref['url'].startswith('https://'):
            raise ValueError('fuente no HTTPS')
        lines.append(f"- [{ref['titulo']}]({ref['url']}).")
    digest = hashlib.sha256(report.encode('utf-8')).hexdigest()
    lines += ['', 'J.2 es un indice informativo; la explicacion debe remitirse a la clausula correspondiente. '
              'N3155 es material educativo con TODOs, no una certificacion de exhaustividad ni la norma C17.', '',
              f'Evidencia de ejecucion: `{report_path}`; SHA-256 del texto normalizado a LF: `{digest}`.', '',
              'Para ampliar: editar el JSON, justificar la regla, vincular un fixture y un control valido, '
              'ejecutar pruebas aisladas y regenerar. Una pieza pendiente no se pinta de verde.', '',
              'Con Python 3.10 o posterior, desde el proyecto:', '', '```text',
              f'python scripts/museo-ub.py --evidencia {report_path}',
              f'python scripts/museo-ub.py --evidencia {report_path} --check', '```', '']
    return '\n'.join(lines), len(ids), len(runtime), len(mapped)

def main():
    p = argparse.ArgumentParser()
    p.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    p.add_argument('--evidencia', default='evidencias/campo-minas-ronda6.txt')
    p.add_argument('--check', action='store_true')
    a = p.parse_args()
    root = a.root.resolve()
    text, cards, mines, mapped = render(root, a.evidencia)
    output = root / 'docs/MUSEO-UB.md'
    if a.check:
        if not output.is_file() or output.read_text(encoding='utf-8') != text:
            raise SystemExit('ROJO: museo desactualizado; regenerar con la evidencia elegida')
    else:
        output.write_text(text, encoding='utf-8', newline='\n')
    print(f'MUSEO-OK: {cards} piezas; {mapped}/{mines} minas con vitrina; enlaces y evidencia comprobados; ningun fragmento ejecutado')

if __name__ == '__main__':
    main()
