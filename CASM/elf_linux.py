#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Adapta el payload CASM 0.2 verificado a ELF Linux x86-64, sin enlazador externo.

No implementa C, libc ni el ABI SysV para funciones generales. La tabla metal64
solo ofrece puts; los enteros actuales ya llegan como datos decimales constantes.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import sys

from verify import verify

BASE = 0x400000
HEADER = 64 + 2*56


def encode(body):
    # Entrada: r15 = tabla metal64; llamar payload; sys_exit(0).
    # puts: contar hasta NUL, sys_write(1, rsi, longitud), retornar.
    puts = bytearray()
    labels, branches = {}, []
    def emit(hex_text): puts.extend(bytes.fromhex(hex_text))
    def label(name): labels[name] = len(puts)
    def jump(opcode, name):
        emit(opcode)
        branches.append((len(puts),name))
        puts.extend(bytes(4))
    emit('31 D2')                            # xor edx,edx
    label('scan');emit('80 3C 16 00')         # cmp byte [rsi+rdx],0
    jump('0F 84','length')
    emit('48 FF C2');jump('E9','scan')        # inc rdx
    label('length');emit('48 85 D2')
    jump('0F 84','done')                    # texto vacio
    emit('BF 01 00 00 00')                  # stdout
    label('write');emit('B8 01 00 00 00 0F 05')
    emit('48 83 F8 FC');jump('0F 84','write') # EINTR: reintentar
    emit('48 85 C0');jump('0F 8E','failed')   # error o escritura sin progreso
    emit('48 01 C6 48 29 C2')               # avanzar rsi, descontar rdx
    jump('0F 85','write')                   # escritura parcial
    label('done');emit('C3')
    label('failed');emit('BF 4A 00 00 00 B8 3C 00 00 00 0F 05 0F 0B')
    for at,name in branches:
        struct.pack_into('<i',puts,at,labels[name]-(at+4))
    puts = bytes(puts)
    entry_size = 21
    puts_offset = HEADER + entry_size
    table = (puts_offset + len(puts) + 7) & ~7
    payload = table + 24
    code = (bytes.fromhex('4C 8D 3D')+struct.pack('<i',table-(HEADER+7))
            +b'\xe8'+struct.pack('<i',payload-(HEADER+12))
            +bytes.fromhex('31 FF B8 3C 00 00 00 0F 05'))
    assert len(code) == entry_size
    size = payload + len(body)
    ident = b'\x7fELF'+bytes([2,1,1,0])+bytes(8)
    header = struct.pack('<16sHHIQQQIHHHHHH', ident, 2, 62, 1, BASE+HEADER,
                         64, 0, 0, 64, 56, 2, 0, 0, 0)
    load = struct.pack('<IIQQQQQQ',1,5,0,BASE,BASE,size,size,0x1000)  # RX, nunca W
    stack = struct.pack('<IIQQQQQQ',0x6474e551,6,0,0,0,0,0,16)  # RW, nunca X
    blob = header+load+stack+code+puts
    blob += bytes(table-len(blob))
    blob += bytes(16)+struct.pack('<Q', BASE+puts_offset)+body
    assert len(blob) == size and struct.unpack_from('<Q',blob,table+16)[0] == BASE+puts_offset
    return blob, {'entry_offset': HEADER, 'puts_offset': puts_offset,
                  'table_offset': table, 'payload_offset': payload, 'payload_bytes': len(body)}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('source',type=Path);p.add_argument('hex',type=Path);p.add_argument('ir',type=Path)
    p.add_argument('--output',type=Path,required=True)
    a = p.parse_args()
    packet, verified = verify(a.source,a.hex,a.ir)
    outputs = [a.output,a.output.with_suffix('.linux.hex'),a.output.with_suffix('.linux.json')]
    paths = [x.resolve() for x in [a.source,a.hex,a.ir,*outputs]]
    if len(set(paths)) != len(paths):
        raise ValueError('OUTPUT-ALIAS')
    if any(x.exists() or x.is_symlink() for x in outputs):
        raise ValueError('OUTPUT-EXISTS: elegir un destino nuevo')
    blob, layout = encode(packet[24:])
    # Antes de publicar: volver del HEX a bytes debe recuperar exactamente el ELF.
    text = ' '.join(f'{b:02X}' for b in blob)+'\n'
    assert bytes.fromhex(text) == blob
    report = {'schema':1,'target':'Linux ELF x86-64','self_hosted':False,
              'adapter':'CASM 0.2 metal64 puts -> Linux write/exit',
              'elf_sha256':hashlib.sha256(blob).hexdigest(),
              'payload_sha256':hashlib.sha256(packet[24:]).hexdigest(),
              'source_sha256':hashlib.sha256(a.source.read_bytes()).hexdigest(),
              'adapter_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'layout':layout,'load_flags':'RX','stack_flags':'RW, NX',
              'dynamic_linker':False,'external_assembler_or_linker':False,
              'limits':['Direccion fija, sin PIE; laboratorio, no compilador autocompilado.',
                        'Solo acepta el contrato CASM 0.2 que verify.py valida.']}
    a.output.parent.mkdir(parents=True,exist_ok=True)
    # Creacion exclusiva: ni sobrescribir entradas ni seguir enlaces de salida.
    with outputs[1].open('x') as file:file.write(text)
    with outputs[2].open('x') as file:file.write(json.dumps(report,indent=2)+'\n')
    # El ejecutable se publica al final, despues de sus dos controles de salida.
    with a.output.open('xb') as file:file.write(blob)
    a.output.chmod(0o755)
    print('GREEN [ELF-LINUX] payload verificado, HEX exacto, RX y pila NX')


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, struct.error) as error:
        print(f'RED [ELF-LINUX] {error}',file=sys.stderr)
        sys.exit(1)
