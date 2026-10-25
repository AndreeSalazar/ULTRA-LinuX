#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Uso del compilador autocompilado. Solo publica ELF/HEX tras verificar bytes."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import resource
import stat
import subprocess
import sys
import tempfile

from seed import ROOT,HERE,OUT,complete_source
from verify import audit


def digest(data):return hashlib.sha256(data).hexdigest()


def read_regular(path,cap):
    fd=os.open(path,os.O_RDONLY|os.O_NOFOLLOW|os.O_NONBLOCK)
    try:
        info=os.fstat(fd)
        if not stat.S_ISREG(info.st_mode) or info.st_size>cap:
            raise ValueError('archivo regular acotado requerido: '+str(path))
        data=bytearray()
        while len(data)<=cap:
            chunk=os.read(fd,min(65536,cap+1-len(data)))
            if not chunk:break
            data.extend(chunk)
        if len(data)>cap:raise ValueError('archivo excede limite: '+str(path))
        return bytes(data)
    finally:os.close(fd)


def limits():
    resource.setrlimit(resource.RLIMIT_CPU,(15,15))
    resource.setrlimit(resource.RLIMIT_CORE,(0,0))
    resource.setrlimit(resource.RLIMIT_AS,(67108864,67108864))
    resource.setrlimit(resource.RLIMIT_STACK,(1048576,1048576))
    resource.setrlimit(resource.RLIMIT_FSIZE,(4194304,4194304))


def compiler():
    if OUT.resolve()!=OUT:raise ValueError('directorio de artefactos redirigido')
    binary=OUT/'casmc-native.elf'
    manifest=OUT/'self-host.json'
    if manifest.is_symlink() or not manifest.is_file() or manifest.stat().st_size>1048576:raise ValueError('manifiesto nativo ausente o redirigido')
    record=json.loads(read_regular(manifest,1048576))
    if binary.is_symlink() or not binary.is_file() or binary.stat().st_size>262144:raise ValueError('semilla nativa ausente o redirigida')
    blob=read_regular(binary,262144);source=read_regular(HERE/'compiler.casm',65536)
    if digest(blob)!=record['compiler_sha256'] or digest(source)!=record['source_sha256']:
        raise ValueError('compiler/source cambiaron; repetir el bootstrap verificado')
    if record['self_hosted'] is not True or (HERE/'compiler.casm').read_text()!=complete_source():
        raise ValueError('perfil Native sin autocompilacion vigente')
    audit(blob)
    return binary,record


def compile_source(path):
    if os.getuid()==0:raise ValueError('usar como usuario sin privilegios del escritorio')
    if path.is_symlink() or not path.is_file() or path.stat().st_size>65536:
        raise ValueError('fuente regular, sin enlaces y de hasta 65536 bytes')
    frozen=read_regular(path,65536)
    binary,record=compiler();OUT.mkdir(parents=True,exist_ok=True)
    run=Path(tempfile.mkdtemp(prefix='run-',dir=OUT))
    source=run/'source.casm';source.write_bytes(frozen)
    pending=run/'analysis.elf';hexfile=run/'analysis.hex'
    result=subprocess.run([str(binary),str(source),str(pending),str(hexfile)],capture_output=True,
                          timeout=20,preexec_fn=limits,env={})
    (run/'registro.txt').write_bytes(result.stdout+result.stderr)
    if result.returncode:
        where=source.read_bytes();line=1;column=1
        try:
            at=int(result.stdout.splitlines()[0]);at=min(at,len(where))
            line=where[:at].count(b'\n')+1;column=at-where[:at].rfind(b'\n')
        except (ValueError,IndexError):pass
        raise ValueError(f'{path}:{line}:{column}: error: CASM Native codigo {result.returncode}; {run}/registro.txt')
    pending.chmod(0o600)
    blob=pending.read_bytes();report=audit(blob)
    if hexfile.stat().st_size>3*262144 or bytes.fromhex(hexfile.read_text())!=blob:
        raise ValueError('HEX-MISMATCH: ELF y HEX no coinciden')
    report.update({'source_sha256':digest(source.read_bytes()),'compiler_sha256':record['compiler_sha256'],
                   'compiler_self_hosted':True,'profile':'CASM Native 0.1 x86-64'})
    (run/'verified-native.json').write_text(json.dumps(report,indent=2)+'\n')
    hexfile.rename(run/'program.hex');pending.rename(run/'program.elf');(run/'program.elf').chmod(0o755)
    return run


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('source',type=Path,nargs='?');p.add_argument('--estado',action='store_true');a=p.parse_args()
    if a.estado:
        _,record=compiler();print(json.dumps({'profile':'CASM Native 0.1 x86-64','self_hosted':True,'legacy_metal64_self_hosted':False,'sha256':record['compiler_sha256']}));return 0
    if a.source is None:p.error('falta fuente.casm')
    out=compile_source(a.source)
    print(f'GREEN [CASM-NATIVE] {out}/program.hex\nELF: {out}/program.elf\nVerificacion: {out}/verified-native.json')
    return 0


if __name__=='__main__':
    try:sys.exit(main())
    except (OSError,ValueError,KeyError,subprocess.SubprocessError) as error:
        print(f'RED [CASM-NATIVE] {error}',file=sys.stderr);sys.exit(1)
