#!/usr/bin/env python3
"""Local HTTPS trust failures, separate from Ethernet/TCP fault injection.
Run in a fresh network namespace: sudo unshare -n python3 probar_red_tls.py
Never disables certificate verification and never connects to an external host.
"""
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import ssl
import subprocess
import sys
import tempfile
import threading


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        body = b'ULTRA-TLS-LOCAL\n'
        self.send_response(200)
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_):
        pass


class TLSServer(ThreadingHTTPServer):
    def handle_error(self, request, client_address):
        error = sys.exc_info()[1]
        expected = isinstance(error, (ssl.SSLError, BrokenPipeError, ConnectionResetError))
        self.alerts.append({'tipo': type(error).__name__, 'esperada': expected, 'detalle': str(error)})
        if not expected:
            super().handle_error(request, client_address)


def command(*args):
    return subprocess.run(args, capture_output=True, text=True, timeout=10)


def main():
    links = json.loads(command('ip', '-j', 'link').stdout)
    if {link['ifname'] for link in links} != {'lo'}:
        raise SystemExit('ROJO: se requiere namespace nuevo con solo loopback')
    command('ip', 'link', 'set', 'lo', 'up').check_returncode()
    cases = []
    with tempfile.TemporaryDirectory(prefix='ultra-tls-') as tmp:
        os.chmod(tmp, 0o755)  # Certificate is public; the private key remains 0600.
        root = Path(tmp)
        cert, key = root / 'cert.pem', root / 'key.pem'
        command('openssl', 'req', '-x509', '-newkey', 'rsa:2048', '-nodes',
                '-subj', '/CN=localhost', '-addext', 'subjectAltName=DNS:localhost',
                '-days', '1', '-keyout', str(key), '-out', str(cert)).check_returncode()
        key.chmod(0o600); cert.chmod(0o644)
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.minimum_version = ssl.TLSVersion.TLSv1_2
        context.load_cert_chain(cert, key)
        server = TLSServer(('127.0.0.1', 0), Handler)
        server.alerts = []
        server.socket = context.wrap_socket(server.socket, server_side=True)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        port = server.server_port
        try:
            prefix = ('setpriv', '--reuid=1000', '--regid=1000', '--clear-groups',
                      '--no-new-privs', '--bounding-set=-all',
                      'env', '-i', 'PATH=/usr/bin', 'LANG=C', 'curl', '--silent', '--show-error',
                      '--noproxy', '*', '--connect-timeout', '2', '--max-time', '3', '--fail')
            tests = [
                ('HTTPS: certificado desconocido se rechaza', (), 'localhost', 60),
                ('HTTPS: confianza explicita y nombre correcto', ('--cacert', str(cert)), 'localhost', 0),
                ('HTTPS: nombre incorrecto se rechaza aun con CA confiada', ('--cacert', str(cert)), '127.0.0.1', 60),
            ]
            for name, extra, host, expected in tests:
                result = command(*prefix, *extra, f'https://{host}:{port}/')
                assert result.returncode == expected, (name, result.returncode, result.stderr)
                assert (result.stdout == 'ULTRA-TLS-LOCAL\n') if expected == 0 else not result.stdout
                cases.append({'caso': name, 'salida_curl': result.returncode,
                              'respuesta': result.stdout, 'diagnostico': result.stderr.strip()})
        finally:
            server.shutdown(); server.server_close(); thread.join(timeout=2)
        assert all(alert['esperada'] for alert in server.alerts), server.alerts
    print(json.dumps({'entorno': 'VM taller; HTTPS en loopback aislado; curl UID1000 sin capacidades',
                      'casos': cases, 'alertas_servidor': server.alerts}, indent=2, ensure_ascii=False))


if __name__ == '__main__':
    main()
