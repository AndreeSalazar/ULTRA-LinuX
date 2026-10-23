"""Compile a test-only observer with fixed paths redirected into a fixture."""
from pathlib import Path
import subprocess


def build_fixture(root, flags=None):
    src = Path(__file__).resolve().parents[1] / 'src'
    code = (src / 'recursos.c').read_text().replace('/proc/', str(root / 'proc') + '/')
    candidate = root / 'fixture-recursos.c'
    candidate.write_text(code)
    binary = root / 'fixture-recursos'
    subprocess.run(['gcc', '-std=c17', '-O2', '-Wall', '-Wextra', '-Werror', *(flags or []),
                    '-I', str(src), str(candidate),
                    str(src / 'guardian/produccion/ultra_custodia.c'),
                    str(src / 'guardian/produccion/ultra_owner.c'),
                    '-o', str(binary)], check=True, timeout=60)
    return str(binary)
