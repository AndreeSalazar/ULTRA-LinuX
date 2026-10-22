#!/usr/bin/env python3
"""Validate classifications and reject fabricated runtime credit; no native code."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('museum', ROOT/'scripts/museo-sistemas.py')
museum = importlib.util.module_from_spec(spec); spec.loader.exec_module(museum)
DATA = json.loads((ROOT/'Museo/catalogo.json').read_text(encoding='utf-8'))


class MuseumChecks(unittest.TestCase):
    def setUp(self): self.data = copy.deepcopy(DATA)

    def test_current_generated_index(self):
        self.assertEqual(museum.render(self.data),(ROOT/'Museo/README.md').read_text(encoding='utf-8'))

    def test_asm_not_iso_ub(self):
        self.data['piezas'][-1]['clase'] = 'UB'
        with self.assertRaisesRegex(ValueError,'clase'): museum.render(self.data)

    def test_duplicate_rejected(self):
        self.data['piezas'][1]['id'] = self.data['piezas'][0]['id']
        with self.assertRaisesRegex(ValueError,'duplicado'): museum.render(self.data)

    def test_invented_success_rejected(self):
        self.data['piezas'][0]['estado'] = 'VERDE'
        with self.assertRaisesRegex(ValueError,'ejecuciones'): museum.render(self.data)

    def test_missing_control_rejected(self):
        del self.data['piezas'][0]['control']
        with self.assertRaisesRegex(ValueError,'incompleta'): museum.render(self.data)

    def test_wrong_language_prefix_rejected(self):
        self.data['piezas'][0]['id'] = 'CXX-999'
        with self.assertRaisesRegex(ValueError,'ID'): museum.render(self.data)

    def test_non_https_rejected(self):
        self.data['piezas'][0]['fuente'] = 'file:///privado'
        with self.assertRaisesRegex(ValueError,'HTTPS'): museum.render(self.data)


if __name__ == '__main__': unittest.main(verbosity=2)
