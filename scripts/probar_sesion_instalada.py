#!/usr/bin/env python3
"""Real restart/poweroff via UID1000 requests, on an isolated virtual disk."""
import importlib.util
import json
from pathlib import Path
import re
import shutil
import socket
import subprocess
import sys
import time

spec=importlib.util.spec_from_file_location('boot',Path(__file__).with_name('probar_arranque.py'))
boot=importlib.util.module_from_spec(spec);spec.loader.exec_module(boot)


def main():
    boot.check_kernel()
    assert boot.BMO and boot.BUILD.is_relative_to(boot.ROOT/'build')
    image=boot.BUILD/'disco-sesion-prueba.img';firmware=boot.BUILD/'ovmf-vars-sesion.fd'
    shutil.copyfile(boot.IMG,image);shutil.copyfile(boot.QEMU_SHARE/'edk2-i386-vars.fd',firmware)
    transcript='';failures=[];proc=None;port=17555
    def start():
        process=subprocess.Popen([str(boot.QEMU),'-machine','pc','-accel','whpx','-cpu','host','-smp','2','-m','3072',
            '-drive',f'if=pflash,format=raw,readonly=on,file={boot.QEMU_SHARE / "edk2-x86_64-code.fd"}',
            '-drive',f'if=pflash,format=raw,file={firmware}','-drive',f'file={image},format=raw,if=virtio',
            '-device','intel-hda','-vga','virtio','-nic','none','-display','none','-no-reboot',
            '-serial',f'tcp:127.0.0.1:{port},server=on,wait=on','-monitor',f'tcp:127.0.0.1:{port+1},server=on,wait=off'],
            stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        for _ in range(50):
            try:connection=socket.create_connection(('127.0.0.1',port),timeout=1);break
            except OSError:time.sleep(.2)
        else:process.kill();raise RuntimeError('No se abrio consola QEMU')
        connection.settimeout(.5)
        return process,connection
    try:
        for action in ('reiniciar','apagar'):
            proc,conn=start();transcript+=boot.read_until(conn,'ultra:',120)
            def command(text,expected):
                nonlocal transcript
                out=boot.run(conn,[(text,expected)],failures);transcript+=out;return out
            if action=='apagar':
                command("grep -q 'HERONITE: ultimo inicio' /bmo/ULTRA/informe/gpu-$(uname -r).txt && echo INFORME-PREVIO-$((90+1))",'INFORME-PREVIO-91')
            command('ultra-escritorio --hyprland --software >/tmp/power-sesion.log 2>&1 & echo ESCRITORIO-$((90+2))','ESCRITORIO-92')
            ipc='ultra-entrar /usr/bin/env XDG_RUNTIME_DIR=/run/user/1000 /usr/bin/hyprctl -i 0 '
            for _ in range(60):
                raw=command(ipc+"-j clients; echo SESION-IPC-$((90+3))",'SESION-IPC-93').replace('\r','')
                match=re.search(r'(?m)^\s*\[\s*(?:\{|\])',raw)
                if match and json.JSONDecoder().raw_decode(raw[match.start():].lstrip())[0]:break
                time.sleep(.5)
            else:raise RuntimeError('No abrio el escritorio')
            user='ultra-entrar /usr/bin/setpriv --reuid=1000 --regid=1000 --clear-groups --no-new-privs --bounding-set=-all /usr/local/bin/ultra-sesion-control '
            command(user+'estado','"activo": true')
            if '--suspender' in sys.argv and action=='reiniciar':
                command(user+'solicitar suspender >/tmp/power-suspend.log 2>&1 & echo SUSPENSION-SOLICITADA-$((90+4))','SUSPENSION-SOLICITADA-94')
                time.sleep(3)
                with socket.create_connection(('127.0.0.1',port+1),timeout=5) as monitor:
                    monitor.sendall(b'system_wakeup\n');time.sleep(2)
                command('ultra-entrar /usr/bin/cat /run/ultra-sesion-control/resultado.json','"fase": "reanudado"')
            command(user+'solicitar '+action,'"ok": true')
            ending=boot.read_until(conn,'Restarting system' if action=='reiniciar' else 'DESMONTAJE]: /bmo desmontado limpio',60)
            transcript+=ending;proc.wait(timeout=30);conn.close()
            if proc.returncode!=0:raise RuntimeError('QEMU termino con error')
            print('VERDE: solicitud UID1000 '+action+' cerro escritorio y termino QEMU',flush=True)
        if failures:raise RuntimeError('Fallo de comprobaciones: '+repr(failures))
        record={'restart_request_uid':1000,'poweroff_request_uid':1000,'report_exported_before_restart':True,
                'restart_qemu_exit':0,'poweroff_qemu_exit':0,'suspend_resume_tested':'--suspender' in sys.argv,
                'physical_nvidia_suspend_tested':False}
        (boot.BUILD/'sesion-real.json').write_text(json.dumps(record,indent=2)+'\n')
    finally:
        if proc and proc.poll() is None:proc.kill();proc.wait()
        (boot.BUILD/'sesion-real.log').write_text(transcript,encoding='utf-8')


if __name__=='__main__':main()
