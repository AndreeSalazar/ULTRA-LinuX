#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Semilla externa: compila el mismo compiler.casm como subconjunto C compatible.

Las generaciones nativas no usan este archivo ni GCC/Clang. No sustituye ni
declara autocompilado el frontend metal64 0.2.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

from runtime import runtime

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
OUT=ROOT/'build/CASM/native'


def complete_source():
    code,labels=runtime()
    lines=['// CASM Native x86-64: full compiler source; generated constants from runtime.py.',
           f'u64 runtime_bytes[{len(code)}] = {{'+','.join(map(str,code))+'};',
           f'u64 rt_size = {len(code)};',f'u64 rt_main_fix = {labels["main_fix"]};']
    for name in ['input','output','hexout','print','panic','overflow','division','bounds','depth']:
        lines.append(f'u64 rt_{name} = {240+labels[name]};')
    return '\n'.join(lines)+'\n'+(HERE/'compiler.body.casm').read_text()


PREFIX=r'''
#define _POSIX_C_SOURCE 200809L
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
typedef uint64_t u64;
static int casm_argc; static char **casm_argv;
static _Noreturn void casm_panic(u64 code) { fputs("CASM trap\n",stderr); exit((int)(code&255)); }
static u64 casm_print(u64 n) { printf("%llu\n",(unsigned long long)n); return 0; }
static u64 casm_input(u64 *cells,u64 cap) {
    unsigned char bytes[65536]; u64 n=0; int fd; ssize_t got; size_t i;
    if(casm_argc<4) casm_panic(65);
    fd=open(casm_argv[1],O_RDONLY|O_NOFOLLOW); if(fd<0) casm_panic(74);
    for(;;) {
        got=read(fd,bytes,sizeof bytes); if(got<0) { if(errno==EINTR) continue; casm_panic(74); }
        if(!got) break;
        for(i=0;i<(size_t)got;i++) { if(n>=cap)casm_panic(65); cells[n++]=bytes[i]; }
    }
    close(fd); return n;
}
static u64 casm_output(u64 *cells,u64 cap,u64 n,int hex) {
    u64 i=0; size_t used=0,at; ssize_t done; int fd; unsigned char bytes[65536];
    static const char digits[]="0123456789ABCDEF";
    if(n>cap)casm_panic(72); if(casm_argc<4)casm_panic(65);
    fd=open(casm_argv[hex?3:2],O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW,hex?0644:0755);
    if(fd<0)casm_panic(74);
    while(i<n) {
        used=0;
        while(i<n && used<65530) {
            if(cells[i]>255)casm_panic(70);
            if(hex) { bytes[used++]=(unsigned char)digits[cells[i]/16]; bytes[used++]=(unsigned char)digits[cells[i]%16]; bytes[used++]='\n'; }
            else bytes[used++]=(unsigned char)cells[i];
            i++;
        }
        at=0; while(at<used) { done=write(fd,bytes+at,used-at); if(done<0 && errno==EINTR)continue; if(done<=0)casm_panic(74); at+=(size_t)done; }
    }
    close(fd); return 0;
}
#define input(a) casm_input((a),sizeof(a)/sizeof((a)[0]))
#define output(a,n) casm_output((a),sizeof(a)/sizeof((a)[0]),(n),0)
#define hexout(a,n) casm_output((a),sizeof(a)/sizeof((a)[0]),(n),1)
#define print(n) casm_print(n)
#define panic(n) casm_panic(n)
'''


def build(compiler='gcc',sanitized=False):
    OUT.mkdir(parents=True,exist_ok=True)
    source=complete_source()
    frozen=HERE/'compiler.casm'
    if frozen.read_text()!=source:
        raise ValueError('compiler.casm no coincide: ejecutar seed.py generar y revisar el cambio')
    # Solo numeros lexicales; no tocar comentarios ni identificadores (u64).
    tokens=re.findall(r'//[^\n]*|/\*[\s\S]*?\*/|[A-Za-z_][A-Za-z_0-9]*|0x[0-9a-fA-F]+|[0-9]+|.',source,re.S)
    c=''.join(t+'ULL' if re.fullmatch(r'(?:0x[0-9a-fA-F]+|[0-9]+)',t) else t for t in tokens)
    declarations=re.findall(r'\bu64\s+([A-Za-z_][A-Za-z_0-9]*)\s*\(([^)]*)\)\s*\{',source)
    prototypes='\n'.join('u64 '+name+'('+args+');' for name,args in declarations)
    path=OUT/(compiler+('-asan' if sanitized else '')+'-seed.c')
    path.write_text(PREFIX+prototypes+'\n'+c+'\nint main(int ac,char **av) { casm_argc=ac;casm_argv=av;return (int)casm_main(); }\n')
    binary=path.with_suffix('.elf')
    flags=['-std=c11','-D_GNU_SOURCE','-Wall','-Wextra','-Werror','-Wno-misleading-indentation','-O1' if sanitized else '-O2',
           '-fstack-protector-strong','-fstack-clash-protection','-fcf-protection=full','-fPIE','-pie','-Wl,-z,relro,-z,now,-z,noexecstack']
    if sanitized:flags+=['-g','-fsanitize=address,undefined','-fno-sanitize-recover=all','-fno-omit-frame-pointer']
    subprocess.run([compiler,*flags,str(path),'-o',str(binary)],check=True,timeout=180)
    record={'schema':1,'profile':'CASM Native x86-64','external_seed':compiler,'self_hosted_seed':False,
            'source_sha256':hashlib.sha256(source.encode()).hexdigest(),'flags':flags,
            'compiler_version':subprocess.check_output([compiler,'--version'],text=True).splitlines()[0],
            'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest()}
    path.with_suffix('.json').write_text(json.dumps(record,indent=2)+'\n')
    return binary


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('action',choices=['generar','construir'])
    p.add_argument('--compiler',choices=['gcc','clang'],default='gcc');p.add_argument('--sanitized',action='store_true');a=p.parse_args()
    if a.action=='generar':(HERE/'compiler.casm').write_text(complete_source(),newline='\n')
    else:print(build(a.compiler,a.sanitized))
