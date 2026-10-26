#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Banco Clang C17/C++20. Solo fuentes del museo, QEMU sin root y Bubblewrap."""
import hashlib
import json
import os
from pathlib import Path
import resource
import shutil
import signal
import subprocess
import tempfile
import time

ROOT=Path(__file__).resolve().parents[4]
HERE=Path(__file__).resolve().parent
PROFILES={
    'asan':['-fsanitize=address','-fsanitize-address-use-after-return=always','-fsanitize-address-use-after-scope'],
    'ubsan':['-fsanitize=undefined','-fno-sanitize-recover=all'],
}


def limits():
    resource.setrlimit(resource.RLIMIT_CPU,(15,15))
    resource.setrlimit(resource.RLIMIT_CORE,(0,0))
    resource.setrlimit(resource.RLIMIT_FSIZE,(33554432,33554432))
    # ASan maps terabytes of virtual shadow memory: do not cap RLIMIT_AS.


def sandbox(job,argv):
    args=['/usr/bin/bwrap','--unshare-all','--die-with-parent','--new-session','--clearenv']
    for name in ['/usr','/lib','/lib64','/bin']:
        if Path(name).exists():args+=['--ro-bind',name,name]
    if Path('/etc/ld.so.cache').is_file():args+=['--ro-bind','/etc/ld.so.cache','/etc/ld.so.cache']
    args+=['--dev','/dev','--proc','/proc','--tmpfs','/tmp','--bind',str(job),'/work','--chdir','/work',
           '--setenv','PATH','/usr/bin:/bin','--setenv','HOME','/tmp','--setenv','LC_ALL','C',
           '--setenv','ASAN_OPTIONS','detect_leaks=0:detect_stack_use_after_return=1:halt_on_error=1:symbolize=0',
           '--setenv','UBSAN_OPTIONS','halt_on_error=1:print_stacktrace=0']
    return args+argv


def run(job,argv,name,timeout=30):
    log=job/(name+'.txt')
    with log.open('wb') as f:
        proc=subprocess.Popen(sandbox(job,argv),stdout=f,stderr=subprocess.STDOUT,
                              preexec_fn=limits,start_new_session=True,env={'PATH':'/usr/bin:/bin'})
        try:code=proc.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            os.killpg(proc.pid,signal.SIGKILL);proc.wait();code=None
    with log.open('rb') as f:text=f.read(65536).decode('utf-8','replace')
    return code,text


def main():
    if os.getuid()==0 or Path('/sys/class/dmi/id/sys_vendor').read_text().strip()!='QEMU':
        raise SystemExit('Solo taller QEMU, usuario sin privilegios')
    if not Path('/usr/bin/bwrap').is_file():raise SystemExit('Bubblewrap ausente: no ejecutar sin aislamiento')
    base=ROOT/'build/llvm-ub';base.mkdir(parents=True,exist_ok=True)
    out=Path(tempfile.mkdtemp(prefix='run-',dir=base))
    # Verify absent home/project mounts and a distinct network namespace.
    host_net=os.readlink('/proc/self/ns/net')
    probe=out/'isolation';probe.mkdir()
    code,text=run(probe,['/usr/bin/python3','-c',
        "import os; assert not os.path.exists('/home/builder'); assert not os.path.exists('/juegos'); print(os.readlink('/proc/self/ns/net'))"],'probe')
    if code!=0 or text.strip()==host_net:raise SystemExit('Aislamiento no disponible: '+text)
    cases=json.loads((HERE/'casos.json').read_text())['cases'];rows=[];tools={}
    for language in ['c17','cpp20']:
        tool='/usr/bin/clang' if language=='c17' else '/usr/bin/clang++'
        if not Path(tool).is_file():raise SystemExit('Falta '+tool)
        tool_job=out/('tool-'+language);tool_job.mkdir()
        code,version=run(tool_job,[tool,'--version'],'version')
        if code!=0:raise SystemExit(version)
        tools[language]=version.splitlines()[0]
        for case in cases:
            if language not in case['languages']:continue
            for opt in ['-O0','-O2']:
                for bad in [False,True]:
                    name=f"{case['id']}-{language}-{opt[1:]}-{'mal' if bad else 'bien'}"
                    job=out/name;job.mkdir();suffix='.c' if language=='c17' else '.cpp'
                    source=job/('case'+suffix);source.write_text(case['source'])
                    flags=['-std=c17' if language=='c17' else '-std=c++20',opt,'-g','-fno-omit-frame-pointer',
                           '-fno-optimize-sibling-calls','-fPIE','-pie',*PROFILES[case['profile']],f'-DBAD={int(bad)}']
                    if language=='cpp20' and case['profile']=='ubsan':flags+=['-fsanitize=vptr']
                    code,log=run(job,[tool,*flags,source.name,'-o','case.elf'],'compile')
                    status='compile_failed';exit_code=None
                    if code is None:status='timeout'
                    elif code==0:
                        exit_code,log=run(job,['/work/case.elf'],'execute',timeout=10)
                        if exit_code is None:status='timeout'
                        elif bad:
                            status='passed' if exit_code!=0 and case['marker'] in log else 'known_gap' if case.get('known_gap') else 'detection_missing'
                        else:status='passed' if exit_code==0 and not log.strip() else 'control_failed'
                    rows.append({'id':name,'language':language,'profile':case['profile'],'variant':'mal' if bad else 'bien',
                                 'expected_marker':case['marker'] if bad else None,'status':status,'exit_code':exit_code,
                                 'flags':flags,'known_gap_reason':case.get('known_gap'),
                                 'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),
                                 'log':str((job/('execute.txt' if code==0 else 'compile.txt')).relative_to(ROOT))})
    report={'schema':1,'kind':'ultra-llvm-ub-lab','created_unix':int(time.time()),'toolchains':tools,
            'isolation':{'qemu':True,'uid':os.getuid(),'bubblewrap':True,'network_namespace_distinct':True,
                         'project_and_home_not_mounted':True},
            'production_certified':False,'scope':'Only these fixtures and inputs; not arbitrary C/C++ or the kernel',
            'cases':rows,'pending_profiles':['tsan','msan','libfuzzer','cfi-lto'],
            'suite_sha256':hashlib.sha256((HERE/'casos.json').read_bytes()).hexdigest()}
    (out/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    # Replace only the current report, never remove previous runs or games.
    pending=out/'current.pending.json';pending.write_text(json.dumps(report,indent=2)+'\n');pending.replace(base/'current.json')
    passed=sum(x['status']=='passed' for x in rows)
    gaps=sum(x['status']=='known_gap' for x in rows)
    print(f'LLVM LAB: {passed}/{len(rows)} fulfilled; {gaps} known gaps (NOT passed); report: {out}/report.json')
    return 0 if passed+gaps==len(rows) else 1


if __name__=='__main__':raise SystemExit(main())
