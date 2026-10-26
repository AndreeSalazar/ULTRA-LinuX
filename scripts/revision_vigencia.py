#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Identidad de contenido y estados. No importa ni ejecuta codigo del proyecto."""
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import subprocess
import sys
import stat

ENV={'PATH':'/usr/bin:/bin','LC_ALL':'C','HOME':'/tmp'}
SYSTEM_ROOT=Path('/')


def system_mounts():
    args=[]
    for name in ['/usr','/lib','/lib64','/bin']:
        path=SYSTEM_ROOT/name.lstrip('/')
        if path.is_symlink():args+=['--symlink',os.readlink(path),name]
        elif path.exists():args+=['--ro-bind',str(path),name]
    return args


def digest(path,max_bytes=1073741824):
    fd=os.open(path,os.O_RDONLY|getattr(os,'O_NOFOLLOW',0)|getattr(os,'O_NONBLOCK',0))
    with os.fdopen(fd,'rb') as f:
        before=os.fstat(f.fileno())
        if not stat.S_ISREG(before.st_mode) or before.st_size>max_bytes:raise ValueError('Archivo no regular o fuera del presupuesto de identidad')
        result=hashlib.sha256();total=0
        while True:
            block=f.read(min(65536,max_bytes+1-total))
            if not block:break
            total+=len(block)
            if total>max_bytes:raise ValueError('Archivo crecio durante identidad')
            result.update(block)
        after=os.fstat(f.fileno())
        if (before.st_size,before.st_mtime_ns,before.st_ctime_ns)!=(after.st_size,after.st_mtime_ns,after.st_ctime_ns):
            raise ValueError('Archivo cambio durante identidad')
        return result.hexdigest()


def canonical(value):return hashlib.sha256(json.dumps(value,sort_keys=True,separators=(',',':')).encode()).hexdigest()


def system_path(path):
    logical=Path(path)
    if not logical.is_absolute() or not any(logical.is_relative_to(x) for x in ['/usr','/lib','/lib64']):raise ValueError('Ruta de sistema invalida')
    p=(SYSTEM_ROOT/str(logical).lstrip('/')).resolve(strict=True)
    if SYSTEM_ROOT!=Path('/') and not p.is_relative_to(SYSTEM_ROOT.resolve()):raise ValueError('Enlace de sistema escapa del contexto declarado')
    if not p.is_file() or not any(p.is_relative_to((SYSTEM_ROOT/x).resolve()) for x in ['usr','lib','lib64'] if (SYSTEM_ROOT/x).exists()):
        raise ValueError('Dependencia fuera de los directorios de sistema admitidos')
    return p


def environment(compilers):
    if SYSTEM_ROOT!=Path('/'):
        command=['/usr/bin/bwrap','--unshare-all','--die-with-parent','--new-session','--clearenv',*system_mounts(),
                 '--dev','/dev','--proc','/proc','--tmpfs','/tmp','--ro-bind',str(Path(__file__).resolve()),'/engine.py',
                 '--setenv','PATH','/usr/bin:/bin','--setenv','LC_ALL','C','/usr/bin/python3','-I','-S','/engine.py','--environment-json']
        return json.loads(subprocess.check_output(command,env=ENV,text=True,timeout=30))
    binaries={};versions={}
    for name in [*sorted(set(compilers)),'bwrap','python3']:
        if name not in ['gcc','g++','clang','clang++','bwrap','python3']:raise ValueError('Compilador desconocido')
        executable=system_path('/usr/bin/'+name);binaries[str(executable)]=digest(executable)
        if name in compilers:
            versions[name]=subprocess.check_output([str(executable),'--version'],env=ENV,text=True,timeout=10).splitlines()[0]
            if name in ['gcc','g++']:
                backend=subprocess.check_output([str(executable),'-print-prog-name='+('cc1plus' if name=='g++' else 'cc1')],env=ENV,text=True,timeout=10).strip()
                p=system_path(backend);binaries[str(p)]=digest(p)
    # ldd is only used on trusted, fixed system executables/backends, never project binaries.
    for name in list(binaries):
        result=subprocess.run(['/usr/bin/ldd',name],env=ENV,text=True,capture_output=True,timeout=10)
        if result.returncode!=0:raise ValueError('No se pudo identificar el runtime de '+name)
        for path in re.findall(r'(?:=>\s+|^\s*)(/[^\s]+)',result.stdout,re.M):
            p=system_path(path);binaries[str(p)]=digest(p)
    return {'versions':versions,'binaries_and_libraries':binaries}


def dependency_key(path,root):
    p=Path(os.path.normpath(str(path)))
    if p.is_relative_to('/project'):return 'project:'+str(p.relative_to('/project'))
    if p.is_relative_to(root):return 'project:'+str(p.relative_to(root))
    system_path(p)
    return 'system:'+str(p)


