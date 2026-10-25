#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Comprueba invalidacion de la cache; no necesita compilar C."""
import json
import os
from pathlib import Path
import tempfile
import unittest

import bootstrap


class CacheSemilla(unittest.TestCase):
    def setUp(self):
        bootstrap.BASE.mkdir(parents=True,exist_ok=True)
        self.tmp=tempfile.TemporaryDirectory(prefix='cache-banco-',dir=bootstrap.BASE)
        self.path=Path(self.tmp.name)
        binary=self.path/'casmc';binary.write_bytes(b'#!/bin/sh\nexit 0\n');binary.chmod(0o755)
        self.record={'schema':1,'self_hosted':False,'diagnostic':False,
                     'binary_sha256':bootstrap.sha(binary),
                     'sources':{n:bootstrap.sha(bootstrap.ROOT/'CASM'/n) for n in bootstrap.SOURCES}}
        (self.path/'toolchain.txt').write_text('banco sintetico\n')
        self.save()

    def save(self):
        (self.path/'toolchain.json').write_text(json.dumps(self.record))

    def tearDown(self):
        self.tmp.cleanup()

    def test_cambio_de_binario_invalida(self):
        self.assertTrue(bootstrap.current(self.path))
        (self.path/'casmc').write_bytes(b'otra version')
        self.assertFalse(bootstrap.current(self.path))

    def test_cambio_de_fuentes_y_perfil(self):
        self.assertTrue(bootstrap.current(self.path,False))
        self.assertFalse(bootstrap.current(self.path,True))
        self.record['sources']['src/main.c']='0'*64;self.save()
        self.assertFalse(bootstrap.current(self.path))

    def test_json_invalido_y_faltante(self):
        (self.path/'toolchain.json').write_text('[]')
        self.assertFalse(bootstrap.current(self.path))
        (self.path/'toolchain.json').unlink()
        self.assertFalse(bootstrap.current(self.path))

    @unittest.skipUnless(os.name=='posix','enlaces Linux')
    def test_rechaza_manifiesto_y_variante_redirigidos(self):
        target=self.path/'original.json'
        (self.path/'toolchain.json').rename(target)
        (self.path/'toolchain.json').symlink_to(target)
        self.assertFalse(bootstrap.current(self.path))
        link=bootstrap.BASE/('banco-enlace-'+str(os.getpid()))
        try:
            link.symlink_to(self.path,target_is_directory=True)
            with self.assertRaises(ValueError):bootstrap.directory(link.name)
        finally:
            link.unlink(missing_ok=True)


if __name__=='__main__':
    unittest.main(verbosity=2)
