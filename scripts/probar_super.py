#!/usr/bin/env python3
"""Fixed action dispatch: untrusted launcher output cannot become shell code."""
import importlib.machinery
import importlib.util
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

ROOT=Path(__file__).resolve().parents[1]
loader=importlib.machinery.SourceFileLoader('super_test',str(ROOT/'src/rootfs/bin/ultra-super'))
spec=importlib.util.spec_from_loader(loader.name,loader);app=importlib.util.module_from_spec(spec);loader.exec_module(app)


class Super(unittest.TestCase):
    def test_fixed_selection_and_namespace(self):
        with patch.object(app.subprocess,'run',return_value=subprocess.CompletedProcess([],0,'2\n','')) as run:
            self.assertEqual(app.selected('menu'),['foot'])
        args=run.call_args.args[0]
        self.assertIn('--index',args);self.assertIn('--namespace=ultra-super-menu',args)
        self.assertIn('Super+Enter',run.call_args.kwargs['input'])
        self.assertNotIn('shell',run.call_args.kwargs)
    def test_cancel_dispatches_nothing(self):
        with patch.object(app.subprocess,'run',return_value=subprocess.CompletedProcess([],1,'','')):
            self.assertIsNone(app.selected('captura'))
    def test_invalid_or_injected_selection_rejected(self):
        for text in ('999','-1','0; touch /tmp/no','٠','1\n2'):
            with self.subTest(text=text),patch.object(app.subprocess,'run',return_value=subprocess.CompletedProcess([],0,text,'')):
                with self.assertRaises(ValueError):app.selected('menu')
    def test_catalogue_has_matching_runtime_bindings(self):
        lua=(ROOT/'src/rootfs/desktop/hyprland.lua').read_text()
        for key,cmd in [('SUPER + SPACE','ultra-super menu'),('SUPER + SHIFT + SPACE','ultra-ambiente menu'),
                        ('SUPER + F1','ultra-super atajos'),('SUPER + S','ultra-super captura')]:
            self.assertIn('hl.bind("'+key+'", hl.dsp.exec_cmd("'+cmd+'"))',lua)
    def test_input_shortcut_preserves_session_exit(self):
        lua=(ROOT/'src/rootfs/desktop/hyprland.lua').read_text()
        self.assertIn('hl.bind("SUPER + CTRL + M", hl.dsp.exec_cmd("ultra-entrada"))',lua)
        self.assertEqual(lua.count('hl.bind("SUPER + CTRL + M",'),1)
        self.assertIn('hl.bind("SUPER + SHIFT + M", hl.dsp.exit())',lua)
    def test_clipboard_shortcut_keeps_float_action_separate(self):
        lua=(ROOT/'src/rootfs/desktop/hyprland.lua').read_text()
        self.assertIn('hl.bind("SUPER + V", hl.dsp.exec_cmd("ultra-portapapeles menu"))',lua)
        self.assertEqual(lua.count('hl.bind("SUPER + V",'),1)
        self.assertIn('hl.bind("SUPER + CTRL + V", hl.dsp.window.float',lua)
    def test_workspace_menu_covers_nine_numbered_desktops(self):
        rows=app.MENUS['escritorios'][2]
        self.assertEqual([row[3] for row in rows],[['ultra-escritorio-ir',str(i)] for i in range(1,10)])


if __name__=='__main__':unittest.main(verbosity=2)
