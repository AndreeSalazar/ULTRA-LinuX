#!/usr/bin/env python3
"""A command echoed by the tty must never make a failed check pass."""
import contextlib
import importlib.util
import io
from pathlib import Path
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('boot_bank', Path(__file__).with_name('probar_arranque.py'))
bank = importlib.util.module_from_spec(spec); spec.loader.exec_module(bank)


class Output(unittest.TestCase):
    def check(self, actual):
        sent = []
        class Connection:
            def sendall(self, data): sent.append(data.decode())
        def response(conn, marker, timeout):
            token = marker.removeprefix('ULTRA-CHECK-END-')
            return sent[-1]+'\nULTRA-CHECK-BEGIN-'+token+'\n'+actual+'\n'+marker+'\nultra:~# '
        failures = []
        with patch.object(bank, 'read_until', side_effect=response), contextlib.redirect_stdout(io.StringIO()):
            bank.run(Connection(), [('grep missing && echo SUCCESS', 'SUCCESS')], failures)
        return failures

    def test_echoed_command_does_not_count(self):
        self.assertEqual(self.check('grep: missing: No such file'), ['grep missing && echo SUCCESS'])

    def test_execution_output_does_count(self):
        self.assertEqual(self.check('SUCCESS'), [])


if __name__ == '__main__': unittest.main(verbosity=2)
