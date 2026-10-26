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
import shutil
sys.path.insert(0,str(Path(__file__).resolve().parent))
import revision_vigencia as identity

ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'build/revision-proyecto'


def limits():
    import resource
    resource.setrlimit(resource.RLIMIT_CPU,(45,45))
    resource.setrlimit(resource.RLIMIT_CORE,(0,0))
    resource.setrlimit(resource.RLIMIT_FSIZE,(16777216,16777216))
    resource.setrlimit(resource.RLIMIT_AS,(2147483648,2147483648))


def sha(path):return identity.digest(path,16777216)


def inputs():
    paths=list((ROOT/'src').glob('*.c'))+list((ROOT/'src').glob('*.h'))
    paths+=list((ROOT/'src/caja').glob('*.h'))+[ROOT/'src/caja/caja.c']
    paths+=list((ROOT/'src/guardian/produccion').glob('*.c'))+list((ROOT/'src/guardian/produccion').glob('*.h'))
    paths+=list((ROOT/'CASM/src').glob('*.c'))+list((ROOT/'CASM/src').glob('*.h'))
    paths += [ROOT/'src/guardian/ultra_guard.h',ROOT/'src/guardian/produccion/perfil-gcc.sh',ROOT/'scripts/revisar-proyecto.py',ROOT/'scripts/revision_vigencia.py']
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


def engine_hash():return identity.canonical([identity.digest(__file__),identity.digest(identity.__file__)])


def plans():
    profile=(ROOT/'src/guardian/produccion/perfil-gcc.sh').read_text()
    warn=shlex.split(re.search(r'ULTRA_WARN=\((.*?)\)',profile,re.S).group(1))
    if not warn or any(not re.fullmatch(r'-std=c(?:11|17|23)|-W(?![lap],)[A-Za-z0-9=+_-]+',x) for x in warn):
        raise ValueError('Opciones del perfil fuera del contrato de revision')
    result=[]
    for path in [p for p in inputs() if p.suffix=='.c']:
        rel=str(path.relative_to(ROOT));casm=rel.startswith('CASM/')
        for cc,phase in [('gcc','analyzer-O0'),('gcc','warnings-O2'),('clang','analyzer-O0'),('clang','warnings-O2')]:
            flags=(['-std=c11','-Wall','-Wextra','-Werror','-pedantic'] if casm else warn) if cc=='gcc' else ['-std=c11' if casm else '-std=c17','-Wall','-Wextra','-Wconversion','-Wshadow','-Wformat=2']
            flags=[*flags,'-O0' if phase.startswith('analyzer') else '-O2','-I/project/src']
            if cc=='gcc' and phase.startswith('analyzer'):flags+=['-fanalyzer']
            result.append({'file':rel,'compiler':cc,'phase':phase,'flags':flags})
    return result


def state():
    path=OUT/'current.json'
    if not path.exists():return {'schema':1,'state':'pendiente','counts':{},'units':[],'production_certified':False,'reason':'Sin informe'}
    if not path.is_file() or path.is_symlink() or path.stat().st_size>4194304:raise ValueError('Informe no regular o demasiado grande')
    record=json.loads(path.read_text())
    return identity.assess(record,ROOT,fingerprint(),plans(),identity.environment(['gcc','clang']),engine_hash())


def validate():
    result=state()
    if result['state']!='revisado':raise ValueError('REVISION-'+('CADUCADA' if result['state']=='caducado' else 'PENDIENTE')+': '+json.dumps(result.get('counts',{})))
    print('REVISION-VIGENTE: contenido, dependencias, configuracion y herramientas coinciden; alcance estatico seleccionado')


