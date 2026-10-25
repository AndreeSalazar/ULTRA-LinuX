#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-only
# Banco raiz vacia, solo dentro del taller QEMU. El programa corre como UID 1000.
set -euo pipefail
[[ $(id -u) == 0 && $(cat /sys/class/dmi/id/sys_vendor) == QEMU ]] || {
    echo 'RED [AISLADO]: requiere root del taller QEMU; no ejecutar en el PC real' >&2; exit 1;
}
here=$(cd -- "$(dirname -- "$0")" && pwd)
base=$(cd -- "$here/../.." && pwd)
parent="$base/build/CASM/native"
[[ -f $parent/self-host.json && -f $parent/casmc-native.elf && ! -L $parent ]]
root=$(mktemp -d "$parent/aislado-XXXXXX")
chmod 0755 "$root"
install -o 0 -g 0 -m 0555 "$parent/casmc-native.elf" "$root/casmc"
install -o 0 -g 0 -m 0444 "$here/compiler.casm" "$root/compiler.casm"
install -d -o 1000 -g 1000 -m 0700 "$root/out"
printf 'u64 casm_main() { print(2026); return 0; }\n' > "$root/program.casm"
chmod 0444 "$root/program.casm"
timeout 20 chroot --userspec=1000:1000 --groups=1000 "$root" /casmc /compiler.casm /out/one.elf /out/one.hex
timeout 20 chroot --userspec=1000:1000 --groups=1000 "$root" /out/one.elf /compiler.casm /out/two.elf /out/two.hex
cmp "$root/casmc" "$root/out/one.elf"
cmp "$root/casmc" "$root/out/two.elf"
cmp "$root/out/one.hex" "$root/out/two.hex"
timeout 20 chroot --userspec=1000:1000 --groups=1000 "$root" /out/two.elf /program.casm /out/program.elf /out/program.hex
[[ $(timeout 5 chroot --userspec=1000:1000 --groups=1000 "$root" /out/program.elf) == 2026 ]]
python3 - "$here" "$parent" "$root" <<'PY'
from pathlib import Path
import hashlib,json,sys
sys.path.insert(0,sys.argv[1])
from verify import audit
p=Path(sys.argv[2]);root=Path(sys.argv[3])
for f in root.glob('out/*.elf'):
    audit(f.read_bytes())
    assert bytes.fromhex(f.with_suffix('.hex').read_text())==f.read_bytes()
report=json.loads((p/'self-host.json').read_text())
report['empty_root_execution']={'passed':True,'uid':1000,'libraries_present':False,'external_tools_present':False,
    'only_inputs':['casmc','compiler.casm','program.casm'],'new_generations_equal':True,'different_program_output':'2026',
    'source_sha256':hashlib.sha256((root/'compiler.casm').read_bytes()).hexdigest()}
(p/'self-host.json').write_text(json.dumps(report,indent=2)+'\n')
PY
echo 'GREEN [AISLADO]: UID 1000, sin GCC/Clang/Python/libc; dos generaciones iguales y programa distinto ejecutado'
