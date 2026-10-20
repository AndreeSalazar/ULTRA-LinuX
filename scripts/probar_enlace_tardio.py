#!/usr/bin/env python3
"""Real dhcpcd/QEMU regression. Uses an image copy and a private virtual NIC.
Usage: probar_enlace_tardio.py generico|pc --antes|--despues
--antes proves the old bug: link appears after the helper has exited, no DHCP.
--despues proves recovery AFTER the new helper's foreground deadline, then HTTPS.
"""
import importlib.util
import base64
import hashlib
import json
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import time


def main():
    profile, version = sys.argv[1:3]
    source = Path(sys.argv[3]).resolve() if len(sys.argv)>3 else None
    assert profile in ('generico', 'pc') and version in ('--antes', '--despues')
    sys.argv = [sys.argv[0], profile, '--bmo', '--brave']
    spec = importlib.util.spec_from_file_location('arranque', Path(__file__).with_name('probar_arranque.py'))
    base = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(base)
    base.check_kernel()
    image = base.BUILD / 'enlace-tardio.img'
    variables = base.BUILD / 'enlace-tardio-vars.fd'
    shutil.copyfile(base.IMG, image)
    shutil.copyfile(base.QEMU_SHARE / 'edk2-i386-vars.fd', variables)
    process = subprocess.Popen([str(base.QEMU), '-machine', 'pc', '-accel', 'whpx', '-cpu', 'host',
        '-smp', '1', '-m', '1024', '-S', '-display', 'none', '-no-reboot',
        '-drive', f'if=pflash,format=raw,readonly=on,file={base.QEMU_SHARE / "edk2-x86_64-code.fd"}',
        '-drive', f'if=pflash,format=raw,file={variables}',
        '-drive', f'file={image},format=raw,if=virtio',
        '-netdev', 'user,id=ultranet', '-device', 'virtio-net-pci,netdev=ultranet,id=ultranic',
        '-serial', 'tcp:127.0.0.1:15655,server=on,wait=off',
        '-monitor', 'tcp:127.0.0.1:15656,server=on,wait=off'],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    transcript = ''
    counter = 0
    result = {'perfil': profile, 'version': version, 'escenarios': []}
    try:
        for _ in range(50):
            try:
                console = socket.create_connection(('127.0.0.1', 15655), timeout=1)
                monitor = socket.create_connection(('127.0.0.1', 15656), timeout=1)
                break
            except OSError:
                time.sleep(.2)
        else:
            raise RuntimeError('QEMU no abrio los puertos de prueba')
        console.settimeout(.5); monitor.settimeout(.5)
        def hmp(command):
            monitor.sendall((command+'\n').encode())
            response = base.read_until(monitor, '(qemu)', 5)
            assert 'not found' not in response and 'Error' not in response, response
        base.read_until(monitor, '(qemu)', 5)
        hmp('set_link ultranic off')
        hmp('cont')
        transcript += base.read_until(console, 'ultra:', 120)
        def command(text, timeout=25):
            nonlocal transcript, counter
            counter += 1
            marker = f'ENLACE-FIN-{100+counter}'
            console.sendall((text+f'; echo ENLACE-FIN-$((100+{counter}))\n').encode())
            output = base.read_until(console, marker, timeout)
            if 'ultra:' not in output.split(marker, 1)[-1]:
                output += base.read_until(console, 'ultra:', 5)
            transcript += output
            return output
        if source is not None:
            # Preflight only: inject into the disposable image's RAM overlay.
            # Final validation omits this option and tests the built image itself.
            encoded = base64.b64encode(source.read_bytes()).decode()
            command(': > /arch/tmp/red-candidato.b64')
            for offset in range(0, len(encoded), 256):
                command(f"printf '%s' '{encoded[offset:offset+256]}' >> /arch/tmp/red-candidato.b64")
            output = command('busybox base64 -d /arch/tmp/red-candidato.b64 > /arch/usr/local/libexec/ultra-preparar-red; sha256sum /arch/usr/local/libexec/ultra-preparar-red')
            digest = hashlib.sha256(source.read_bytes()).hexdigest()
            assert digest in output, output
            result['fuente_inyectada_preflight_sha256'] = digest
        started = time.monotonic()
        output = command('ultra-entrar /usr/bin/bash /usr/local/libexec/ultra-preparar-red; echo RED-RC-$?', 22)
        elapsed = time.monotonic()-started
        assert 'RED-RC-1' in output, output
        assert elapsed < 18, elapsed
        result['espera_sin_cable_s'] = elapsed
        result['escenarios'].append('espera de primer plano acotada, sin cable ni IPv4')
        # The helper has completely exited. Only a persistent DHCP client can recover.
        hmp('set_link ultranic on')
        ready = False
        deadline = time.monotonic()+18
        while time.monotonic()<deadline:
            output = command('ultra-entrar /usr/bin/ip -4 route show default')
            if '\ndefault ' in output.replace('\r', ''):
                ready = True; break
            if version == '--antes' and time.monotonic()>deadline-12:
                break
            time.sleep(.5)
        assert ready == (version == '--despues'), (ready, transcript)
        if version == '--antes':
            result['escenarios'].append('defecto reproducido: carrier nuevo no inicia DHCP tras salir')
        else:
            result['escenarios'].append('DHCP recupera IPv4 al conectar DESPUES de salir del preparador')
            output = command('ultra-entrar /usr/bin/grep nameserver /etc/resolv.conf')
            assert '\nnameserver ' in output.replace('\r', ''), output
            result['escenarios'].append('DNS recibido mediante DHCP')
            https = ('ultra-entrar /usr/bin/setpriv --reuid=1000 --regid=1000 --clear-groups '
                     '--no-new-privs --bounding-set=-all /usr/bin/curl --silent --show-error '
                     '--noproxy "*" --fail --connect-timeout 4 --max-time 10 https://example.com/; echo HTTPS-RC-$?')
            output = command(https, 18)
            assert 'Example Domain' in output and 'HTTPS-RC-0' in output, output
            result['escenarios'].append('HTTPS real con CA valida, como UID1000 sin capacidades')
            hmp('set_link ultranic off'); time.sleep(.5); hmp('set_link ultranic on')
            # DHCP may temporarily withdraw the old DNS/route while rebinding.
            # Wait for actual recovery, rather than assuming a fixed 3 s lease time.
            deadline = time.monotonic()+22
            while time.monotonic()<deadline:
                output = command(https, 18)
                if 'Example Domain' in output and 'HTTPS-RC-0' in output:
                    break
                time.sleep(.5)
            else:
                raise AssertionError(output)
            result['escenarios'].append('reconexion tras desconectar y reconectar cable virtual')
            output = command('ultra-entrar /usr/bin/bash /usr/local/libexec/ultra-preparar-red; echo RED-RC-$?', 18)
            assert 'RED-RC-0' in output, output
            result['escenarios'].append('reentrada con red activa devuelve configuracion sin espera de lease')
        console.sendall(b'poweroff\n')
        transcript += base.read_until(console, 'APAGADO', 20)
        process.wait(timeout=20)
        print(json.dumps(result, ensure_ascii=False, indent=2))
    finally:
        if process.poll() is None:
            process.kill(); process.wait(timeout=10)
        (base.BUILD / ('enlace-tardio-'+version[2:]+'.log')).write_text(transcript, encoding='utf-8')


if __name__ == '__main__':
    main()
