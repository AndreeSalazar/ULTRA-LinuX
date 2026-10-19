#!/usr/bin/env python3
"""Model the vendor's per-minor semantics; never invent nvidiactl."""
from pathlib import Path
import subprocess
import sys
import tempfile

source = Path(sys.argv[1]).read_text()
with tempfile.TemporaryDirectory(prefix='ultra-nvidia-') as tmp:
    root = Path(tmp)
    stub = root / 'nvidia-modprobe'
    stub.write_text('''#!/bin/bash
[[ $TEST_CASE != fail ]] || exit 8
minors=(); modeset=0; uvm=0
while (( $# )); do
    case "$1" in
        -c) minors+=("$2"); shift ;;
        -c[0-9]*) minors+=("${1#-c}") ;;
        -m) modeset=1 ;;
        -u) uvm=1 ;;
        *) exit 9 ;;
    esac
    shift
done
nodes=()
for minor in "${minors[@]}"; do
    if (( uvm )); then nodes+=(nvidia-uvm); continue; fi
    case "$minor" in
        0) nodes+=(nvidia0) ;;
        255) nodes+=(nvidiactl) ;;
        *) exit 9 ;;
    esac
done
(( ! modeset )) || nodes+=(nvidia-modeset)
for node in "${nodes[@]}"; do
    [[ $TEST_CASE != missing || $node != nvidiactl ]] || continue
    ln -sf /dev/null "$TEST_ROOT/$node"
done
''')
    stub.chmod(0o755)
    script = source.replace('/usr/bin/nvidia-modprobe', str(stub))
    # Fixture runs unprivileged; fake character devices use /dev/null, never a GPU.
    script = script.replace('[[ $(id -u) == 0 ]]', '[[ 0 == 0 ]]')
    for node in ('nvidia0', 'nvidiactl', 'nvidia-modeset'):
        script = script.replace('/dev/' + node, str(root / node))
    for case, expected in [('valid', 0), ('missing', 1), ('fail', 1), ('legacy-no-control', 1), ('legacy-uvm', 1)]:
        for node in root.glob('nvidia*'):
            if node != stub:
                node.unlink()
        candidate = script
        if case == 'legacy-no-control': candidate = script.replace('-c 0 -c 255 -m', '-c 0 -m')
        if case == 'legacy-uvm': candidate = script.replace('-c 0 -c 255 -m', '-c0 -u')
        result = subprocess.run(['bash', '-c', candidate], env={'PATH': '/usr/bin',
                                'TEST_ROOT': tmp, 'TEST_CASE': case}, capture_output=True)
        assert result.returncode == expected, (case, result.stdout, result.stderr)
        if case in ('missing', 'legacy-no-control', 'legacy-uvm'):
            assert b'nvidiactl' in result.stdout, result.stdout
        print(f'VERDE [nodos NVIDIA: {case}, salida {expected}]')
