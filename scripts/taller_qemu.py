#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Own QEMU workshop: localhost SSH, cloud-init and a separate Linux disk."""
import argparse, hashlib, io, json, os, pathlib, socket, subprocess, sys, time
ROOT=pathlib.Path(__file__).resolve().parents[1]
VM=ROOT/'vm'
QEMU=pathlib.Path(os.environ.get('ProgramFiles','C:/Program Files'))/'qemu'
HIDDEN=getattr(subprocess,'CREATE_NO_WINDOW',0)

def run(*args):
    return subprocess.run([str(a) for a in args],check=True,capture_output=True,text=True)

def free(port):
    with socket.socket() as s:
        try:s.bind(('127.0.0.1',port))
        except OSError:raise RuntimeError(f'Puerto local {port} ocupado; no se modifica el proceso existente.')

def prepare():
    free(14444)
    sys.path.insert(0,str(ROOT/'herramientas/python'))
    import pycdlib
    info=json.loads((ROOT/'evidencias/imagen-arch.json').read_text(encoding='utf-8-sig'))
    image=VM/info['Image']
    if hashlib.file_digest(image.open('rb'),'sha256').hexdigest().upper()!=info['SHA256'].upper():
        raise RuntimeError('La imagen base no coincide con su registro.')
    key=VM/'builder_ed25519'
    if not key.exists():run('ssh-keygen','-t','ed25519','-f',key,'-N','','-C','linux-reconstruido-qemu-local')
    pub=key.with_suffix('.pub').read_text().strip()
    archive=VM/'public/kernel-repo.tar'
    digest=hashlib.file_digest(archive.open('rb'),'sha256').hexdigest()
    user=f'''#cloud-config
hostname: linux-reconstruido-taller
users:
  - name: builder
    groups: [wheel]
    sudo: ["ALL=(ALL) NOPASSWD:ALL"]
    shell: /bin/bash
    lock_passwd: true
    ssh_authorized_keys:
      - {pub}
disable_root: true
ssh_pwauth: false
package_update: true
package_upgrade: true
packages:
  - base-devel
  - bc
  - cpio
  - libelf
  - ncurses
  - openssl
  - pahole
  - python
  - git
  - rsync
  - gdb
  - ccache
  - pkgconf
  - cmake
  - ninja
runcmd:
  - [bash, -lc, 'set -eu; mkdir -p /home/builder/linux-reconstruido/upstream; cd /home/builder/linux-reconstruido/upstream; curl --fail --retry 3 http://10.0.2.2:18765/kernel-repo.tar -o kernel-repo.tar; echo "{digest}  kernel-repo.tar" | sha256sum -c -; tar -xf kernel-repo.tar; rm kernel-repo.tar; git --git-dir=linux.git config core.symlinks true; git --git-dir=linux.git config core.filemode true; git --git-dir=linux.git config core.ignorecase false; git --git-dir=linux.git worktree add --detach ../kernel HEAD; chown -R builder:builder /home/builder/linux-reconstruido; touch /var/lib/cloud/reconstruido-ready']
final_message: "Linux Reconstruido: cloud-init finalizado; comprobar paquetes y repositorio antes de declarar listo."
'''
    meta='instance-id: linux-reconstruido-arch-v1\nlocal-hostname: linux-reconstruido-taller\n'
    iso=pycdlib.PyCdlib();iso.new(interchange_level=3,vol_ident='cidata',joliet=3,rock_ridge='1.09')
    for name,data,iso_name in [('user-data',user,'USERDATA.;1'),('meta-data',meta,'METADATA.;1')]:
        raw=data.encode();iso.add_fp(io.BytesIO(raw),len(raw),iso_path='/'+iso_name,rr_name=name,joliet_path='/'+name)
    iso.write(str(VM/'seed.iso'));iso.close()
    disk=VM/'taller.qcow2'
    if not disk.exists():run(QEMU/'qemu-img.exe','create','-f','qcow2','-F','qcow2','-b',image,disk,'64G')
    print('Seed y disco Linux de 64 GiB virtuales preparados. Base oficial conservada.')

