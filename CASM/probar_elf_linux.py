#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Prueba el adaptador ELF con fuentes acotadas; ejecutarlo en Linux x86-64/QEMU."""
import json
from pathlib import Path
import platform
import struct
import subprocess
import sys
import tempfile
import unittest

import bootstrap
from verify import verify


@unittest.skipUnless(sys.platform=='linux' and platform.machine()=='x86_64','Linux x86-64')
class ElfLinux(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='elf-banco-',dir=bootstrap.BASE)
        self.root = Path(self.tmp.name)
        self.source,self.hex,self.ir = [self.root/name for name in ('source.casm','body.hex','body.json')]
        self.elf = self.root/'program.elf'

    def tearDown(self):
        self.tmp.cleanup()

    def compile(self, source):
        self.source.write_text(source)
        subprocess.run([str(bootstrap.BASE/'host/casmc'),str(self.source),str(self.hex),str(self.ir)],check=True,capture_output=True,timeout=10)

    def export(self):
        return subprocess.run([sys.executable,str(bootstrap.ROOT/'CASM/elf_linux.py'),str(self.source),str(self.hex),str(self.ir),'--output',str(self.elf)],capture_output=True,text=True,timeout=10)

    def test_ejecucion_enteros_y_cadenas_con_elf_rx_y_pila_nx(self):
        self.compile('void metal_main(void) { const char a[]="A"; const int64_t n=-42; metal_puts(a); metal_put_int(n); metal_puts(a); return; }')
        result=self.export();self.assertEqual(result.returncode,0,result.stderr)
        run=subprocess.run([str(self.elf)],capture_output=True,timeout=5)
        self.assertEqual((run.returncode,run.stdout),(0,b'A-42A'))
        blob=self.elf.read_bytes()
        self.assertEqual(bytes.fromhex(self.elf.with_suffix('.linux.hex').read_text()),blob)
        self.assertEqual(struct.unpack_from('<II',blob,64),(1,5))
        self.assertEqual(struct.unpack_from('<II',blob,120),(0x6474e551,6))
        report=json.loads(self.elf.with_suffix('.linux.json').read_text())
        at=report['layout']['payload_offset']
        packet,_=verify(self.source,self.hex,self.ir)
        self.assertEqual(blob[at:],packet[24:])
        with open('/dev/full','wb') as full:
            failed=subprocess.run([str(self.elf)],stdout=full,timeout=5)
        self.assertEqual(failed.returncode,74,'un fallo de write no debe aparentar exito')

    def test_entrada_vacia_y_texto_vacio(self):
        self.compile('void metal_main(void) { const char a[]=""; metal_puts(a); return; }')
        self.assertEqual(self.export().returncode,0)
        run=subprocess.run([str(self.elf)],capture_output=True,timeout=5)
        self.assertEqual((run.returncode,run.stdout),(0,b''))

    def test_rechaza_bytes_alterados_sin_publicar(self):
        self.compile('void metal_main(void) { return; }')
        self.hex.write_text('90\n')
        self.assertNotEqual(self.export().returncode,0)
        self.assertFalse(self.elf.exists())

    def test_no_sobrescribe_salida(self):
        self.compile('void metal_main(void) { return; }')
        self.elf.write_bytes(b'conservar')
        self.assertNotEqual(self.export().returncode,0)
        self.assertEqual(self.elf.read_bytes(),b'conservar')


if __name__=='__main__':
    unittest.main(verbosity=2)
