#!/usr/bin/env python3
"""Reach Discord's main window in a disposable QEMU, without an account."""
import importlib.util
import json
from pathlib import Path
import re
import shutil
import socket
import subprocess
import time

spec = importlib.util.spec_from_file_location('boot', Path(__file__).with_name('probar_arranque.py'))
boot = importlib.util.module_from_spec(spec); spec.loader.exec_module(boot)


def main():
    boot.check_kernel()
    assert boot.BMO and boot.BUILD.is_relative_to(boot.ROOT/'build')
    image = boot.BUILD/'disco-discord-prueba.img'
    firmware = boot.BUILD/'ovmf-vars-discord.fd'
    shutil.copyfile(boot.IMG, image)
    shutil.copyfile(boot.QEMU_SHARE/'edk2-i386-vars.fd', firmware)
    port = 16555
    transcript = ''
    failures = []
    proc = subprocess.Popen([str(boot.QEMU), '-machine', 'pc', '-accel', 'whpx', '-cpu', 'host',
        '-smp', '2', '-m', '4096', '-drive', f'if=pflash,format=raw,readonly=on,file={boot.QEMU_SHARE / "edk2-x86_64-code.fd"}',
        '-drive', f'if=pflash,format=raw,file={firmware}', '-drive', f'file={image},format=raw,if=virtio',
        '-device', 'intel-hda', '-vga', 'virtio', '-nic', 'user,model=virtio-net-pci',
        '-display', 'none', '-no-reboot', '-serial', f'tcp:127.0.0.1:{port},server=on,wait=on',
        '-monitor', f'tcp:127.0.0.1:{port+1},server=on,wait=off'],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(50):
            try: conn = socket.create_connection(('127.0.0.1', port), timeout=1); break
            except OSError: time.sleep(.2)
        else: raise RuntimeError('No se abrio la consola QEMU')
        conn.settimeout(.5)
        transcript += boot.read_until(conn, 'ultra:', 120)
        def command(text, expected):
            nonlocal transcript
            result = boot.run(conn, [(text, expected)], failures); transcript += result
            return result
        command('mkdir -p /arch/juegos/usuarios/ultra; chown 1000:1000 /arch/juegos/usuarios/ultra; echo HOME-$((20+2))', 'HOME-22')
        command('ultra-escritorio --hyprland --software >/tmp/discord-sesion.log 2>&1 & echo SESION-$((20+3))', 'SESION-23')
        ipc = 'ultra-entrar /usr/bin/env XDG_RUNTIME_DIR=/run/user/1000 /usr/bin/hyprctl -i 0 '
        def clients():
            raw = command(ipc+"-j clients; printf 'DISCORD-IPC-%s\\n' $((20+5))", 'DISCORD-IPC-25').replace('\r', '')
            match = re.search(r'(?m)^\s*\[\s*(?:\{|\])', raw)
            if not match: return []
            return json.JSONDecoder().raw_decode(raw[match.start():].lstrip())[0]
        for _ in range(60):
            if clients(): break
            time.sleep(1)
        else: raise RuntimeError('El escritorio no abrio ventanas')
        launch = 'hl.exec_cmd("env HOME=/juegos/usuarios/ultra XDG_CONFIG_HOME=/juegos/usuarios/ultra/.config XDG_CACHE_HOME=/juegos/usuarios/ultra/.cache ultra-discord")'
        import shlex
        command(ipc+'eval '+shlex.quote(launch), 'ok')
        main_window = None
        for _ in range(180):
            for app in clients():
                size = app.get('size', [0,0])
                if ('discord' in str(app.get('class','')).lower() and
                    str(app.get('title','')).rstrip().endswith('Discord') and size[0]>=200 and size[1]>=200):
                    main_window = app; break
            if main_window: break
            time.sleep(1)
        if not main_window: raise RuntimeError('Discord no paso de su ventana de actualizacion')
        pid = main_window['pid']
        command(f"grep -Eq '^Uid:[[:space:]]+1000[[:space:]]+1000' /proc/{pid}/status && grep -q '^NoNewPrivs:[[:space:]]*1' /proc/{pid}/status && printf 'DISCORD-SEGURO-%s\\n' $((20+4))", 'DISCORD-SEGURO-24')
        with socket.create_connection(('127.0.0.1', port+1), timeout=5) as monitor:
            # Keep the login screenshot ignored: its QR is not a public artifact.
            import tempfile
            shot = Path(tempfile.gettempdir())/'ultra-discord-principal.png'
            shot.unlink(missing_ok=True)
            monitor.sendall(f'screendump {shot} -f png\n'.encode()); time.sleep(2)
            shutil.copyfile(shot, boot.BUILD/'discord-principal.png')
            monitor.sendall(b'sendkey meta_l-shift-m\n'); time.sleep(3)
        conn.sendall(b'poweroff\n'); transcript += boot.read_until(conn, 'APAGADO', 30)
        proc.wait(timeout=30)
        if failures: raise RuntimeError('Fallo de comprobaciones: '+repr(failures))
        record = {'main_window': {'class':main_window['class'], 'title':main_window['title'], 'size':main_window['size']},
                  'uid':1000, 'no_new_privs':True, 'account_login_performed':False,
                  'qemu_exit':proc.returncode, 'main_window_mapped':True}
        (boot.BUILD/'discord-principal.json').write_text(json.dumps(record, indent=2)+'\n')
        print('VERDE: Discord principal mapeado, sin cuenta, UID1000 y NNP1')
    finally:
        if proc.poll() is None: proc.kill(); proc.wait()
        (boot.BUILD/'discord-principal.log').write_text(transcript, encoding='utf-8')


if __name__ == '__main__': main()
