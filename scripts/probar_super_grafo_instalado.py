#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Banco del ejecutable instalado. Solo usa un proyecto sintetico temporal."""
import json
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    assert os.getuid() == 1000, 'ejecutar como usuario del escritorio'
    binary = '/usr/local/bin/ultra-carpetas'
    with tempfile.TemporaryDirectory(prefix='ultra-sg-instalado-') as tmp:
        root = Path(tmp)
        os.environ['HOME'] = tmp
        os.environ['XDG_CACHE_HOME'] = str(root / '.cache')
        os.environ['XDG_CONFIG_HOME'] = str(root / '.config')
        os.environ.pop('ULTRA_SIN_CACHE', None)
        (root / 'a.h').write_text('#pragma once\n#define VALOR 42\n')
        (root / 'main.c').write_text('#include "a.h"\nint main(void) { return VALOR; }\n')
        def run(*args):
            return subprocess.run(args, cwd=root, check=True, text=True, capture_output=True, timeout=90).stdout
        def query(tool, args=None):
            return json.loads(run(binary, '--consulta', tool, tmp, json.dumps(args or {})))
        run('git', 'init', '-q')
        run('git', 'add', 'a.h', 'main.c')
        run('git', '-c', 'user.name=Banco', '-c', 'user.email=banco@localhost', 'commit', '-qm', 'base sintetica')
        summary = query('resumen')
        assert summary['archivos_leidos'] >= 2 and summary['dependencias'] >= 1, summary
        # Un segundo acceso debe poder reutilizar el indice incremental.
        assert query('resumen')['reutilizados_de_la_cache'] >= 2
        for compiler in ('gcc', 'clang'):
            result = query('gcc_diagnosticos', {'archivo': 'main.c', 'compilador': compiler, 'perfil': 'generico'})
            assert result['compilador'] == compiler and result['errores'] == 0, result
        history = query('cronologia', {'periodos': 2})
        assert history, 'cronologia vacia'
        run('/usr/bin/python3', '/usr/share/ultra/probar_revision_instalada.py')
        print('SUPER-GRAFO-INSTALADO-OK: indice, dependencias, cache, GCC, Clang y cronologia')


if __name__ == '__main__':
    main()
