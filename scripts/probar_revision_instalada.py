#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Validacion y caducidad en la copia personal instalada; ejecutar como usuario."""
from pathlib import Path
import os
import subprocess
import json


def main():
    assert os.getuid()!=0
    command=['/usr/local/bin/ultra-revisar-proyecto','estado']
    result=subprocess.run(command,capture_output=True,text=True,timeout=20)
    assert result.returncode==0 and 'REVISION-VIGENTE' in result.stdout,(result.stdout,result.stderr)
    identifier=Path('/usr/share/ultra/proyecto-revisado/REVISION-ID').read_text().strip()
    root=Path(os.environ.get('XDG_DATA_HOME',str(Path.home()/'.local/share')))/'ultra/proyectos'/('ultra-'+identifier)
    def graph_state():
        response=subprocess.run(['/usr/local/bin/ultra-carpetas','--consulta','revision_estado',str(root),'{"limite":2}'],capture_output=True,text=True,timeout=50)
        assert response.returncode==0,(response.stdout,response.stderr)
        return json.loads(response.stdout)
    assert graph_state()['state']=='revisado'
    header=root/'src/init.h';original=header.read_bytes();metadata=header.stat()
    try:
        header.write_bytes((b' ' if original[:1]!=b' ' else b'X')+original[1:])
        os.utime(header,ns=(metadata.st_atime_ns,metadata.st_mtime_ns))
        changed=subprocess.run(command,capture_output=True,text=True,timeout=20)
        assert changed.returncode!=0 and 'REVISION-CADUCADA' in changed.stderr+changed.stdout
        assert graph_state()['state']=='caducado'
    finally:header.write_bytes(original)
    result=subprocess.run(command,capture_output=True,text=True,timeout=20)
    assert result.returncode==0 and 'REVISION-VIGENTE' in result.stdout
    status=subprocess.run(['/usr/local/bin/ultra-revisar-proyecto','estado-json'],capture_output=True,text=True,timeout=40)
    # The helper prepares paths before its JSON response; the engine line is last.
    body=json.loads(status.stdout.splitlines()[-1]);assert body['state']=='revisado' and body['production_certified'] is False
    record=root/'build/revision-proyecto/current.json';saved=record.read_bytes()
    try:
        value=json.loads(saved);key=next(iter(value['environment']['binaries_and_libraries']))
        value['environment']['binaries_and_libraries'][key]='0'*64;record.write_text(json.dumps(value))
        changed=subprocess.run(command,capture_output=True,text=True,timeout=40)
        assert changed.returncode!=0 and 'REVISION-CADUCADA' in changed.stderr+changed.stdout
    finally:record.write_bytes(saved)
    # A malicious project-local engine must not execute through the helper.
    local=root/'scripts/revisar-proyecto.py';saved_script=local.read_bytes();marker=root/'MOTOR-LOCAL-EJECUTADO'
    try:
        local.write_text('from pathlib import Path\nPath('+repr(str(marker))+').touch()\n')
        subprocess.run(command,capture_output=True,text=True,timeout=40)
        assert not marker.exists()
    finally:local.write_bytes(saved_script)
    result=subprocess.run(command,capture_output=True,text=True,timeout=40);assert result.returncode==0
    print('REVISION-INSTALADA-OK: cabecera con fecha/tamano conservados, identidad de herramienta y motor local distinguidos')


if __name__=='__main__':main()
