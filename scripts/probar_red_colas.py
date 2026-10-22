#!/usr/bin/env python3
"""Compare FIFO/FQ-CoDel only inside fresh mount AND network namespaces.

Synthetic 10 Mbit/s egress bottleneck, one bulk TCP flow and small UDP probes.
No physical interface, external endpoint, packet capture or personal data.
"""
import json
import os
from pathlib import Path
import select
import socket
import statistics
import struct
import subprocess
import sys
import threading
import time


def command(*args):
    result = subprocess.run(args, text=True, capture_output=True, timeout=10)
    if result.returncode:
        raise RuntimeError(str(args)+': '+result.stderr.strip())
    return result.stdout


def server():
    print(os.getpid(), flush=True)
    assert sys.stdin.readline().strip() == 'start'
    tcp = socket.socket(); tcp.setsockopt(socket.SOL_SOCKET,socket.SO_REUSEADDR,1)
    tcp.bind(('192.0.2.2',32012)); tcp.listen(2)
    udp = socket.socket(socket.AF_INET,socket.SOCK_DGRAM); udp.bind(('192.0.2.2',32013))
    clients = {}
    print('ready', flush=True)
    while True:
        ready,_,_ = select.select([tcp,udp,*clients],[],[],1)
        for sock in ready:
            if sock is tcp:
                client,_ = tcp.accept(); clients[client] = 0
            elif sock is udp:
                data,peer = udp.recvfrom(128); udp.sendto(data,peer)
            else:
                data = sock.recv(65536)
                if data:
                    clients[sock] += len(data)
                else:
                    print(json.dumps({'tcp_bytes':clients.pop(sock)}),flush=True); sock.close()


def run_case(qdisc, peer):
    # Replacing the same TBF handle keeps its child; remove the preceding tree
    # so changing child kind never tries to mutate an existing FIFO in place.
    existing = json.loads(command('tc','-j','qdisc','show','dev','labtx'))
    if any(item.get('root') and item['kind'] != 'noqueue' for item in existing):
        command('tc','qdisc','del','dev','labtx','root')
    command('tc','qdisc','replace','dev','labtx','root','handle','1:',
            'tbf','rate','10mbit','burst','16384','latency','1000ms')
    if qdisc == 'fifo':
        command('tc','qdisc','replace','dev','labtx','parent','1:1','handle','10:', 'pfifo','limit','1000')
    else:
        command('tc','qdisc','replace','dev','labtx','parent','1:1','handle','10:', 'fq_codel')
    stop = threading.Event()
    failures = []
    def bulk():
        try:
            with socket.create_connection(('192.0.2.2',32012),timeout=2) as sock:
                sock.settimeout(.3)
                while not stop.is_set():
                    try:
                        sock.sendall(b'X'*16384)
                    except socket.timeout:
                        continue
        except OSError as error:
            failures.append(type(error).__name__)
    thread = threading.Thread(target=bulk,daemon=True); thread.start()
    pending, rtts, sent = {}, [], 0
    with socket.socket(socket.AF_INET,socket.SOCK_DGRAM) as udp:
        udp.setblocking(False)
        start = time.monotonic(); next_probe = start
        while time.monotonic()-start < 8:
            now = time.monotonic()
            if now-start < 6 and now >= next_probe:
                pending[sent] = now
                udp.sendto(struct.pack('!I',sent),('192.0.2.2',32013)); sent += 1
                next_probe = now+.02
            if now-start >= 6:
                stop.set()
            ready,_,_ = select.select([udp],[],[],.01)
            if ready:
                data,_ = udp.recvfrom(128)
                if len(data) == 4:
                    seq, = struct.unpack('!I',data)
                    before = pending.pop(seq,None)
                    if before is not None:
                        rtts.append((time.monotonic()-before)*1000)
    stop.set(); thread.join(timeout=2)
    if thread.is_alive() or failures:
        raise RuntimeError('generador no termino correctamente: '+str(failures))
    assert select.select([peer.stdout],[],[],5)[0], 'falta contador de recepcion TCP'
    received = json.loads(peer.stdout.readline())['tcp_bytes']
    ordered = sorted(rtts)
    assert len(ordered) >= 20, 'pocas respuestas UDP para calcular percentiles'
    return {'qdisc':qdisc, 'TCP_bytes_receptor':received, 'UDP_enviados':sent,
            'UDP_recibidos':len(ordered), 'UDP_sin_respuesta_en_ventana':len(pending),
            'RTT_mediana_ms':round(statistics.median(ordered),3),
            'RTT_p95_ms':round(ordered[int(.95*(len(ordered)-1))],3),
            'RTT_max_ms':round(max(ordered),3),
            'tc':command('tc','-s','qdisc','show','dev','labtx')}


def main():
    assert os.geteuid() == 0
    assert os.readlink('/proc/self/ns/mnt') != os.readlink('/proc/1/ns/mnt'), 'namespace de montajes requerido'
    assert {i['ifname'] for i in json.loads(command('ip','-j','link'))} == {'lo'}, 'namespace nuevo solo con lo requerido'
    command('ip','link','set','lo','up')
    peer = subprocess.Popen(['unshare','-n',sys.executable,__file__,'--server'],
                            stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True)
    try:
        pid = int(peer.stdout.readline())
        command('ip','link','add','labtx','type','veth','peer','name','labrx')
        command('ip','link','set','labrx','netns',str(pid))
        command('ip','address','add','192.0.2.1/24','dev','labtx')
        command('ip','link','set','labtx','up')
        prefix = ('nsenter','-t',str(pid),'-n')
        command(*prefix,'ip','address','add','192.0.2.2/24','dev','labrx')
        command(*prefix,'ip','link','set','labrx','up')
        command(*prefix,'ip','link','set','lo','up')
        peer.stdin.write('start\n'); peer.stdin.flush(); assert peer.stdout.readline().strip() == 'ready'
        results = []
        for qdisc in ('fifo','fq_codel','fq_codel','fifo'):
            result = run_case(qdisc,peer); results.append(result)
            print(json.dumps({k:v for k,v in result.items() if k!='tc'}), flush=True)
        report = {'entorno':'VM de taller Linux, veth privado; no es NIC real ni kernel ULTRA',
                  'kernel':os.uname().release, 'cuello_sintetico_Mbit_s':10,
                  'carga':'TCP mas sondas UDP; FIFO de 1000 paquetes frente a FQ-CoDel por defecto',
                  'casos':results,'red_externa_expuesta':False,
                  'limite':'No demuestra mejora en descarga, recepcion WAN, r8169, TLS o juegos reales.'}
        Path(sys.argv[1]).write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
    finally:
        peer.terminate()
        try: peer.wait(timeout=3)
        except subprocess.TimeoutExpired: peer.kill(); peer.wait(timeout=3)


if __name__ == '__main__':
    if '--server' in sys.argv:
        server()
    else:
        main()
