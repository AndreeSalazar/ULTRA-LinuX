#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Validacion y caducidad en la copia personal instalada; ejecutar como usuario."""
from pathlib import Path
import os
import subprocess


def main():
    assert os.getuid()!=0
    command=['/usr/local/bin/ultra-revisar-proyecto','estado']
    result=subprocess.run(command,capture_output=True,text=True,timeout=20)
    assert result.returncode==0 and 'REVISION-VIGENTE' in result.stdout,(result.stdout,result.stderr)
    identifier=Path('/usr/share/ultra/proyecto-revisado/REVISION-ID').read_text().strip()
    root=Path(os.environ.get('XDG_DATA_HOME',str(Path.home()/'.local/share')))/'ultra/proyectos'/('ultra-'+identifier)
    header=root/'src/init.h';original=header.read_bytes()
    try:
        header.write_bytes(original+b'\n/* cambio sintetico del banco de caducidad */\n')
        changed=subprocess.run(command,capture_output=True,text=True,timeout=20)
        assert changed.returncode!=0 and 'REVISION-CADUCADA' in changed.stderr+changed.stdout
    finally:header.write_bytes(original)
    result=subprocess.run(command,capture_output=True,text=True,timeout=20)
    assert result.returncode==0 and 'REVISION-VIGENTE' in result.stdout
    print('REVISION-INSTALADA-OK: UID sin root; fuente actual y cabecera cambiada/restaurada distinguidas')


if __name__=='__main__':main()