def dependency_path(key,root):
    if key.startswith('project:'):
        suffix=Path(key[8:])
        if suffix.is_absolute() or '..' in suffix.parts:raise ValueError('Ruta de dependencia invalida')
        p=root/suffix
        if not p.resolve(strict=True).is_relative_to(root.resolve()):raise ValueError('Dependencia del proyecto redirigida fuera')
        return p
    if key.startswith('system:'):return system_path(key[7:])
    raise ValueError('Dominio de dependencia desconocido')


def read_dependencies(file,root):
    if not file.is_file() or file.stat().st_size>1048576:raise ValueError('Falta el registro acotado de dependencias')
    text=file.read_text().replace('\\\n',' ')
    _,separator,body=text.partition(':')
    if not separator:raise ValueError('Dependencias sin regla Make')
    names=shlex.split(body)
    if not names or len(names)>4096:raise ValueError('Cantidad de dependencias invalida')
    return sorted({dependency_key(n,root) for n in names})


def hashes(keys,root):return {key:digest(dependency_path(key,root),16777216) for key in keys}


def assess(record,root,sources,config,env,engine):
    """Pure state decision except bounded dependency reads. Never equates warnings with proven UB."""
    expected={(p['file'],p['compiler'],p['phase']) for p in config}
    rows=record.get('checks',[])
    if record.get('schema')!=2 or record.get('kind')!='ultra-project-review' or record.get('production_certified') is not False:
        raise ValueError('Informe anterior o contrato desconocido; repetir revision')
    if not isinstance(rows,list) or len(rows)>5000:raise ValueError('Informe demasiado grande')
    by_key={}
    for row in rows:
        key=(row.get('file'),row.get('compiler'),row.get('phase'))
        if key in by_key:raise ValueError('Comprobacion duplicada')
        by_key[key]=row
    if any(k not in expected for k in by_key):raise ValueError('Comprobacion fuera del contrato esperado')
    reasons=[]
    if record.get('configuration_sha256')!=canonical(config):reasons.append('cambio configuracion o conjunto de unidades')
    if record.get('environment')!=env:reasons.append('cambio compilador, backend, runtime o bibliotecas del sistema')
    if record.get('engine_sha256')!=engine:reasons.append('cambio motor de revision')
    changed_sources={name for name in set(record.get('sources',{}))|set(sources) if record.get('sources',{}).get(name)!=sources.get(name)}
    dependencies=record.get('dependencies',{})
    if not isinstance(dependencies,dict) or len(dependencies)>10000:raise ValueError('Dependencias invalidas')
    changed=set()
    for key,want in dependencies.items():
        try:
            if not isinstance(want,str) or len(want)!=64 or digest(dependency_path(key,root),16777216)!=want:changed.add(key)
        except (OSError,ValueError):changed.add(key)
    if any('project:'+s not in dependencies for s in changed_sources):reasons.append('cambio fuente, cabecera, perfil o revisor fuera del registro de includes')
    output=[]
    for filename in sorted({p['file'] for p in config}):
        checks=[by_key.get(k) for k in sorted(expected) if k[0]==filename]
        why=list(reasons);state='revisado'
        if any(c is None for c in checks):state='pendiente';why.append('faltan comprobaciones requeridas')
        elif any(c.get('status')!='clean' for c in checks):
            state='diagnostico' if any(c.get('status')=='diagnostics' for c in checks) else 'pendiente'
            why.append('hay diagnosticos o una herramienta no completo la comprobacion')
        if reasons:state='caducado'
        for check in checks:
            if check is None:continue
            deps=check.get('dependencies')
            if not isinstance(deps,list) or not deps or any(k not in dependencies for k in deps):
                state='pendiente';why.append('dependencias de una comprobacion incompletas');continue
            affected=changed.intersection(deps)
            if affected:state='caducado';why.extend(sorted(affected)[:4])
        # The source itself is a mandatory dependency, even if an artifact was edited.
        if filename in changed_sources:state='caducado';why.append('cambio fuente '+filename)
        output.append({'file':filename,'state':state,'reasons':list(dict.fromkeys(why))[:8],'runtime_failure_confirmed':False})
    counts={s:sum(x['state']==s for x in output) for s in ['revisado','caducado','pendiente','diagnostico']}
    return {'schema':1,'state':'revisado' if output and counts['revisado']==len(output) else 'caducado' if counts['caducado'] else 'diagnostico' if counts['diagnostico'] else 'pendiente',
            'counts':counts,'units':output,'production_certified':False,'checked_content':True,
            'scope':'Revision estatica seleccionada; no prueba ausencia de UB ni confirma fallos de runtime.'}


if __name__=='__main__':
    if sys.argv[1:]!=['--environment-json']:raise SystemExit('Accion de identidad desconocida')
    print(json.dumps(environment(['gcc','clang'])))
