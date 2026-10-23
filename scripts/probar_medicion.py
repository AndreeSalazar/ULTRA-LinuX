#!/usr/bin/env python3
"""Hermetic procfs fixtures: arithmetic, partial PSS, filtering and work bound."""
import pathlib
import subprocess
import sys
import tempfile
from fixture_recursos import build_fixture

source = pathlib.Path(sys.argv[1]).read_text()
with tempfile.TemporaryDirectory() as directory:
    root = pathlib.Path(directory)
    proc = root / 'proc'
    proc.mkdir()
    (proc / 'meminfo').write_text('MemTotal: 8192 kB\nMemAvailable: 6144 kB\nSwapTotal: 1024 kB\nSwapFree: 512 kB\n')
    (proc / 'stat').write_text('cpu  10 0 20 70 0 0 0 0 0 0\n')
    def process(pid, uid, rss=None, pss=None):
        item = proc / str(pid)
        item.mkdir()
        (item / 'status').write_text(f'Name:\tapp{pid}\nUid:\t{uid} {uid} {uid} {uid}\n')
        if rss is not None:
            (item / 'smaps_rollup').write_text(f'Rss: {rss} kB\nPss: {pss} kB\n')
    process(10, 1000, 120, 80)
    process(11, 1000, 100, 40)
    process(12, 1000)  # Unavailable != zero; aggregate is explicitly partial.
    process(13, 1001, 9999, 9999)
    script = root / 'meter'
    text = source.replace('[[ $(id -u) == 0 ]]', '[[ 1 == 1 ]]')
    text = text.replace('/proc/', f'{proc}/')
    text = text.replace('/run/ultra-escritorio/medicion.stop', str(root / 'stop'))
    text = text.replace('/dev/nvidia', str(root / 'missing-nvidia'))
    if len(sys.argv) > 2:
        resource = root / 'resources'
        resource_text = pathlib.Path(sys.argv[2]).read_text().replace('/proc/', f'{proc}/').replace('/sys/', str(root / 'sys')+'/')
        resource.write_text(resource_text.replace('/usr/local/libexec/ultra-recursos-contadores', build_fixture(root)))
        # Exercise the snapshot inside the session with the same proc fixture.
        # Privilege dropping is checked separately in the real workshop.
        text = text.replace('user_query /usr/local/libexec/ultra-medir-recursos', f'bash "{resource}"')
    script.write_text(text)
    def run(mode='software'):
        result = subprocess.run(['bash', str(script), mode, '--once'], capture_output=True, text=True, timeout=5)
        assert result.returncode == 0, result.stderr
        return result.stdout
    output = run()
    if len(sys.argv) > 2:
        assert 'ULTRA RECURSOS v1 solo_lectura=1' in output
        assert 'MEM MemAvailable: 6144 kB' in output
        assert 'PSI recurso=memory NA' in output
        assert output.count('INVENTARIO RECURSOS v1') == 1
        print('VERDE: snapshot de recursos integrado en sesion con el mismo procfs de prueba')
    assert 'usada_estimada_kB=2048' in output
    assert 'procesos=3 medidos=2 truncado=0 rss_suma_parcial_kB=220 pss_suma_parcial_kB=120' in output
    assert 'pid=13 ' not in output
    assert 'rss_kB=NA pss_kB=NA' in output
    assert 'GPU NA modo_software_o_nodos_ausentes' in output
    assert 'fin_medicion muestras=1' in output
    print('VERDE: RAM, UID, RSS/PSS parcial, GPU ausente y una muestra')
    assert 'GPU NA modo_software_o_nodos_ausentes' in run('nvidia')
    print('VERDE: modo NVIDIA sin nodos no consulta ni prepara dispositivos')
    (proc / 'meminfo').write_text('MemTotal: 8192 kB\n')
    assert 'usada_estimada_kB=NA' in run()
    print('VERDE: MemAvailable ausente no se inventa')
    for pid in range(100, 240):
        process(pid, 1000, 4, 4)
    assert 'procesos=128 medidos=' in run() and 'truncado=1' in run()
    print('VERDE: limite 128 procesos')
    (root / 'stop').touch()
    stopped = run()
    assert 'fin_medicion muestras=0' in stopped
    assert 'INVENTARIO RECURSOS v1' not in stopped
    print('VERDE: parada previa sin atribuir una muestra inexistente')
