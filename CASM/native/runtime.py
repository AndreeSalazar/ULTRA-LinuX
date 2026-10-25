#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Plantillas x86-64 propias del perfil Native; no invoca as, ld ni compiladores."""
import struct

DATA = 0x10000000
SCRATCH = DATA + 0x800000
MEMORY = 0x810000
HEADER = 240


class Bytes:
    def __init__(self):
        self.code=bytearray();self.labels={};self.fix=[]
    def e(self,text):self.code.extend(bytes.fromhex(text))
    def q(self,value):self.code.extend(struct.pack('<Q',value))
    def label(self,name):self.labels[name]=len(self.code)
    def jump(self,opcode,name):
        self.e(opcode);self.fix.append((len(self.code),name));self.code.extend(bytes(4))
    def finish(self):
        for at,name in self.fix:struct.pack_into('<i',self.code,at,self.labels[name]-(at+4))
        return bytes(self.code),self.labels


def runtime():
    a=Bytes()
    a.label('entry')
    a.e('48 8B 04 24 48 BA');a.q(DATA)
    a.e('48 89 02 48 8D 44 24 08 48 89 42 08 E8')
    main_fix=len(a.code);a.e('00 00 00 00')
    a.e('48 89 C7 B8 3C 00 00 00 0F 05 0F 0B')
    # write_all(fd=rdi, bytes=rsi, count=rdx). EINTR y escrituras parciales.
    a.label('write_all');a.e('48 85 D2');a.jump('0F 84','write_done')
    a.label('write_loop');a.e('B8 01 00 00 00 0F 05 48 83 F8 FC')
    a.jump('0F 84','write_loop');a.e('48 85 C0');a.jump('0F 8E','io')
    a.e('48 01 C6 48 29 C2');a.jump('0F 85','write_loop')
    a.label('write_done');a.e('C3')
    # input(array): argumentos propios ptr, capacidad; lee argv[1].
    a.label('input');a.e('55 48 89 E5 4C 8B 65 18 4C 8B 6D 10 48 B8');a.q(DATA)
    a.e('48 83 38 04');a.jump('0F 82','source')
    a.e('48 8B 40 08 48 8B 78 08 BE 00 00 02 00 31 D2 B8 02 00 00 00 0F 05 48 85 C0')
    a.jump('0F 88','io');a.e('49 89 C6 45 31 FF')
    a.label('read_loop');a.e('4C 89 F7 48 BE');a.q(SCRATCH)
    a.e('BA 00 00 01 00 31 C0 0F 05 48 83 F8 FC');a.jump('0F 84','read_loop')
    a.e('48 85 C0');a.jump('0F 88','io');a.jump('0F 84','read_done')
    a.e('49 89 C0 31 C9')
    a.label('read_cell');a.e('4D 39 EF');a.jump('0F 83','source')
    a.e('0F B6 04 0E 4B 89 04 FC 49 FF C7 48 FF C1 4C 39 C1');a.jump('0F 82','read_cell')
    a.jump('E9','read_loop')
    a.label('read_done');a.e('4C 89 F7 B8 03 00 00 00 0F 05 4C 89 F8 C9 C3')
    # output(array,n) / hexout(array,n), argv[2]/argv[3], creacion exclusiva.
    a.label('output');a.e('45 31 C9');a.jump('E9','output_common')
    a.label('hexout');a.e('41 B9 01 00 00 00')
    a.label('output_common');a.e('55 48 89 E5 4C 8B 65 20 4C 8B 6D 18 4C 8B 75 10 4D 39 EE')
    a.jump('0F 87','bounds');a.e('48 B8');a.q(DATA)
    a.e('48 83 38 04');a.jump('0F 82','source')
    a.e('48 8B 40 08 48 8B 78 10 BA ED 01 00 00 4D 85 C9')
    a.jump('0F 84','open_output')
    a.e('48 8B 78 18 BA A4 01 00 00')
    a.label('open_output');a.e('BE C1 00 02 00 B8 02 00 00 00 0F 05 48 85 C0')
    a.jump('0F 88','io');a.e('49 89 C5 45 31 FF')
    a.label('output_chunk');a.e('31 C9 48 BE');a.q(SCRATCH)
    a.label('output_cell');a.e('4D 39 F7');a.jump('0F 83','flush')
    a.e('4B 8B 04 FC 48 3D FF 00 00 00');a.jump('0F 87','overflow')
    a.e('4D 85 C9');a.jump('0F 85','hex_cell')
    a.e('88 04 0E 48 FF C1');a.jump('E9','next_cell')
    a.label('hex_cell');a.e('48 89 C2 48 C1 E8 04 04 30 3C 39');a.jump('0F 86','hex_high')
    a.e('04 07');a.label('hex_high');a.e('88 04 0E 48 FF C1 80 E2 0F 80 C2 30 80 FA 39')
    a.jump('0F 86','hex_low');a.e('80 C2 07')
    a.label('hex_low');a.e('88 14 0E 48 FF C1 C6 04 0E 0A 48 FF C1')
    a.label('next_cell');a.e('49 FF C7 48 81 F9 FA FF 00 00');a.jump('0F 82','output_cell')
    a.label('flush');a.e('48 85 C9');a.jump('0F 84','output_done')
    a.e('4C 89 EF 48 89 CA');a.jump('E8','write_all');a.jump('E9','output_chunk')
    a.label('output_done');a.e('4C 89 EF B8 03 00 00 00 0F 05 31 C0 C9 C3')
    # print(u64), sin libc.
    a.label('print');a.e('48 8B 44 24 08 48 BE');a.q(SCRATCH+32)
    a.e('C6 06 0A 41 B8 0A 00 00 00')
    a.label('digit');a.e('31 D2 49 F7 F0 80 C2 30 48 FF CE 88 16 48 85 C0')
    a.jump('0F 85','digit');a.e('48 BA');a.q(SCRATCH+33)
    a.e('48 29 F2 BF 01 00 00 00');a.jump('E8','write_all');a.e('31 C0 C3')
    a.label('panic');a.e('48 8B 7C 24 08');a.jump('E9','trap')
    for name,code in [('source',65),('overflow',70),('division',71),('bounds',72),('depth',73),('io',74)]:
        a.label(name);a.e('BF');a.code.extend(struct.pack('<I',code));a.jump('E9','trap')
    a.label('trap');a.e('49 89 FA BF 02 00 00 00 48 8D 35')
    a.fix.append((len(a.code),'message'));a.e('00 00 00 00 BA 0A 00 00 00 B8 01 00 00 00 0F 05 4C 89 D7 B8 3C 00 00 00 0F 05 0F 0B')
    a.label('message');a.code.extend(b'CASM trap\n')
    code,labels=a.finish()
    labels['main_fix']=main_fix
    return code,labels


if __name__=='__main__':
    import json
    code,labels=runtime()
    print(json.dumps({'bytes':len(code),'labels':labels},indent=2))
