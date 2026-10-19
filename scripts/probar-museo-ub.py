#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""The museum must reject stale evidence and invented or misclassified exhibits."""
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('museo', ROOT / 'scripts/museo-ub.py')
museum = importlib.util.module_from_spec(spec)
spec.loader.exec_module(museum)

class MuseumChecks(unittest.TestCase):
    def setUp(self):
        build = (ROOT / 'build').resolve()
        build.mkdir(exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(prefix='museo-tests-', dir=build)
        self.root = Path(self.temp.name).resolve()
        # Verify the generated cleanup target before registering recursive cleanup.
        if not self.root.is_relative_to(build):
            raise RuntimeError('directorio de pruebas fuera de build')
        self.addCleanup(self.temp.cleanup)
        shutil.copytree(ROOT / 'src/guardian', self.root / 'src/guardian')
        (self.root / 'docs').mkdir()
        (self.root / 'evidencias').mkdir()
        self.registry = self.root / 'docs/museo-ub.json'
        shutil.copyfile(ROOT / 'docs/museo-ub.json', self.registry)
        for filename in ('campo-minas-ronda6.txt', 'hilos-ronda6.txt', 'dominios-ronda7.txt'):
            shutil.copyfile(ROOT / 'evidencias' / filename, self.root / 'evidencias' / filename)
        self.data = json.loads(self.registry.read_text(encoding='utf-8'))

    def save(self):
        self.registry.write_text(json.dumps(self.data), encoding='utf-8')

    def render(self):
        return museum.render(self.root, 'evidencias/campo-minas-ronda6.txt')

    def test_current_registry(self):
        _, cards, mines, mapped = self.render()
        self.assertEqual((cards, mines, mapped), (24, 62, 26))

    def test_invented_mine_rejected(self):
        self.data['piezas'][0]['runtime'] = ['mina-inventada']
        self.save()
        with self.assertRaisesRegex(ValueError, 'mina inexistente'):
            self.render()

    def test_duplicate_identifier_rejected(self):
        self.data['piezas'][1]['id'] = self.data['piezas'][0]['id']
        self.save()
        with self.assertRaisesRegex(ValueError, 'duplicado'):
            self.render()

    def test_wrong_class_rejected(self):
        self.data['piezas'][0]['clase'] = 'DEFINIDO'
        self.save()
        with self.assertRaisesRegex(ValueError, 'clasificacion'):
            self.render()

    def test_truncated_report_rejected(self):
        report = self.root / 'evidencias/campo-minas-ronda6.txt'
        lines = report.read_text(encoding='utf-8').splitlines()
        lines = [line for line in lines if not line.startswith('VERDE    entero-desborde:')]
        report.write_text('\n'.join(lines), encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'incompleta'):
            self.render()

    def test_false_totals_rejected(self):
        report = self.root / 'evidencias/campo-minas-ronda6.txt'
        text = report.read_text(encoding='utf-8').replace('VERDE=53 NEUTRA=1', 'VERDE=52 NEUTRA=2')
        report.write_text(text, encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'inconsistente'):
            self.render()

    def test_outside_reference_rejected(self):
        self.data['piezas'][0]['evidencia_externa'] = ['../fuera-del-proyecto.txt']
        self.save()
        with self.assertRaisesRegex(ValueError, 'fuera del proyecto'):
            self.render()

if __name__ == '__main__':
    unittest.main(verbosity=2)
