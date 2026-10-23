#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

source=Path(sys.argv[1]).resolve()
categories=[('1 Aplicaciones','Navegador Brave','ultra-brave','00d4e8ff'),
            ('2 Discos y bibliotecas','Panel de discos y preparacion','foot','2de2b8ff'),
            ('3 Juegos','Steam','ultra-juegos','76ff58ff'),
            ('4 Pantalla y graficos','Resolucion, Hz y escala','ultra-pantallas','5298ffff'),
            ('5 Audio','Volumen 100%','ultra-audio','ffc857ff'),
            ('6 Ambiente','Limpiar notificaciones','makoctl','c49affff'),
            ('7 Sistema y ayuda','Ayuda y atajos','foot','ff657aff')]
with tempfile.TemporaryDirectory() as directory:
    root=Path(directory); binary=root/'bin'; binary.mkdir(); runtime=root/'runtime'; runtime.mkdir()
    fuzzel=binary/'fuzzel'
    fuzzel.write_text('''#!/usr/bin/python
import json,os,sys
from pathlib import Path
root=Path(os.environ['MENU_CASE']); path=root/'queue.json'
queue=json.loads(path.read_text()); value=queue.pop(0); path.write_text(json.dumps(queue))
rows=sys.stdin.read().splitlines()
with (root/'dialogs.jsonl').open('a') as log: log.write(json.dumps(dict(args=sys.argv[1:],rows=rows))+'\\n')
if value is None: sys.exit(1)
print(value)
'''); fuzzel.chmod(0o755)
    identity=binary/'id'; identity.write_text('#!/bin/sh\necho "${MENU_UID:-1000}"\n'); identity.chmod(0o755)
    for name in ['foot','ultra-brave','ultra-juegos','ultra-pantallas','ultra-audio','makoctl','notify-send','hyprctl']:
        command=binary/name
        command.write_text('#!/bin/sh\nprintf "%s\\n" "'+name+' $*" >> "$MENU_CASE/launched"\n')
        command.chmod(0o755)
    env=dict(os.environ,PATH=str(binary)+':/usr/bin',XDG_RUNTIME_DIR=str(runtime),MENU_CASE=str(root))
    def execute(queue):
        (root/'queue.json').write_text(json.dumps(queue))
        for name in ['launched','dialogs.jsonl']: (root/name).unlink(missing_ok=True)
        result=subprocess.run(['bash',str(source),'menu'],env=env,capture_output=True,text=True,timeout=4)
        assert result.returncode==0,result.stderr
        dialogs=[json.loads(line) for line in (root/'dialogs.jsonl').read_text().splitlines()]
        launched=(root/'launched').read_text() if (root/'launched').exists() else ''
        return dialogs,launched
    for label,choice,command,color in categories:
        dialogs,launched=execute([label,choice])
        assert '--border-color='+color in dialogs[1]['args']
        assert choice in dialogs[1]['rows'] and launched.startswith(command+' ')
        assert '--only-match' in dialogs[0]['args']
        if command=='foot' and label.startswith('2 '): assert '--fullscreen' in launched
        print('PASS color y ruta:',label)
    dialogs,launched=execute(['3 Juegos',None,None])
    assert len(dialogs)==3 and not launched
    assert '--select-index=2' in dialogs[2]['args']
    print('PASS Escape vuelve a raiz y conserva categoria')
    dialogs,launched=execute(['5 Audio','Volver a categorias',None])
    assert len(dialogs)==3 and not launched
    print('PASS Volver no ejecuta acciones')
    dialogs,launched=execute(['1 Aplicaciones','$(touch /tmp/menu-inyectado)'])
    assert not launched
    print('PASS entrada libre no se ejecuta')
    env['MENU_UID']='0'
    result=subprocess.run(['bash',str(source),'menu'],env=env,capture_output=True,timeout=4)
    assert result.returncode==1
    print('PASS root rechazado')
print('RESULTADO: 11 controles')
