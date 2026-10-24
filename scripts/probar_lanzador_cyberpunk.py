#!/usr/bin/env python3
"""Exercise launcher boundaries with disposable stubs; no games, accounts or network."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT=Path(__file__).resolve().parents[1]

class Launch(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory()
        self.root=Path(self.temp.name); self.storage=self.root/'juegos'
        self.home=self.storage/'usuarios/ultra'; self.home.mkdir(parents=True)
        self.game=self.storage/'Cyberpunk2077/bin/x64/Cyberpunk2077.exe'
        self.game.parent.mkdir(parents=True); self.game.write_bytes(b'MZ-fixture')
        self.bin=self.root/'bin';self.bin.mkdir()
        self.record=self.root/'record'
        self.env=dict(os.environ,HOME=str(self.home),PATH=f'{self.bin}:'+os.environ['PATH'],RECORD=str(self.record),WINEPREFIX='preserved-sentinel')
        for name,body in {
            'id':'echo 1000', 'mountpoint':'exit 0',
            'df':"printf 'Filesystem 1024-blocks Used Available Capacity Mounted on\\nfixture 99999999 0 99999999 0 /fixture\\n'",
            'notify-send':':', 'ultra-espacio':'exit 0',
            'ultra-gog-cyberpunk':'exit ${REGISTER_RESULT:-0}',
            'ultra-hud':'shift; exec "$@"',
            'heroic':'printf "%s\\n" "$WINEPREFIX" "$@" > "$RECORD"',
            'umu-run':'printf "%s\\n" "$WINEPREFIX" "$PROTON_LOG" "$PROTON_LOG_DIR" "$@" > "$RECORD"; exit 17',
        }.items():
            path=self.bin/name;path.write_text('#!/usr/bin/bash\n'+body+'\n');path.chmod(0o755)
        self.script=self.root/'launch'
        source=(ROOT/'src/rootfs/bin/ultra-juegos').read_text()
        source=source.replace('/opt/heroic/heroic',str(self.bin/'heroic')).replace('/juegos',str(self.storage))
        self.script.write_text(source)
    def tearDown(self):self.temp.cleanup()
    def run_action(self,action='cyberpunk'):
        return subprocess.run(['/usr/bin/bash',str(self.script),action],env=self.env,capture_output=True,text=True)
    def test_normal_opens_manager_without_launching_game_or_second_prefix(self):
        result=self.run_action();self.assertEqual(result.returncode,0,result.stderr)
        self.assertEqual(self.record.read_text().splitlines(),['preserved-sentinel','--ozone-platform=wayland'])
        self.assertFalse((self.home/'Juegos/prefijos/cyberpunk2077').exists())
    def test_registration_failure_opens_manager_without_automatic_launch(self):
        self.env['REGISTER_RESULT']='1'
        result=self.run_action();self.assertEqual(result.returncode,0,result.stderr)
        self.assertEqual(self.record.read_text().splitlines(),['preserved-sentinel','--ozone-platform=wayland'])
    def test_missing_game_does_not_start_client_or_download(self):
        self.game.unlink();self.assertNotEqual(self.run_action().returncode,0)
        self.assertFalse(self.record.exists())
    def test_recovery_records_failure_and_diagnostic_without_replacing_prefix(self):
        result=self.run_action('cyberpunk-directo');self.assertEqual(result.returncode,17,result.stderr)
        rows=self.record.read_text().splitlines()
        self.assertEqual(rows[0],str(self.home/'Juegos/prefijos/cyberpunk2077'))
        self.assertEqual(rows[1],'1')
        self.assertIn('codigo 17',(self.home/'.cache/ultra/cyberpunk.log').read_text())

if __name__=='__main__':unittest.main(verbosity=2)
