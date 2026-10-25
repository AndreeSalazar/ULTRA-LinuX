#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Comprueba el lanzador MCP con un Cargo simulado, sin descargar ni compilar."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


@unittest.skipUnless(os.name == 'posix', 'banco del lanzador Linux')
class LanzadorMcp(unittest.TestCase):
    def test_revisa_cambios_respeta_target_y_no_usa_binario_tras_fallo(self):
        source = Path(__file__).resolve().with_name('super-grafo-mcp.sh')
        with tempfile.TemporaryDirectory(prefix='ultra-mcp-') as tmp:
            root = Path(tmp)
            scripts = root / 'scripts'
            app = root / 'src/apps/ultra-carpetas'
            tools = root / 'tools'
            for path in (scripts, app, tools, root / 'carpeta con espacios'):
                path.mkdir(parents=True)
            shutil.copyfile(source, scripts / source.name)
            cargo = tools / 'cargo'
            cargo.write_text('#!/bin/sh\nprintf "%s|%s\\n" "$PWD" "$*" >> "$BANCO_LOG"\n'
                             'test ! -e "$BANCO_FALLO"\n')
            cargo.chmod(0o755)
            target = app / 'cache propia'
            (target / 'release').mkdir(parents=True)
            binary = target / 'release/ultra-carpetas'
            binary.write_text('#!/bin/sh\nprintf "MCP:%s:%s\\n" "$1" "$2"\n')
            binary.chmod(0o755)
            env = {**os.environ, 'PATH': str(tools) + os.pathsep + os.environ['PATH'],
                   'CARGO_TARGET_DIR': 'cache propia', 'BANCO_LOG': str(root / 'cargo.log'),
                   'BANCO_FALLO': str(root / 'fallar')}
            command = ['sh', str(scripts / source.name), 'carpeta con espacios']
            for _ in range(2):
                run = subprocess.run(command, cwd=root, env=env, text=True, capture_output=True, timeout=10)
                self.assertEqual(run.returncode, 0, run.stderr)
                self.assertEqual(run.stdout.strip(), f'MCP:--mcp:{root / "carpeta con espacios"}')
            lines = (root / 'cargo.log').read_text().splitlines()
            self.assertEqual(lines, [f'{app}|build --release --locked --quiet'] * 2)
            (root / 'fallar').touch()
            run = subprocess.run(command, cwd=root, env=env, text=True, capture_output=True, timeout=10)
            self.assertNotEqual(run.returncode, 0)
            self.assertEqual(run.stdout, '', 'no debe iniciar el ejecutable viejo')


if __name__ == '__main__':
    unittest.main(verbosity=2)
