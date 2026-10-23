"""Paired whole-reader measurements on identical, immutable proc snapshots."""
import hashlib
import json
from pathlib import Path
import resource
import statistics
import subprocess
import sys
import time
from fixture_recursos import build_fixture

baseline, destination = map(Path, sys.argv[1:])
destination.mkdir(exist_ok=False)
proc = destination / 'proc'
proc.mkdir()
for name in ('meminfo', 'vmstat'):
    (proc/name).write_bytes(Path('/proc', name).read_bytes())
flags = ['-fhardened', '-fstrict-flex-arrays=3', '-fsanitize=undefined,float-cast-overflow,bounds-strict',
         '-fsanitize-trap=all', '-fno-strict-aliasing', '-static-pie']
binary = build_fixture(destination, flags)
source = baseline.read_text()
source = source[source.index('if [[ -r /proc/meminfo ]]'):source.index('for key in swappiness')]
old = destination/'anterior.sh'
old.write_text(source.replace('/proc/', str(proc)+'/'))
commands = {'bash': ['bash', str(old)], 'c-custodia': [binary]}
expected = subprocess.check_output(commands['bash'])
assert subprocess.check_output(commands['c-custodia']) == expected
records=[]
for pair in range(12):
    for route in (list(commands) if pair%2==0 else list(reversed(commands))):
        before=resource.getrusage(resource.RUSAGE_CHILDREN)
        start=time.monotonic_ns()
        for _ in range(20):
            assert subprocess.check_output(commands[route]) == expected
        wall=time.monotonic_ns()-start
        after=resource.getrusage(resource.RUSAGE_CHILDREN)
        records.append(dict(pair=pair, route=route, repetitions=20, wall_ns=wall,
            child_cpu_s=(after.ru_utime-before.ru_utime)+(after.ru_stime-before.ru_stime),
            minor_faults=after.ru_minflt-before.ru_minflt, major_faults=after.ru_majflt-before.ru_majflt))
reductions=[]
for pair in range(12):
    rows={r['route']:r for r in records if r['pair']==pair}
    reductions.append(100*(1-rows['c-custodia']['wall_ns']/rows['bash']['wall_ns']))
report=dict(schema=1, flags=flags, pairs=12, repetitions=20,
    output_sha256=hashlib.sha256(expected).hexdigest(),
    median_paired_wall_reduction_percent=statistics.median(reductions),
    median_wall_ns_per_snapshot={k:statistics.median(r['wall_ns']/20 for r in records if r['route']==k) for k in commands},
    scope='QEMU, process startup plus fixed MEM/VM parsing; not games, physical RAM/SSD or FPS', records=records)
(destination/'resultado.json').write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps({k:v for k,v in report.items() if k!='records'}, indent=2))
