#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Hermetic resource snapshots: missing != zero, raw counters, work caps, no writes."""
import hashlib
import pathlib
import subprocess
import sys
import tempfile
from fixture_recursos import build_fixture

source = pathlib.Path(sys.argv[1]).read_text()
with tempfile.TemporaryDirectory() as directory:
    root = pathlib.Path(directory)
    def put(name, text):
        p = root / name
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(text)
    put('proc/uptime', '123.0 42.0\n')
    put('proc/loadavg', '0.10 0.20 0.30 1/20 123\n')
    put('proc/pressure/memory', 'some avg10=0.25 avg60=0.05 avg300=0.01 total=1234\nfull avg10=0.00 avg60=0.00 avg300=0.00 total=0\n')
    put('proc/meminfo', 'MemTotal: 8192 kB\nMemAvailable: 6144 kB\nCached: 1024 kB\nShmem: 512 kB\nSwapTotal: 0 kB\nDirty: invalid kB\n')
    put('proc/vmstat', 'pgfault 1000\npgmajfault 2\npswpout 0\ncompact_stall 7\npgscan_direct 10\noom_kill 0\ninvalid 9\n')
    put('sys/kernel/mm/transparent_hugepage/enabled', 'always [madvise] never\n')
    for i in range(18):
        put(f'sys/devices/system/cpu/cpufreq/policy{i}/scaling_driver', 'amd-pstate-epp\n')
    for i in range(34):
        put(f'sys/block/fake{i}/queue/scheduler', '[none] mq-deadline\n')
        put(f'sys/block/fake{i}/stat', '1 0 8 2 0 0 0 0 0 2 2\n')
    script = root / 'meter'
    text = source.replace('/proc/', str(root / 'proc')+'/').replace('/sys/', str(root / 'sys')+'/')
    text = text.replace('/usr/local/libexec/ultra-recursos-contadores', build_fixture(root))
    script.write_text(text)
    def hashes():
        return {p.relative_to(root).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
                for p in root.rglob('*') if p.is_file()}
    before = hashes()
    result = subprocess.run(['bash', str(script)], capture_output=True, text=True, timeout=5)
    assert result.returncode == 0, result.stderr
    out = result.stdout
    assert 'PSI recurso=cpu NA' in out and 'PSI recurso=io NA' in out
    assert 'PSI recurso=memory some avg10=0.25' in out
    assert 'MEM SwapTotal: 0 kB' in out and 'MEM Dirty:' not in out
    assert 'VM compact_stall=7' in out and 'VM pgmajfault=2' in out
    assert 'POLITICA thp=always [madvise] never' in out
    assert 'POLITICA zswap=NA' in out
    assert 'CPU politicas_mostradas=16 limite=16' in out
    assert 'BLOQUE dispositivos_mostrados=32 limite=32' in out
    assert 'BLOQUE dispositivo=fake0 stat=1 0 8 2' in out
    assert len(out) < 131072
    assert before == hashes(), 'observer changed a fixture'
    print('VERDE: PSI, ausencia vs cero, contadores, politicas, limites 16/32 y fuentes intactas')
    assert subprocess.run(['bash', str(script), '--apply'], capture_output=True, timeout=5).returncode == 2
    print('VERDE: rechaza argumentos; no modo de aplicar ajustes')
    put('sys/devices/system/cpu/online', '0-11\n')
    put('sys/devices/system/node/online', '0\n')
    put('sys/devices/system/cpu/cpu0/cache/index0/type', 'Data\n')
    put('sys/devices/system/cpu/cpu0/cache/index0/size', '32K\n')
    put('sys/block/fake0/device/model', 'Virtual Disk\n')
    put('sys/block/fake0/size', '2097152\n')
    put('sys/block/fake0/ro', '1\n')
    for i in range(18):
        put(f'sys/block/fake0/slaves/lower{i}', '')
    for vendor, model in [('AuthenticAMD', 'AMD test'), ('GenuineIntel', 'Intel test')]:
        put('proc/cpuinfo', f'processor\t: 0\nvendor_id\t: {vendor}\nmodel name\t: {model}\ncpu cores\t: 6\n\nprocessor : 1\nmodel name : SECOND_CPU_NOT_COLLECTED\n')
        before = hashes()
        result = subprocess.run(['bash', str(script), '--inventario'], capture_output=True, text=True, timeout=5)
        assert result.returncode == 0, result.stderr
        out = result.stdout
        assert f'IDENTIDAD_CPU vendor_id= {vendor}' in out
        assert f'IDENTIDAD_CPU modelname= {model}' in out
        assert 'SECOND_CPU_NOT_COLLECTED' not in out
        assert 'TOPOLOGIA cpu_online=0-11' in out
        assert 'CAPACIDAD ram_tipo_fisico=NA' in out
        assert 'IDENTIDAD_BLOQUE dispositivo=fake0 model=Virtual Disk' in out
        assert 'IDENTIDAD_BLOQUE dispositivo=fake0 solo_lectura=1' in out
        assert 'CAPA dispositivo=fake0 inferiores_mostrados=16 limite=16' in out
        assert before == hashes()
    print('VERDE: inventario AMD/Intel actual, cache, disco, capas acotadas y ausencia sin inventar DDR/ECC')