def review():
    if not sys.platform.startswith('linux'):raise ValueError('Revisar requiere Linux; inventario tambien funciona en Windows')
    if os.getuid()==0:raise ValueError('Revisar como usuario sin privilegios, nunca root')
    if not Path('/usr/bin/bwrap').is_file():raise ValueError('Bubblewrap obligatorio: no se revisa sin aislamiento')
    OUT.mkdir(parents=True,exist_ok=True);run=Path(tempfile.mkdtemp(prefix='run-',dir=OUT))
    env=identity.environment(['gcc','clang']);snapshot=fingerprint();rows=[];configuration=plans();dependencies={};jobs=[]

    def invoke(job,args,logname,project):
        sandbox=['/usr/bin/bwrap','--unshare-all','--die-with-parent','--new-session','--clearenv']
        sandbox+=identity.system_mounts()
        sandbox+=['--dev','/dev','--proc','/proc','--tmpfs','/tmp','--ro-bind',str(project),'/project',
                  '--bind',str(job),'/work','--chdir','/work','--setenv','PATH','/usr/bin:/bin','--setenv','LC_ALL','C']
        with (job/logname).open('wb') as log:
            child=subprocess.Popen(sandbox+args,stdout=log,stderr=subprocess.STDOUT,start_new_session=True,preexec_fn=limits,env=identity.ENV)
            try:return child.wait(timeout=60)
            except subprocess.TimeoutExpired:os.killpg(child.pid,signal.SIGKILL);child.wait();return None

    # Discover the include closure first, then freeze project bytes once for all passes.
    for plan in configuration:
        rel,cc,phase=plan['file'],plan['compiler'],plan['phase']
        job=run/(rel.replace('/','_')+'-'+cc+'-'+phase);job.mkdir();jobs.append(job)
        args=['/usr/bin/'+cc,*plan['flags'],str(Path('/project')/rel),'-M','-MF','/work/before.d','-MT','unit']
        if invoke(job,args,'dependencies.txt',ROOT)!=0:raise ValueError('No se pudo registrar la dependencia de '+rel+'; ver '+str(job))
        keys=identity.read_dependencies(job/'before.d',ROOT);current=identity.hashes(keys,ROOT)
        for key,value in current.items():
            if key in dependencies and dependencies[key]!=value:raise ValueError('Dependencia cambio durante descubrimiento: '+key)
            dependencies[key]=value
    frozen=run/'snapshot';frozen.mkdir()
    for key in sorted(set(dependencies)|{'project:'+n for n in snapshot}):
        if not key.startswith('project:'):continue
        source=identity.dependency_path(key,ROOT);dest=frozen/key[8:];dest.parent.mkdir(parents=True,exist_ok=True)
        shutil.copyfile(source,dest)
        if identity.digest(dest)!=(dependencies.get(key) or snapshot[key[8:]]):raise ValueError('Fuente cambio durante fotografia: '+key)
    if snapshot!=fingerprint():raise ValueError('Fuentes cambiaron durante fotografia')
    for plan,job in zip(configuration,jobs):
        rel,cc,phase=plan['file'],plan['compiler'],plan['phase']
        args=['/usr/bin/'+cc,*plan['flags'],str(Path('/project')/rel),'-MD','-MF','/work/after.d','-MT','unit']
        if cc=='clang' and phase.startswith('analyzer'):args+=['--analyze','-Xclang','-analyzer-output=sarif','-o','/work/analysis.sarif']
        else:args+=['-c','-o','/dev/null']
        code=invoke(job,args,'diagnostics.txt',frozen)
        with (job/'diagnostics.txt').open('rb') as log:raw=log.read(1048577)
        text=raw.decode('utf-8','replace');findings=0;keys=[]
        if (job/'analysis.sarif').is_file():
            doc=json.loads((job/'analysis.sarif').read_text());findings=sum(len(r.get('results',[])) for r in doc.get('runs',[]))
        status='timeout' if code is None else 'tool_failed' if code else 'output_limit' if len(raw)>1048576 else 'diagnostics' if findings or re.search(r'\b(warning|error):',text) else 'clean'
        if code==0:
            try:
                keys=identity.read_dependencies(job/'after.d',ROOT)
                if keys!=identity.read_dependencies(job/'before.d',ROOT) or any(k not in dependencies for k in keys):status='dependencies_changed'
            except ValueError:status='dependencies_missing'
        rows.append({'file':rel,'compiler':cc,'phase':phase,'status':status,'exit_code':code,'findings':findings,
                     'flags':args[1:],'dependencies':keys,'log':str((job/'diagnostics.txt').relative_to(ROOT))})
    if snapshot!=fingerprint() or identity.hashes(dependencies,ROOT)!=dependencies or identity.environment(['gcc','clang'])!=env:
        for row in rows:row['status']='environment_changed_during_review'
    report={'schema':2,'kind':'ultra-project-review','created_unix':int(time.time()),'sources':snapshot,'toolchains':env['versions'],
            'environment':env,'configuration_sha256':identity.canonical(configuration),'engine_sha256':engine_hash(),'dependencies':dependencies,
            'inventory':inventory(),'checks':rows,'production_certified':False,
            'scope':'Own selected C translation units and headers. No kernel, upstream libraries or games. No production programs executed.'}
    (run/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    (run/'current.pending.json').write_text(json.dumps(report,indent=2)+'\n');(run/'current.pending.json').replace(OUT/'current.json')
    counts={s:sum(c['status']==s for c in rows) for s in sorted({c['status'] for c in rows})}
    print(json.dumps({'report':str(run/'report.json'),'translation_units':len({p['file'] for p in configuration}),'dependencies':len(dependencies),'checks':counts},indent=2))
    return 0 if counts.get('clean',0)==len(rows) else 1


def main():
    global ROOT,OUT
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('action',choices=['inventario','preparar','revisar','validar','estado-json']);p.add_argument('--raiz',type=Path);p.add_argument('--sistema',type=Path);p.add_argument('--salida',type=Path);a=p.parse_args()
    if a.raiz:ROOT=a.raiz.resolve(strict=True);OUT=ROOT/'build/revision-proyecto'
    if a.sistema:
        system=a.sistema.resolve(strict=True)
        if not (system/'ULTRA-SISTEMA').is_file():raise ValueError('Contexto de sistema no identificado')
        identity.SYSTEM_ROOT=system
    if a.salida:OUT=a.salida.resolve();OUT.mkdir(parents=True,exist_ok=True)
    if a.action=='inventario':print(json.dumps(inventory(),indent=2));return 0
    if a.action=='preparar':
        OUT.mkdir(parents=True,exist_ok=True)
        commands=[]
        for path in inputs():
            if path.suffix=='.c':commands.append({'directory':str(ROOT),'file':str(path),'arguments':['gcc','-std=c11' if path.is_relative_to(ROOT/'CASM') else '-std=c17','-I'+str(ROOT/'src'),'-c',str(path)]})
        (ROOT/'build/compile_commands.json').write_text(json.dumps(commands,indent=2)+'\n')
        print('SUPER-GRAFO: base de compilacion preparada para',len(commands),'unidades');return 0
    if a.action=='validar':validate();return 0
    if a.action=='estado-json':
        try:result=state()
        except (ValueError,OSError,KeyError,TypeError,subprocess.SubprocessError) as error:result={'schema':1,'state':'pendiente','counts':{},'units':[],'production_certified':False,'reason':str(error)[:400]}
        print(json.dumps(result));return 0
    return review()


if __name__=='__main__':
    try:raise SystemExit(main())
    except (ValueError,OSError,subprocess.SubprocessError) as error:raise SystemExit('ROJO [REVISION] '+str(error))
