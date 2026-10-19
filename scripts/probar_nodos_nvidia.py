#!/usr/bin/env python3
"""Regression: graphics node creation must not accidentally select the UVM branch."""
from pathlib import Path
import subprocess
import sys
import tempfile

source = Path(sys.argv[1]).read_text()
lines = source.splitlines()
prepare = next(line.strip() for line in lines if line.strip().startswith('/usr/bin/nvidia-modprobe'))
check = next(line.strip() for line in lines if line.strip().startswith('[[ -c /dev/nvidia0'))
with tempfile.TemporaryDirectory(prefix='ultra-nvidia-') as tmp:
    root = Path(tmp)
    stub = root / 'nvidia-modprobe'
    stub.write_text('''#!/bin/bash
[[ "$*" == '-c 0 -m' ]] || exit 9
[[ $TEST_CASE != fail ]] || exit 8
for node in nvidia0 nvidiactl nvidia-modeset; do
    [[ $TEST_CASE != missing || $node != nvidiactl ]] || continue
    ln -s /dev/null "$TEST_ROOT/$node"
done
''')
    stub.chmod(0o755)
    script = prepare.replace('/usr/bin/nvidia-modprobe', str(stub)) + '\n' + check
    for node in ('nvidia0', 'nvidiactl', 'nvidia-modeset'):
        script = script.replace('/dev/' + node, str(root / node))
    for case, expected in [('valid', 0), ('missing', 1), ('fail', 1)]:
        for node in root.glob('nvidia*'):
            if node != stub:
                node.unlink()
        result = subprocess.run(['bash', '-c', script], env={'PATH': '/usr/bin',
                                'TEST_ROOT': tmp, 'TEST_CASE': case}, capture_output=True)
        assert result.returncode == expected, (case, result.stdout, result.stderr)
        print(f'VERDE [nodos NVIDIA: {case}, salida {expected}]')
