#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Genera la tabla de fichas del museo para ULTRA Carpetas (SUPER GRAFO).

Lee las dos fuentes del museo (docs/museo-ub.json y Museo/catalogo.json) y escribe
src/apps/carpetas-nucleo/src/fichas.rs. No compila ni ejecuta ningun caso: solo copia
textos. `--comprobar` falla si el archivo generado no coincide con los catalogos.
"""
import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SALIDA = ROOT / 'src/apps/carpetas-nucleo/src/fichas.rs'


def rust(s):
    return '"' + s.replace('\\', '\\\\').replace('"', '\\"').replace('\n', ' ') + '"'


def fichas():
    ub = json.loads((ROOT / 'docs/museo-ub.json').read_text(encoding='utf-8'))
    nuevo = json.loads((ROOT / 'Museo/catalogo.json').read_text(encoding='utf-8'))
    salida = []
    for p in ub['piezas']:
        fuente = ub['fuentes'][p['fuente']]
        salida.append(dict(id=p['id'], lenguaje='C', clase=p['clase'], titulo=p['titulo'], regla=p['causa'],
                           control=p['alternativa'], limite=p['limite'], referencia=f"{fuente['titulo']}, {p['clausula']}",
                           url=fuente['url'], pagina='docs/MUSEO-UB.md#' + p['id'].lower()))
    for p in nuevo['piezas']:
        salida.append(dict(id=p['id'], lenguaje=p['lenguaje'], clase=p['clase'], titulo=p['titulo'], regla=p['regla'],
                           control=p['control'], limite=p['limite'], referencia=p['seccion'], url=p['fuente'],
                           pagina='Museo/README.md#' + p['id'].lower()))
    ids = [f['id'] for f in salida]
    if len(ids) != len(set(ids)):
        raise SystemExit('ids repetidos entre los catalogos')
    return salida


def generar():
    lineas = ['// SPDX-License-Identifier: GPL-2.0-only',
              '//! GENERADO por scripts/fichas-carpetas.py desde docs/museo-ub.json y Museo/catalogo.json.',
              '//! No editar a mano: cambiar el catalogo y volver a generar (`--comprobar` lo exige).',
              '',
              '/// Una ficha del museo: lo que se ensena al detectar su patron.',
              '#[derive(Debug, Clone, Copy, PartialEq, Eq)]',
              'pub struct Ficha {',
              '    pub id: &\'static str,',
              '    /// "C", "C++" o "ASM".',
              '    pub lenguaje: &\'static str,',
              '    pub clase: &\'static str,',
              '    pub titulo: &\'static str,',
              '    pub regla: &\'static str,',
              '    /// La alternativa o el control valido.',
              '    pub control: &\'static str,',
              '    pub limite: &\'static str,',
              '    pub referencia: &\'static str,',
              '    pub url: &\'static str,',
              '    /// Donde leerla en el repositorio.',
              '    pub pagina: &\'static str,',
              '}',
              '',
              'pub const FICHAS: &[Ficha] = &[']
    for f in fichas():
        campos = ', '.join(f'{k}: {rust(v)}' for k, v in f.items())
        lineas.append(f'    Ficha {{ {campos} }},')
    lineas += ['];', '']
    return '\n'.join(lineas)


def main():
    a = argparse.ArgumentParser(description=__doc__)
    a.add_argument('--comprobar', action='store_true', help='fallar si fichas.rs no esta al dia')
    args = a.parse_args()
    texto = generar()
    if args.comprobar:
        actual = SALIDA.read_text(encoding='utf-8') if SALIDA.exists() else ''
        if actual != texto:
            raise SystemExit(f'{SALIDA.relative_to(ROOT)} no esta al dia: python scripts/fichas-carpetas.py')
        print(f'ok: {texto.count("Ficha {") - 1} fichas al dia')
        return
    SALIDA.write_text(texto, encoding='utf-8')
    print(f'escrito {SALIDA.relative_to(ROOT)}')


if __name__ == '__main__':
    main()
