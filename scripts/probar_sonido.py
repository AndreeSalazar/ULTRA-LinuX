#!/usr/bin/env python3
"""ULTRA Sonido sin PipeWire: grafo coherente, neutro al apagar y sin saturacion."""
import importlib.machinery
import importlib.util
import itertools
import json
import math
from pathlib import Path
import re
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
loader = importlib.machinery.SourceFileLoader('ultra_sonido', str(ROOT/'src/rootfs/bin/ultra-sonido'))
spec = importlib.util.spec_from_loader(loader.name, loader)
app = importlib.util.module_from_spec(spec); loader.exec_module(app)


def worst_gain(state):
    """Peak gain of the graph for the worst case input (dB of EQ + matrix)."""
    c = app.controls(state)
    eq = 10**(max([0.0]+[state[n] for n, *_ in app.BANDS])/20) if state['activo'] else 1.0
    # Worst case per output: |g1|*|L| + |g2|*|R| + |g3|*|R| with |L|,|R| <= 1.
    matrix = abs(c['mezcla_l:Gain 1'])+abs(c['mezcla_l:Gain 2'])+abs(c['mezcla_l:Gain 3'])
    return eq*matrix


class Grafo(unittest.TestCase):
    def setUp(self):
        self.state = app.default_state()

    def test_graph_is_balanced_and_every_link_names_real_ports(self):
        text = app.graph(dict(self.state, graves=6, amplitud=50, auriculares=40))
        body = re.sub(r'"[^"]*"', '""', text.split('\n', 1)[1])
        for a, b in ('{}', '[]'):
            self.assertEqual(body.count(a), body.count(b))
        names = set(re.findall(r'name = (\w+) label', text))
        self.assertEqual(len(names), 2*(len(app.BANDS)+3))
        ports = re.findall(r'"(\w+):(?:In|Out|In \d)"', text)
        self.assertTrue(set(ports) <= names, set(ports)-names)
        inputs = re.findall(r'input = "([^"]+)"', text)
        self.assertEqual(len(inputs), len(set(inputs)), 'each input port fed once')
        for key in app.controls(self.state):
            self.assertIn(key.split(':')[0], names)
        self.assertIn('filter.smart      = true', text)
        self.assertIn('Gain" = 6.0', text, 'saved values start in the graph')

    def test_off_is_bit_identical_routing(self):
        c = app.controls(dict(self.state, activo=False, graves=12, amplitud=100, auriculares=100))
        for key, value in c.items():
            if key.endswith(':Gain'): self.assertEqual(value, 0)
        self.assertEqual((c['mezcla_l:Gain 1'], c['mezcla_l:Gain 2'], c['mezcla_l:Gain 3']), (1.0, 0.0, 0.0))

    def test_no_combination_exceeds_full_scale(self):
        grid = [-12, 0, 6, 12]
        worst = 0
        for g, p, a, h in itertools.product([-6, 0, 12], grid, [0, 50, 100], [0, 50, 100]):
            state = dict(self.state, graves=g, presencia=p, amplitud=a, auriculares=h)
            worst = max(worst, worst_gain(state))
        # Peaking EQ boosts are compensated; the matrix keeps mono at unity.
        self.assertLessEqual(worst, 2.0001)
        mono = app.controls(dict(self.state, amplitud=100))
        self.assertLessEqual(mono['mezcla_l:Gain 1']+mono['mezcla_l:Gain 2'], 1.0)

    def test_width_and_crossfeed_shape(self):
        wide = app.controls(dict(self.state, amplitud=100))
        self.assertLess(wide['mezcla_l:Gain 2'], 0, 'width subtracts the opposite side')
        phones = app.controls(dict(self.state, auriculares=100))
        self.assertGreater(phones['mezcla_l:Gain 3'], 0)

    def test_profiles_are_inside_ranges(self):
        for name, values in app.PROFILES.items():
            self.assertEqual(set(values), set(app.RANGES), name)
            for key, value in values.items():
                self.assertEqual(app.clamp(key, value), value, (name, key))

    def test_clamp_and_garbage_state(self):
        self.assertEqual(app.clamp('graves', 99), 12.0)
        self.assertEqual(app.clamp('amplitud', -5), 0.0)
        with self.assertRaises(ValueError): app.clamp('graves', 'nan')
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp)/'sonido.json'
            path.write_text(json.dumps({'graves': 500, 'activo': 'si', 'perfil': '../x', 'otro': 1}))
            with patch.object(app, 'STATE', path):
                state = app.load()
        self.assertEqual((state['graves'], state['activo'], state['perfil']), (12.0, True, 'plano'))

    def test_live_update_uses_pw_cli_props_on_the_sink(self):
        calls = []
        class Done:
            returncode = 0
        with patch.object(app, 'node_id', return_value=57), \
             patch.object(app.subprocess, 'run', lambda args, **k: calls.append(args) or Done()):
            self.assertTrue(app.apply(dict(self.state, graves=3)))
        args = calls[0]
        self.assertEqual(args[:4], ['pw-cli', 'set-param', '57', 'Props'])
        self.assertIn('"graves_l:Gain" 3.0', args[4])
        self.assertIn('"mezcla_r:Gain 3"', args[4])


if __name__ == '__main__': unittest.main(verbosity=2)
