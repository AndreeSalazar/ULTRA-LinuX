#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Inventario y revision estatica reproducible de las fuentes propias de ULTRA."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import signal
import subprocess
import sys
import tempfile
import time

ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'build/revision-proyecto'


def limits():
    import resource
    resource.setrlimit(resource.RLIMIT_CPU,(45,45))
    resource.setrlimit(resource.RLIMIT_CORE,(0,0))
    resource.setrlimit(resource.RLIMIT_FSIZE,(16777216,16777216))
    resource.setrlimit(resource.RLIMIT_AS,(2147483648,2147483648))


def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()


def inputs():
    paths=list((ROOT/'src').glob('*.c'))+list((ROOT/'src').glob('*.h'))
    paths+=list((ROOT/'src/caja').glob('*.h'))+[ROOT/'src/caja/caja.c']
    paths+=list((ROOT/'src/guardian/produccion').glob('*.c'))+list((ROOT/'src/guardian/produccion').glob('*.h'))
    paths+=list((ROOT/'CASM/src').glob('*.c'))+list((ROOT/'CASM/src').glob('*.h'))
    paths += [ROOT/'src/guardian/ultra_guard.h',ROOT/'src/guardian/produccion/perfil-gcc.sh',Path(__file__).resolve()]
    return sorted(set(paths))


def fingerprint():return {str(p.relative_to(ROOT)).replace('\\','/'):sha(p) for p in inputs()}


def inventory():
    selected={str(p.relative_to(ROOT)).replace('\\','/') for p in inputs() if p.suffix=='.c'}
    rows=[]
    for base in [ROOT/'src',ROOT/'CASM']:
        for folder,dirs,files in os.walk(base,followlinks=False):
            dirs[:]=[d for d in dirs if d not in ['target','build','__pycache__','.git','vendor'] and not (Path(folder)/d).is_symlink()]
            for name in files:
                p=Path(folder)/name
                if p.suffix not in ['.c','.cpp','.cc','.cxx','.h','.rs','.casm','.S']:continue
                rel=str(p.relative_to(ROOT)).replace('\\','/')
                category='propio-revisable' if rel in selected else 'cabecera-propia' if p in inputs() else 'otro-componente'
                if '/tests/' in rel or '/laboratorio/' in rel or rel in ['src/guardian/minas.c']:
                    category='laboratorio-no-produccion'
                elif p.suffix=='.rs':category='rust-no-revisado-por-clang'
                elif p.suffix=='.casm':category='casm-verificador-propio'
                elif rel=='src/guardian/ultra_guard.c':category='puente-incluye-implementacion-produccion'
                rows.append({'file':rel,'category':category})
    return sorted(rows,key=lambda x:x['file'])


def validate():
    record=json.loads((OUT/'current.json').read_text())
    if record['sources']!=fingerprint():raise ValueError('REVISION-CADUCADA: cambiaron fuentes, cabeceras, perfil o revisor')
    if any(c['status']!='clean' for c in record['checks']):raise ValueError('REVISION-PENDIENTE: hay diagnosticos o fallos de herramientas')
    # Version changes invalidate evidence even if source bytes are identical.
    for cc,version in record['toolchains'].items():
        if subprocess.check_output(['/usr/bin/'+cc,'--version'],text=True,timeout=10).splitlines()[0]!=version:
            raise ValueError('REVISION-CADUCADA: cambio '+cc)
    print('REVISION-VIGENTE: fuentes y herramientas coinciden; solo alcance estatico propio')


