#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Render the C/C++/ASM research index. Never compile or run its proposed cases."""
import argparse
from collections import Counter
import json
from pathlib import Path
import re
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[1]
CLASSES = {'C': {'UB', 'DEFINIDO', 'NO_ESPECIFICADO', 'IMPLEMENTACION'},
           'C++': {'UB', 'DEFINIDO', 'NO_ESPECIFICADO', 'IMPLEMENTACION'},
           'ASM': {'ABI', 'ISA', 'CONTRATO_GCC', 'CONCURRENCIA', 'CONTRASTE'}}
PREFIX = {'C':'C', 'C++':'CXX', 'ASM':'ASM'}
FIELDS = {'id','lenguaje','clase','objetivo','titulo','regla','control','prueba','limite','fuente','seccion','estado'}


def render(data):
    if data.get('version') != 1 or not isinstance(data.get('piezas'), list) or not data['piezas']:
        raise ValueError('catalogo/version invalido')
    seen = set()
    for card in data['piezas']:
        if (not isinstance(card, dict) or set(card) != FIELDS
                or any(not isinstance(v,str) or not v.strip() for v in card.values())):
            raise ValueError('ficha incompleta o campos inesperados')
        language = card['lenguaje']
        if language not in CLASSES or card['clase'] not in CLASSES[language]:
            raise ValueError('clase no permitida para el lenguaje')
        if not re.fullmatch(PREFIX[language]+r'-[0-9]{3}', card['id']) or card['id'] in seen:
            raise ValueError('ID invalido o duplicado')
        seen.add(card['id'])
        if card['estado'] != 'ESTUDIADA':
            raise ValueError('este catalogo no acredita ejecuciones: se requiere evidencia en otro banco')
        url = urlsplit(card['fuente'])
        if url.scheme != 'https' or not url.hostname or url.username or url.password:
            raise ValueError('fuente primaria HTTPS requerida')
    counts = Counter(c['lenguaje'] for c in data['piezas'])
    lines = ['# Museo de C, C++ y ASM', '',
             'Entrada comun de investigacion. Generada desde `catalogo.json`; no mueve ni modifica el sistema de juegos.', '',
             '[Plan de las siete areas: motivos, pruebas, medidas y criterios de cierre](../docs/PLAN-MEJORAS-SISTEMAS.md).', '',
             '## Mapa de carpetas', '',
             '| Area | Donde estudiar y comprobar |', '|---|---|',
             '| C existente | [24 fichas y sus evidencias](../docs/MUSEO-UB.md), [cobertura](../docs/COBERTURA-UB.md), [laboratorio](../src/guardian/laboratorio/) |',
             '| Ampliacion C, C++ y ASM | Este indice y su catalogo; propuestas pendientes de fixtures |',
             '| C++ propio | [Produccion y laboratorio separados](../C++/README.md) |',
             '| ASM | [Alcance x86-64 y reglas](../ASM/README.md) |',
             '| Generar y validar | `python scripts/museo-sistemas.py`; `--comprobar` exige indice actualizado |', '',
             f"Ampliacion: **{len(seen)} fichas estudiadas**: C={counts['C']}, C++={counts['C++']}, ASM={counts['ASM']}. "
             'No suman nuevas detecciones a las 62 minas del banco C existente.', '',
             '## Estados y alcance', '',
             'Todas las fichas nuevas son ESTUDIADA: explicacion y proxima prueba, sin fixture nuevo ni rechazo acreditado. '
             'El catalogo rechaza etiquetas de aprobado sin evidencia. Las pruebas del generador validan datos y clasificacion, no ejecutan UB.', '',
             'UB exige lenguaje, edicion y precondiciones. ASM se clasifica por ISA, ABI, contrato del compilador o concurrencia; '
             'un page fault normal no es UB. Los enlaces al borrador vivo de C++ y al manual actual de GCC pueden cambiar: '
             'un banco ejecutable debera registrar compilador, flags, revision de referencia, arquitectura y binario.', '',
             'El objetivo es ampliar cobertura de forma verificable. No se promete agotar todas las combinaciones ni eliminar UB globalmente.', '']
    for language in CLASSES:
        lines += ['## '+language, '']
        for card in (c for c in data['piezas'] if c['lenguaje'] == language):
            lines += [f"### {card['id']}: {card['titulo']}", '',
                      f"**{card['clase']} / {card['estado']}**. Objetivo: {card['objetivo']}.", '',
                      card['regla'], '', '**Control valido:** '+card['control'], '',
                      '**Proxima prueba, pendiente:** '+card['prueba'], '',
                      '**Limite:** '+card['limite'], '',
                      f"Referencia: [{card['seccion']}]({card['fuente']}).", '']
    lines += ['## Como pasa una ficha al banco', '',
              '1. Fijar una regla, dialecto y precondiciones; escribir tambien un control valido.',
              '2. Crear un fixture minimo en el laboratorio correspondiente, con resultado esperado y limites de tiempo/recursos.',
              '3. Ejecutar los casos invalidos solo en un banco desechable sin discos personales. Registrar tambien controles aceptados.',
              '4. Conservar versiones, flags, hashes y evidencia de cada mecanismo. Un diagnostico no equivale a aislamiento; un silencio no prueba ausencia de UB.',
              '5. Llevar a produccion un cambio concreto revisado, nunca el conjunto de minas o banderas globales por defecto.', '']
    return '\n'.join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--comprobar', action='store_true')
    args = parser.parse_args()
    data = json.loads((ROOT/'Museo/catalogo.json').read_text(encoding='utf-8'))
    text = render(data)
    output = ROOT/'Museo/README.md'
    if args.comprobar:
        if not output.is_file() or output.read_text(encoding='utf-8') != text:
            raise SystemExit('ROJO: regenerar Museo/README.md')
    else:
        output.write_text(text,encoding='utf-8',newline='\n')
    print(f"VERDE: {len(data['piezas'])} fichas clasificadas; ninguna prueba de UB ejecutada")


if __name__ == '__main__': main()