def start(accel="whpx"):
    for port in [12222,18765,14444]:free(port)
    if not (VM/'seed.iso').exists():raise RuntimeError('Ejecutar prepare primero.')
    if accel == 'whpx':
        # The stdio monitor ignores piped input on Windows; a live process after 5 s means WHPX accepted the VM.
        probe=subprocess.Popen([str(QEMU/'qemu-system-x86_64.exe'),'-machine','pc','-accel','whpx','-cpu','host','-S','-m','128','-display','none','-monitor','none','-serial','none','-nodefaults'],stdout=subprocess.DEVNULL,stderr=subprocess.PIPE,text=True,creationflags=HIDDEN)
        try:
            _,detail=probe.communicate(timeout=5)
            raise RuntimeError('WHPX no disponible: comprobar SVM activado en BIOS y completar el reinicio de Windows tras habilitar HypervisorPlatform. El taller no se inicio. Detalle QEMU: '+detail.strip())
        except subprocess.TimeoutExpired:
            probe.kill();probe.communicate()
    clock=['-accel','whpx','-cpu','host'] if accel=='whpx' else ['-accel','tcg,thread=single','-icount','shift=auto','-cpu','qemu64']
    http_log=(VM/'http.log').open('ab')
    server=subprocess.Popen([sys.executable,'-m','http.server','18765','--bind','127.0.0.1','--directory',str(VM/'public')],stdout=http_log,stderr=subprocess.STDOUT,creationflags=HIDDEN)
    args=[str(QEMU/'qemu-system-x86_64.exe'),'-name','Linux Reconstruido - taller GCC','-machine','pc',*clock,'-smp','4','-m','4096','-drive',f'file={VM/"taller.qcow2"},format=qcow2,if=virtio,discard=unmap,detect-zeroes=unmap','-drive',f'file={VM/"seed.iso"},format=raw,media=cdrom,readonly=on','-device','virtio-net-pci,netdev=net0','-netdev','user,id=net0,hostfwd=tcp:127.0.0.1:12222-:22','-qmp','tcp:127.0.0.1:14444,server=on,wait=off','-serial',f'file:{VM/"serial.log"}','-display','none']
    log=(VM/'qemu.log').open('ab')
    try:proc=subprocess.Popen(args,stdout=log,stderr=subprocess.STDOUT,creationflags=HIDDEN)
    except Exception:server.terminate();raise
    time.sleep(2)
    if proc.poll() is not None:server.terminate();raise RuntimeError((VM/'qemu.log').read_text())
    (VM/'session.json').write_text(json.dumps({'QEMU':proc.pid,'HTTP':server.pid,'SSH':'127.0.0.1:12222','Accelerator':accel,'RAMMiB':4096,'VCPUs':4},indent=2))
    print('Taller iniciado. Instalacion interna en curso; SSH local puerto 12222.')

def ssh(command):
    key=VM/'builder_ed25519'
    return run('ssh','-i',key,'-p','12222','-o','BatchMode=yes','-o','KexAlgorithms=curve25519-sha256','-o','ConnectTimeout=30','-o','StrictHostKeyChecking=accept-new','-o',f'UserKnownHostsFile="{(VM/"known_hosts").as_posix()}"','builder@127.0.0.1',command)

def status():
    command="test -f /var/lib/cloud/reconstruido-ready && command -v gcc make ld bison flex bc gdb cmake ninja && test -f /home/builder/linux-reconstruido/kernel/Makefile && test -z \"$(git -C /home/builder/linux-reconstruido/kernel ls-files --deleted)\" && gcc --version | head -1 && git -C /home/builder/linux-reconstruido/kernel rev-parse HEAD"
    result=ssh(command)
    expected=json.loads((ROOT/'evidencias/origen-inicial.json').read_text(encoding='utf-8-sig'))['Commit']
    if expected not in result.stdout:raise RuntimeError('No se verifico el commit original dentro de la VM.')
    # Bootstrap diagnostics are recorded independently; an old error is never called a pass.
    bootstrap=ssh('cloud-init status --long; code=$?; printf "BOOTSTRAP_EXIT=%s\\n" "$code"; exit 0')
    report=result.stdout+'\nBOOTSTRAP DIAGNOSTIC (separate from tool/source checks):\n'+bootstrap.stdout
    print(report)
    (ROOT/'evidencias/taller-qemu.txt').write_text(report,encoding='utf-8')
    print('VERDE: herramientas y commit original comprobados; sin archivos ausentes. Compilacion del kernel pendiente.')
    if 'status: error' in bootstrap.stdout:
        print('AMARILLO: cloud-init conserva error historico del clon interrumpido; inicializador recuperado y comprobado manualmente. No se certifica un arranque limpio.')

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('action',choices=['prepare','start','status']);parser.add_argument('--accel',choices=['whpx','tcg'],default='whpx');args=parser.parse_args()
    try:
        if args.action=='start':start(args.accel)
        else:globals()[args.action]()
    except subprocess.CalledProcessError as exc:print('AMARILLO/ROJO:',exc.stderr or exc.stdout or str(exc),file=sys.stderr);sys.exit(1)
    except Exception as exc:print('AMARILLO/ROJO:',exc,file=sys.stderr);sys.exit(1)