def review():
    if not sys.platform.startswith('linux'):raise ValueError('Revisar requiere Linux; inventario tambien funciona en Windows')
    if os.getuid()==0:raise ValueError('Revisar como usuario sin privilegios, nunca root')
    if not Path('/usr/bin/bwrap').is_file():raise ValueError('Bubblewrap obligatorio: no se revisa sin aislamiento')
    OUT.mkdir(parents=True,exist_ok=True);run=Path(tempfile.mkdtemp(prefix='run-',dir=OUT))
    versions={c:subprocess.check_output(['/usr/bin/'+c,'--version'],text=True,timeout=10).splitlines()[0] for c in ['gcc','clang']}
    snapshot=fingerprint();rows=[];commands=[]
    profile=(ROOT/'src/guardian/produccion/perfil-gcc.sh').read_text()
    warn=shlex.split(re.search(r'ULTRA_WARN=\((.*?)\)',profile,re.S).group(1))
    for path in [p for p in inputs() if p.suffix=='.c']:
        rel=str(path.relative_to(ROOT));casm=rel.startswith('CASM/')
        flags=['-std=c11','-Wall','-Wextra','-Werror','-pedantic'] if casm else warn
        commands.append({'directory':str(ROOT),'file':str(path),'arguments':['gcc',*flags,'-I'+str(ROOT/'src'),'-c',str(path)]})
        for cc,phase in [('gcc','analyzer-O0'),('gcc','warnings-O2'),('clang','analyzer-O0'),('clang','warnings-O2')]:
            job=run/(rel.replace('/','_')+'-'+cc+'-'+phase);job.mkdir()
            opt='-O0' if phase.startswith('analyzer') else '-O2'
            base=flags if cc=='gcc' else ['-std=c11' if casm else '-std=c17','-Wall','-Wextra','-Wconversion','-Wshadow','-Wformat=2']
            args=['/usr/bin/'+cc,*base,opt,'-I/project/src',str(Path('/project')/rel)]
            if cc=='gcc':args+=['-fanalyzer'] if phase.startswith('analyzer') else []
            if cc=='clang' and phase.startswith('analyzer'):args+=['--analyze','-Xclang','-analyzer-output=sarif','-o','/work/analysis.sarif']
            else:args+=['-c','-o','/dev/null']
            sandbox=['/usr/bin/bwrap','--unshare-all','--die-with-parent','--new-session','--clearenv']
            for d in ['/usr','/lib','/lib64','/bin']:
                if Path(d).exists():sandbox+=['--ro-bind',d,d]
            sandbox+=['--dev','/dev','--proc','/proc','--tmpfs','/tmp','--ro-bind',str(ROOT),'/project',
                      '--bind',str(job),'/work','--chdir','/work','--setenv','PATH','/usr/bin:/bin','--setenv','LC_ALL','C']
            with (job/'diagnostics.txt').open('wb') as log:
                child=subprocess.Popen(sandbox+args,stdout=log,stderr=subprocess.STDOUT,start_new_session=True,preexec_fn=limits)
                try:code=child.wait(timeout=60)
                except subprocess.TimeoutExpired:os.killpg(child.pid,signal.SIGKILL);child.wait();code=None
            with (job/'diagnostics.txt').open('rb') as log:text=log.read(1048577).decode('utf-8','replace')
            findings=0
            if (job/'analysis.sarif').is_file():
                doc=json.loads((job/'analysis.sarif').read_text());findings=sum(len(r.get('results',[])) for r in doc.get('runs',[]))
            status='timeout' if code is None else 'tool_failed' if code else 'diagnostics' if findings or re.search(r'\b(warning|error):',text) else 'clean'
            rows.append({'file':rel,'compiler':cc,'phase':phase,'status':status,'exit_code':code,'findings':findings,
                         'flags':args[1:],'log':str((job/'diagnostics.txt').relative_to(ROOT))})
    report={'schema':1,'kind':'ultra-project-review','created_unix':int(time.time()),'sources':snapshot,'toolchains':versions,
            'inventory':inventory(),'checks':rows,'production_certified':False,
            'scope':'Own selected C translation units and headers. No kernel, upstream libraries or games. No production programs executed.'}
    (run/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    (run/'current.pending.json').write_text(json.dumps(report,indent=2)+'\n');(run/'current.pending.json').replace(OUT/'current.json')
    (ROOT/'build/compile_commands.json').write_text(json.dumps(commands,indent=2)+'\n')
    counts={s:sum(c['status']==s for c in rows) for s in sorted({c['status'] for c in rows})}
    print(json.dumps({'report':str(run/'report.json'),'translation_units':len(commands),'checks':counts},indent=2))
    return 0 if counts.get('clean',0)==len(rows) else 1


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('action',choices=['inventario','preparar','revisar','validar']);a=p.parse_args()
    if a.action=='inventario':print(json.dumps(inventory(),indent=2));return 0
    if a.action=='preparar':
        OUT.mkdir(parents=True,exist_ok=True)
        commands=[]
        for path in inputs():
            if path.suffix=='.c':commands.append({'directory':str(ROOT),'file':str(path),'arguments':['gcc','-std=c11' if path.is_relative_to(ROOT/'CASM') else '-std=c17','-I'+str(ROOT/'src'),'-c',str(path)]})
        (ROOT/'build/compile_commands.json').write_text(json.dumps(commands,indent=2)+'\n')
        print('SUPER-GRAFO: base de compilacion preparada para',len(commands),'unidades');return 0
    if a.action=='validar':validate();return 0
    return review()


if __name__=='__main__':
    try:raise SystemExit(main())
    except (ValueError,OSError,subprocess.SubprocessError) as error:raise SystemExit('ROJO [REVISION] '+str(error))
