#!/usr/bin/python
"""Actual cgroup CPU throttling, memory OOM and pidfd close in disposable QEMU."""
import importlib.machinery,importlib.util,os,subprocess,time
from pathlib import Path
loader=importlib.machinery.SourceFileLoader('apps','/usr/local/bin/ultra-aplicaciones')
spec=importlib.util.spec_from_loader(loader.name,loader);app=importlib.util.module_from_spec(spec);loader.exec_module(app)
assert os.geteuid()==1000
base=app.CG
assert {'cpu','memory','pids'}<=set((base/'cgroup.controllers').read_text().split())
try:(base/'memory.max').write_text('max')
except PermissionError:pass
else:raise AssertionError('Usuario pudo aumentar limite del supervisor')
assert 0<int((base/'memory.max').read_text())<int(app.memory()['MemTotal'])*1024
print('LIMITES-ANCESTRO-OK',flush=True)
worker=subprocess.Popen(['/usr/bin/python','-c','while True: pass'])
try:
 time.sleep(.2);row=next(p for p in app.processes() if p['pid']==worker.pid)
 group=app.limit(row,'536870912','25000')
 def cpu():return dict(line.split() for line in (group/'cpu.stat').read_text().splitlines())
 before=cpu();time.sleep(2);after=cpu()
 assert int(after['nr_throttled'])>int(before['nr_throttled'])
 assert int(after['usage_usec'])-int(before['usage_usec'])<1500000
 app.close(row);worker.wait(timeout=5)
 print('LIMITES-CPU-PIDFD-OK',flush=True)
finally:
 if worker.poll() is None:worker.kill();worker.wait()
worker=subprocess.Popen(['/usr/bin/python','-c',"import sys; sys.stdin.readline(); a=bytearray(128*1024**2)"],stdin=subprocess.PIPE)
try:
 time.sleep(.2);row=next(p for p in app.processes() if p['pid']==worker.pid)
 group=app.limit(row,'536870912','max')
 assert int((group/'memory.max').read_text())==536870912
 assert 0<=536870912*9//10-int((group/'memory.high').read_text())<os.sysconf('SC_PAGE_SIZE')
 # Separate hard-cap verification from the intentional soft reclaim/throttle
 # path. Production retains memory.high; only this disposable leaf disables it.
 (group/'memory.high').write_text('max')
 # A 64 MiB test leaf avoids exercising the VM's whole-session pressure
 # at the same time. The application-selected 512 MiB value was checked above.
 (group/'memory.max').write_text(str(64*1024**2))
 worker.stdin.write(b'go\n');worker.stdin.flush()
 worker.wait(timeout=20)
 events=dict(line.split() for line in (group/'memory.events').read_text().splitlines())
 assert int(events['oom_kill'])>0 and worker.returncode!=0
 print('LIMITES-RAM-OOM-OK',flush=True)
finally:
 if worker.poll() is None:worker.kill();worker.wait()
print('LIMITES-FIN-OK',flush=True)
