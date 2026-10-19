#!/usr/bin/env python3
"""Private veth/netem fault lab. Invoke ONLY with: sudo unshare -n -m --propagation private python3 ...
No traffic or qdisc modifications on the workshop or physical Ethernet link.
TCP checks application data; UDP counts received unique sequences and duplicates.
This is Linux 7.2 workshop software networking, not an r8169/ULTRA wire benchmark.
"""
import hashlib
import json
import os
from pathlib import Path
import select
import socket
import struct
import subprocess
import sys
import time


def server():
    print(os.getpid(), flush=True)
    assert sys.stdin.readline().strip() == 'start'
    udp = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    udp.bind(('192.0.2.2', 32001))
    tcp = socket.socket()
    tcp.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    tcp.bind(('192.0.2.2', 32002))
    tcp.listen(1)
    print('ready', flush=True)
    while True:
        ready, _, _ = select.select([tcp, udp], [], [], 2)
        for sock in ready:
            if sock is udp:
                data, peer = udp.recvfrom(2048)
                udp.sendto(data, peer)
            else:
                client, _ = tcp.accept()
                with client:
                    client.settimeout(2)
                    digest = hashlib.sha256()
                    size = 0
                    while data := client.recv(65536):
                        digest.update(data)
                        size += len(data)
                        if size > 2 * 1024 * 1024:
                            break
                    client.sendall(digest.hexdigest().encode())


def command(*args):
    return subprocess.run(args, check=True, capture_output=True, text=True, timeout=10).stdout


def main():
    own, init = os.stat('/proc/self/ns/mnt'), os.stat('/proc/1/ns/mnt')
    if (own.st_dev, own.st_ino) == (init.st_dev, init.st_ino):
        raise SystemExit('ROJO: se requiere tambien namespace de montajes privado (-m)')
    links = json.loads(command('ip', '-j', 'link'))
    if {link['ifname'] for link in links} != {'lo'}:
        raise SystemExit('ROJO: ejecutar en namespace nuevo con solo lo; no se toca la red existente')
    # A sysfs mount is tied to the network namespace at mount time. Without a
    # private remount, /sys would still describe the workshop NIC, not this veth.
    command('mount', '--make-rprivate', '/')
    command('mount', '-t', 'sysfs', '-o', 'nosuid,nodev,noexec', 'sysfs', '/sys')
    observer = str(Path(sys.argv[1]).resolve())
    reports = {'entorno': 'VM taller, veth/netem privados, sin NIC fisica ni TLS', 'casos': []}
    # Separate peer network namespace; no named namespaces persist on disk.
    peer = subprocess.Popen(['unshare', '-n', sys.executable, __file__, '--server'],
                            stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
    try:
        readable, _, _ = select.select([peer.stdout], [], [], 5)
        assert readable, 'peer no inicio'
        pid = int(peer.stdout.readline())
        command('ip', 'link', 'add', 'red0', 'type', 'veth', 'peer', 'name', 'red1')
        command('ip', 'link', 'set', 'red1', 'netns', str(pid))
        command('ip', 'address', 'add', '192.0.2.1/24', 'dev', 'red0')
        command('ip', 'link', 'set', 'red0', 'up')
        prefix = ('nsenter', '-t', str(pid), '-n')
        command(*prefix, 'ip', 'address', 'add', '192.0.2.2/24', 'dev', 'red1')
        command(*prefix, 'ip', 'link', 'set', 'red1', 'up')
        peer.stdin.write('start\n'); peer.stdin.flush()
        assert select.select([peer.stdout], [], [], 5)[0]
        assert peer.stdout.readline().strip() == 'ready'
        snapshots = []

        def observe():
            # Measurement binary is always unprivileged, even in the fault lab.
            snapshots.append(command('setpriv', '--reuid=1000', '--regid=1000', '--clear-groups',
                                     '--no-new-privs', '--bounding-set=-all', observer))

        def tcp_case(name):
            payload = bytes(range(256)) * 4096
            before = time.monotonic()
            with socket.create_connection(('192.0.2.2', 32002), timeout=4) as client:
                for offset in range(0, len(payload), 7919):
                    client.sendall(payload[offset:offset+7919])
                client.shutdown(socket.SHUT_WR)
                digest = b''
                while piece := client.recv(7):
                    digest += piece
                    assert len(digest) <= 64
            assert digest.decode() == hashlib.sha256(payload).hexdigest()
            elapsed = time.monotonic() - before
            reports['casos'].append({'caso': name, 'bytes_utiles': len(payload), 'segundos': elapsed,
                                      'Mbit_s_aplicacion': len(payload) * 8 / elapsed / 1e6,
                                      'sha256_correcto': True})

        def udp_case(name, expect_loss=False):
            received = []
            with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as client:
                client.settimeout(.5)
                for seq in range(100):
                    client.sendto(struct.pack('!I', seq) + b'U' * 508, ('192.0.2.2', 32001))
                    time.sleep(.001)
                while True:
                    try:
                        data, _ = client.recvfrom(2048)
                    except TimeoutError:
                        break
                    assert len(data) == 512 and data[4:] == b'U' * 508
                    seq, = struct.unpack('!I', data[:4])
                    assert 0 <= seq < 100
                    received.append(seq)
            unique = len(set(received))
            assert unique == (0 if expect_loss else 100), (name, unique)
            reports['casos'].append({'caso': name, 'enviados': 100, 'unicos_recibidos': unique,
                                      'duplicados': len(received) - unique})

        observe()
        tcp_case('TCP base: envios y lecturas con tamanos diferentes')
        udp_case('UDP base')
        observe()
        command('tc', 'qdisc', 'add', 'dev', 'red0', 'root', 'netem', 'loss', '100%')
        udp_case('UDP perdida total: no se inventan entregas', expect_loss=True)
        before = time.monotonic()
        try:
            with socket.create_connection(('192.0.2.2', 32002), timeout=.4):
                raise AssertionError('conexion inesperada con perdida total')
        except TimeoutError:
            reports['casos'].append({'caso': 'TCP perdida total: timeout acotado',
                                      'segundos': time.monotonic()-before})
        reports['qdisc_perdida'] = command('tc', '-s', 'qdisc', 'show', 'dev', 'red0')
        command('tc', 'qdisc', 'replace', 'dev', 'red0', 'root', 'netem', 'delay', '3ms', 'duplicate', '10%')
        tcp_case('TCP retardo y duplicados: integridad conservada')
        udp_case('UDP retardo y duplicados: secuencias contadas')
        reports['qdisc_retardo'] = command('tc', '-s', 'qdisc', 'show', 'dev', 'red0')
        command('tc', 'qdisc', 'del', 'dev', 'red0', 'root')
        tcp_case('TCP recuperacion tras quitar los fallos')
        observe()
        reports['muestras_C'] = ''.join(snapshots)
        from resumir_red import summarize
        reports['tasas'] = summarize(reports['muestras_C'])
        assert reports['tasas']['intervalos']
        assert all(item['delta']['rx_bytes'] > 0 and item['delta']['tx_bytes'] > 0
                   for item in reports['tasas']['intervalos'])
        print(json.dumps(reports, indent=2, ensure_ascii=False))
    finally:
        peer.terminate()
        try:
            peer.wait(timeout=3)
        except subprocess.TimeoutExpired:
            peer.kill(); peer.wait(timeout=2)


if __name__ == '__main__':
    if sys.argv[1:] == ['--server']:
        server()
    else:
        main()
