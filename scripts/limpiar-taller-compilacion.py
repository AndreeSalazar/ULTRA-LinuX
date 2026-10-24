#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Only the QEMU workshop: plan by default; preserve sources and pinned packages.

Run after downloading and testing the built images. No personal storage paths.
"""
import argparse,json,os,shutil,socket,subprocess,time
from pathlib import Path

BASE=Path('/home/builder/linux-reconstruido')
PROFILES=('pc','generico','produccion','vigia')
TREES=('x86_64-pc','x86_64-generico','tiny64-baseline','tiny64-vigia','nvidia-open')

def run(*args):return subprocess.run(args,check=True,text=True,capture_output=True).stdout.strip()
def require(condition,message):
    if not condition:raise SystemExit(message)
def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--aplicar',action='store_true');args=parser.parse_args()
    if os.name!='posix' or os.geteuid()!=0 or socket.gethostname()!='linux-reconstruido-taller':raise SystemExit('Solo root en el taller QEMU identificado')
    require(BASE.resolve()==BASE and (BASE/'kernel/Makefile').is_file(),'Base del taller inesperada')
    require(subprocess.run(['systemctl','is-active','--quiet','linux-reconstruido-build']).returncode!=0,'Construccion activa')
    require(not Path('/var/lib/pacman/db.lck').exists(),'Pacman activo')
    require(shutil.rmtree.avoids_symlink_attacks,'rmtree sin proteccion de enlaces')
    mounts=[Path(line.split()[4].replace('\\040',' ')) for line in Path('/proc/self/mountinfo').read_text().splitlines()]
    targets=[BASE/'build'/name for name in TREES]
    for profile in PROFILES:
        art=BASE/'artifacts'/profile
        targets += [art/'sistema-a',art/'modulos']
        targets += [art/name for name in ('ultra-esp.img','disco-bmo.img','disco-bmo-doble.img','build-complete')]
    plan=[]
    for path in targets:
        if not path.exists():continue
        require(path.resolve()==path and path.is_relative_to(BASE) and not path.is_symlink(),'Destino fuera del taller o redirigido')
        require(not any(m==path or m.is_relative_to(path) for m in mounts),'Montaje bajo destino')
        size=int(run('du','-sx','-B1',str(path)).split()[0])
        info=path.stat()
        plan.append(dict(path=str(path),allocated_bytes=size,directory=path.is_dir(),uid=info.st_uid,gid=info.st_gid))
    cache=Path('/var/cache/pacman/pkg');require(cache.resolve()==cache,'Cache redirigida')
    cached=[]
    for path in cache.iterdir():
        if path.is_symlink() or not path.is_file():continue
        if any(tag in path.name for tag in ('nvidia','hyprland','mangohud')):continue
        if '.pkg.tar.' not in path.name:continue
        cached.append(dict(path=str(path),bytes=path.stat().st_size))
    result=dict(applied=False,targets=plan,package_cache=cached,preserved=['kernel','lab','experimentos no reconocidos','configuraciones guardadas','paquetes fijados NVIDIA/Hyprland/MangoHud','kernels, EROFS y manifiestos finales'])
    if args.aplicar:
        backup=BASE/'lab'/('configuraciones-antes-limpieza-'+time.strftime('%Y%m%d-%H%M%S'))
        require(backup.resolve()==backup,'Respaldo redirigido');backup.mkdir(mode=0o755)
        for name in TREES:
            for source in (BASE/'build'/name/'.config',BASE/'build'/name/'build.log'):
                if source.is_file() and not source.is_symlink():shutil.copy2(source,backup/(name+'-'+source.name.lstrip('.')))
        for row in plan:
            path=Path(row['path']);require(path.resolve()==path and path.is_relative_to(BASE),'Destino cambio desde el plan')
            if row['directory']:shutil.rmtree(path)
            else:path.unlink()
            # Kernel configuration is input, not disposable object code. Put it
            # back in its original O= directory so future builds keep all bits.
            saved=backup/(path.name+'-config')
            if path.parent==BASE/'build' and saved.is_file():
                path.mkdir(mode=0o755);os.chown(path,row['uid'],row['gid'])
                shutil.copy2(saved,path/'.config');os.chown(path/'.config',row['uid'],row['gid'])
        for row in cached:
            path=Path(row['path']);require(path.parent==cache and not path.is_symlink(),'Paquete redirigido');path.unlink()
        for profile in PROFILES:
            art=BASE/'artifacts'/profile
            if not art.is_dir():continue
            require(art.resolve()==art and art.is_relative_to(BASE),'Artefactos redirigidos')
            fd=os.open(art/'compilacion-limpiada.json',os.O_WRONLY|os.O_CREAT|os.O_TRUNC|os.O_NOFOLLOW,0o644)
            with os.fdopen(fd,'w') as f:json.dump(dict(rebuild_required_for_test_disks=True,final_kernel_and_erofs_preserved=True),f)
        result.update(applied=True,configuration_backup=str(backup),trim=run('fstrim','-v','/'))
        (BASE/'lab/limpieza-compilacion.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))
if __name__=='__main__':main()
