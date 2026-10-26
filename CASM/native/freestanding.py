#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Adaptador Native -> modulo x86-64 sin servicios Linux. No es un bootloader."""
import argparse
import hashlib
import json
from pathlib import Path
import struct

from runtime import DATA, MEMORY, HEADER, runtime
from verify import BASE, LIMIT, audit, require

STACK_BYTES=1048576


def sha(data):return hashlib.sha256(data).hexdigest()


def convert(blob):
    proof=audit(blob,layout=True);part=proof['layout']
    require(not part['service_calls'],'FREESTANDING-SERVICES: '+','.join(part['service_calls']))
    # Preserve verified instruction offsets and absolute global addresses.
    # No ELF header or Linux runtime is executable in this target.
    text=bytearray(b'\xCC'*part['text_end'])
    end_runtime=HEADER+part['runtime_size']
    text[end_runtime:]=blob[end_runtime:part['text_end']]
    # SysV-compatible outer call boundary, internal CASM ABI remains private.
    # Save RBX/RBP/R12..R15, reserve alignment slot; clear bounded-call counter.
    entry=bytearray.fromhex('53 55 41 54 41 55 41 56 41 57 48 83 EC 08 48 BA')
    entry+=struct.pack('<Q',DATA+16)+bytes.fromhex('48 C7 02 00 00 00 00 E8')
    entry+=struct.pack('<i',part['entry_offset']-(HEADER+len(entry)+4))
    entry+=bytes.fromhex('48 83 C4 08 41 5F 41 5E 41 5D 41 5C 5D 5B C3')
    rt,labels=runtime()
    require(len(entry)<min(labels[n] for n in ['overflow','division','bounds','depth']),'ENTRY-SPACE')
    text[HEADER:HEADER+len(entry)]=entry
    traps={}
    for name in ['overflow','division','bounds','depth']:
        at=HEADER+labels[name];text[at:at+2]=b'\x0F\x0B';traps[name]=BASE+at
    data=blob[part['data_offset']:]
    return {'schema':1,'target':'casm-freestanding-x86-64-v1',
            'entry':BASE+HEADER,'return':'u64 in RAX; SysV nonvolatile registers preserved',
            'preconditions':['x86-64 long mode','DF clear','exclusive initialized RW data per instance',
                             'caller stack aligned to SysV; no concurrent or reentrant calls',
                             'fault handler required; arithmetic/bounds/depth faults execute UD2'],
            'stack_bytes':STACK_BYTES,'stack_executable':False,
            'segments':[{'address':BASE,'permissions':'RX','memory_bytes':len(text),'hex':text.hex()},
                        {'address':DATA,'permissions':'RW','memory_bytes':MEMORY,'hex':data.hex()}],
            'trap_addresses':traps,'linux_services':False,'bootable':False,
            'source_elf_sha256':sha(blob),'text_sha256':sha(text),'data_sha256':sha(data),
            'provenance_elf_hex':blob.hex()}


def validate(package):
    # Re-derive the target from independently audited provenance; reject extras,
    # changed permissions, omitted traps or modified payload, not only bad hashes.
    require(isinstance(package,dict),'MODULE-OBJECT')
    source=package.get('provenance_elf_hex')
    require(isinstance(source,str) and len(source)<=2*LIMIT,'MODULE-PROVENANCE-SIZE')
    expected=convert(bytes.fromhex(source))
    require(package==expected,'MODULE-CONTRACT')
    return {'verified':True,'target':expected['target'],'source_elf_sha256':expected['source_elf_sha256'],
            'text_sha256':expected['text_sha256'],'linux_services':False,'bootable':False}


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('elf',type=Path);p.add_argument('output',type=Path);a=p.parse_args()
    require(a.elf.is_file() and not a.elf.is_symlink() and a.elf.stat().st_size<=LIMIT,'ELF-INPUT')
    with a.elf.open('rb') as f:blob=f.read(LIMIT+1)
    require(len(blob)<=LIMIT,'ELF-SIZE')
    package=convert(blob);validate(package)
    # A new destination only. No mount, boot, executable mapping or disk changes.
    with a.output.open('x',encoding='utf-8',newline='\n') as f:json.dump(package,f,indent=2);f.write('\n')
    print('GREEN [CASM-FREESTANDING]',a.output)


if __name__=='__main__':
    try:main()
    except (OSError,ValueError,KeyError,struct.error) as error:raise SystemExit('RED [CASM-FREESTANDING] '+str(error))
