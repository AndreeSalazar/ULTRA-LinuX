#!/usr/bin/env python3
import pathlib
import tempfile
import json
import subprocess
import sys
from resumir_medicion import summarize

with tempfile.TemporaryDirectory() as directory:
    report = pathlib.Path(directory) / 'gpu-fixture.txt'
    report.write_text('''kernel=fixture-pc
modo=nvidia
compositor=hyprland
RAM total_kB=8192 disponible_kB=6144 usada_estimada_kB=2048
RAM total_kB=8192 disponible_kB=4096 usada_estimada_kB=4096
CPU_contadores cpu 10 0 20 70 0 0 0 0 0 0
CPU_contadores cpu 20 0 40 140 0 0 0 0 0 0
UID1000 procesos=2 medidos=2 truncado=0 rss_suma_parcial_kB=4096 pss_suma_parcial_kB=1024
UID1000 procesos=2 medidos=1 truncado=0 rss_suma_parcial_kB=2000 pss_suma_parcial_kB=99999
GPU GPU-test, 0000:29:00.0, 12288, 300, 5, 53, 19.5
GPU GPU-test, 0000:29:00.0, 12288, 400, 10, 54, [N/A]
GPU NA modo_software_o_nodos_ausentes
''')
    result = summarize(report)
    assert result['ram_sistema_estimada_MiB'] == {'min': 2, 'max_muestreado': 4, 'muestras': 2}
    assert result['pss_UID1000_MiB']['max_muestreado'] == 1
    assert result['cpu_sistema_ocupada_pct']['max_muestreado'] == 30
    assert result['gpu']['GPU-test']['vram_MiB']['max_muestreado'] == 400
    assert result['gpu']['GPU-test']['potencia_W']['muestras'] == 1
    assert result['recursos']['muestras'] == 0
    report.write_text('kernel=legacy\nmodo=nvidia\n')
    result = summarize(report)
    assert result['ram_sistema_estimada_MiB'] is None and result['gpu'] is None
    print('VERDE: unidades RAM/VRAM, sumas parciales excluidas, NA y registros antiguos')
    # Check the added resource summary through the existing entry point,
    # including Windows line endings and the report-size boundary.
    report.write_bytes(b'ULTRA MEDICION v1\r\nmuestra_utc=fixture\r\n'
        b'ULTRA RECURSOS v1 solo_lectura=1 contadores=acumulados\r\n'
        b'RELOJ uptime_idle_s=1.00 0\r\n'
        b'fin_recursos snapshot_no_atomico=1 faltante_no_es_cero=1\r\n')
    assert summarize(report)['recursos']['capturas_completas'] == 1
    with report.open('wb') as stream:
        stream.truncate(32*1024*1024+1)
    try:
        summarize(report)
    except ValueError as error:
        assert '32 MiB' in str(error)
    else:
        raise AssertionError('oversize report accepted')
    print('VERDE: recursos integrados, CRLF y rechazo de informe mayor que 32 MiB')
    cli = str(pathlib.Path(__file__).with_name('resumir_medicion.py'))
    report.write_text('kernel=legacy\nmodo=nvidia\n')
    single = subprocess.run([sys.executable, cli, str(report)], capture_output=True, text=True, timeout=5)
    assert single.returncode == 0 and len(json.loads(single.stdout)) == 1
    for i in range(65):
        (pathlib.Path(directory)/f'gpu-test{i}.txt').write_text('kernel=fixture\n')
    batch = subprocess.run([sys.executable, cli, directory], capture_output=True, text=True, timeout=5)
    assert batch.returncode != 0 and '64 informes' in batch.stderr
    print('VERDE: CLI acepta archivo individual y rechaza lotes excesivos')
