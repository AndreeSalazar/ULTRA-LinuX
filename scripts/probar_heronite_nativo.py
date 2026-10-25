#!/usr/bin/env python3
"""Offline native migration in a disposable image stage; never a user's home."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

stage = Path(sys.argv[1]).resolve()
assert (stage/'ULTRA-SISTEMA').is_file()
assert not (stage/'opt/heroic/heroic').exists()
assert (stage/'etc/ultra/heronite-version').read_text().strip() == '0.15.0'
root = Path(tempfile.mkdtemp(prefix='heronite-migracion-', dir=stage/'tmp'))
home = '/' + str(root.relative_to(stage))
heroic = root/'.config/heroic'
data = root/'.local/share/heronite'


def fingerprint(directory):
    return {str(p.relative_to(directory)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in directory.rglob('*') if p.is_file()}


try:
    (heroic/'gog_store').mkdir(parents=True)
    (heroic/'GamesConfig').mkdir()
    (heroic/'tools/fake-proton').mkdir(parents=True)
    (heroic/'tools/fake-proton/proton').write_bytes(b'not-a-real-runner')
    old = home+'/.config/heroic'
    config = {'defaultSettings': {'wineVersion': {'bin': old+'/tools/fake-proton/proton'}}}
    (heroic/'config.json').write_text(json.dumps(config))
    (heroic/'gog_store/installed.json').write_text(json.dumps({'installed': [
        {'appName': '1423049311', 'install_path': '/juegos/Cyberpunk2077', 'buildId': '12345'}]}))
    (heroic/'GamesConfig/1423049311.json').write_text(json.dumps({'1423049311': {
        'winePrefix': '/juegos/usuarios/ultra/Juegos/prefijos/cyberpunk2077'}}))
    before = fingerprint(heroic)
    command = ['chroot', str(stage), '/usr/bin/env', '-i', 'HOME='+home,
               'PATH=/usr/bin', 'LANG=C.UTF-8', '/usr/bin/heronite-rs', '--help']
    result = subprocess.run(command, text=True, capture_output=True, timeout=30)
    assert result.returncode == 0, result.stderr
    assert '--launch' in result.stdout
    assert (data/'migrated-from-heroic.json').is_file()
    assert fingerprint(heroic) == before, 'original metadata changed'
    new = json.loads((data/'config.json').read_text())
    assert new['defaultSettings']['wineVersion']['bin'] == home+'/.local/share/heronite/tools/fake-proton/proton'
    assert (data/'gog_store/installed.json').read_bytes() == (heroic/'gog_store/installed.json').read_bytes()
    assert (data/'GamesConfig/1423049311.json').read_bytes() == (heroic/'GamesConfig/1423049311.json').read_bytes()
    assert not (data/'gog_store/auth.json').exists()
    marker = (data/'migrated-from-heroic.json').read_bytes()
    subprocess.run(command, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=30)
    assert marker == (data/'migrated-from-heroic.json').read_bytes()
    print('VERDE: Heronite nativo importa, reubica herramientas, conserva prefijo/original y no remigra; sin cuentas ni red')
finally:
    assert root.parent == stage/'tmp' and not root.is_symlink()
    shutil.rmtree(root)
