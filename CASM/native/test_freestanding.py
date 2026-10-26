#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Differential u64, target rejection and isolated module execution. QEMU only."""
import argparse
import copy
import ctypes
import json
import os
from pathlib import Path
import random
import resource
import signal
import subprocess
import sys
import tempfile

from freestanding import convert, validate
from seed import ROOT, OUT
from test import compile_with


def require_lab():
    if os.getuid()==0 or Path('/sys/class/dmi/id/sys_vendor').read_text().strip()!='QEMU':
        raise SystemExit('unprivileged QEMU only')


def execute(path):
    require_lab()
    resource.setrlimit(resource.RLIMIT_CPU,(2,2))
    resource.setrlimit(resource.RLIMIT_CORE,(0,0))
    package=json.loads(path.read_text());validate(package)
    libc=ctypes.CDLL(None,use_errno=True)
    libc.mmap.argtypes=[ctypes.c_void_p,ctypes.c_size_t,ctypes.c_int,ctypes.c_int,ctypes.c_int,ctypes.c_long]
    libc.mmap.restype=ctypes.c_void_p
    libc.mprotect.argtypes=[ctypes.c_void_p,ctypes.c_size_t,ctypes.c_int]
    for segment in package['segments']:
        address=segment['address'];size=(segment['memory_bytes']+4095)&~4095
        # MAP_FIXED_NOREPLACE, never overwrite an existing process mapping.
        mapped=libc.mmap(address,size,3,0x100022,-1,0)
        if mapped!=address:raise OSError(ctypes.get_errno(),'fixed mapping unavailable')
        data=bytes.fromhex(segment['hex']);ctypes.memmove(address,data,len(data))
        if segment['permissions']=='RX' and libc.mprotect(address,size,5):raise OSError('mprotect')
    entry=ctypes.CFUNCTYPE(ctypes.c_uint64)(package['entry'])
    print(entry(),flush=True)


def main():
    require_lab()
    folder=Path(tempfile.mkdtemp(prefix='freestanding-',dir=OUT))
    compiler=OUT/'casmc-native.elf';cases=[];rng=random.Random(20261025)

    def check(name,source,expected=None,trap=False):
        src=folder/(name+'.casm');src.write_text(source)
        elf=compile_with(compiler,src,folder/name)
        package=convert(elf.read_bytes());validate(package)
        target=folder/(name+'.module.json');target.write_text(json.dumps(package))
        result=subprocess.run([sys.executable,__file__,'--execute',str(target)],capture_output=True,timeout=8)
        if trap:assert result.returncode==-signal.SIGILL,(name,result)
        else:assert result.returncode==0 and result.stdout.strip()==str(expected).encode(),(name,result)
        cases.append({'case':name,'passed':True,'kind':'trap-UD2' if trap else 'u64-differential'})
        return package

    for i in range(64):
        a=rng.randrange(1<<24);b=rng.randrange(1,1<<20);width=rng.randrange(0,24)
        ops=[('+',a+b),('*',a*b),('/',a//b),('%',a%b),('&',a&b),('|',a|b),('^',a^b),('<<',a<<width)]
        op,want=ops[i%len(ops)];rhs=width if op=='<<' else b
        check('differential-'+str(i),f'u64 casm_main() {{ return {a}{op}{rhs}; }}',want)
    base=check('max-u64','u64 casm_main() { return 18446744073709551615; }',(1<<64)-1)
    check('arrays-functions','u64 a[3]={4,7,9}; u64 sum(u64 n) { u64 i=0; u64 v=0; while(i<n) { v=v+a[i]; i=i+1; } return v; } u64 casm_main() { return sum(3); }',20)
    for name,expression in [('add-overflow','18446744073709551615+1'),('underflow','0-1'),
                            ('multiply-overflow','18446744073709551615*2'),('divide-zero','4/0'),
                            ('modulo-zero','4%0'),('shift-width','1<<64'),('shift-overflow','2<<63')]:
        check(name,'u64 casm_main() { return '+expression+'; }',trap=True)
    check('bounds','u64 a[2]; u64 casm_main() { return a[2]; }',trap=True)
    check('depth','u64 f() { return f(); } u64 casm_main() { return f(); }',trap=True)
    rejected=[]
    for name,source in [('print','u64 casm_main() { print(1); return 0; }'),
                        ('panic','u64 casm_main() { panic(3); return 0; }'),
                        ('input','u64 a[2]; u64 casm_main() { return input(a); }'),
                        ('output','u64 a[2]; u64 casm_main() { return output(a,2); }'),
                        ('hexout','u64 a[2]; u64 casm_main() { return hexout(a,2); }'),
                        ('unreachable-service','u64 f() { print(1); return 0; } u64 casm_main() { return 0; }')]:
        src=folder/(name+'.casm');src.write_text(source);elf=compile_with(compiler,src,folder/name)
        try:convert(elf.read_bytes())
        except ValueError as error:assert 'FREESTANDING-SERVICES' in str(error)
        else:raise AssertionError(name)
        rejected.append(name)
    mutations=[]
    changes=[('target',lambda p:p.update(target='other')),('entry',lambda p:p.update(entry=p['entry']+1)),
             ('rw-code',lambda p:p['segments'][0].update(permissions='RWX')),
             ('short-data',lambda p:p['segments'][1].update(memory_bytes=1)),
             ('address',lambda p:p['segments'][1].update(address=0)),
             ('code',lambda p:p['segments'][0].update(hex='0f05')),
             ('data',lambda p:p['segments'][1].update(hex='01')),
             ('trap',lambda p:p['trap_addresses'].update(bounds=0)),
             ('stack',lambda p:p.update(stack_executable=True)),('extra',lambda p:p.update(ignore=True)),
             ('hash',lambda p:p.update(text_sha256='0'*64)),('provenance-size',lambda p:p.update(provenance_elf_hex='00'*262145))]
    for name,change in changes:
        p=copy.deepcopy(base);change(p)
        try:validate(p)
        except ValueError:pass
        else:raise AssertionError(name)
        mutations.append(name)
    report={'target':'casm-freestanding-x86-64-v1','execution_environment':'QEMU Linux process harness; not a bare-metal boot',
            'compiler_host':'Linux','module_linux_services':False,'bootable':False,
            'executions':cases,'service_rejections':rejected,'mutation_rejections':mutations,
            'sample_source_elf_sha256':base['source_elf_sha256'],
            'evidence_source':'max-u64 example, not compiler binary'}
    # Keep exact reviewed sample as portable evidence; no ELF header is mapped executable.
    (OUT/'freestanding-example.module.json').write_text(json.dumps(base,indent=2)+'\n')
    (OUT/'freestanding-tests.json').write_text(json.dumps(report,indent=2)+'\n')
    print(f'GREEN [FREESTANDING] {len(cases)} executions; {len(rejected)} service rejections; {len(mutations)} mutations')


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--execute',type=Path);a=p.parse_args()
    if a.execute:execute(a.execute)
    else:main()
