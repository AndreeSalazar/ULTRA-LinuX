#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
set -euo pipefail
lab=$(cd -- "$(dirname -- "$0")" && pwd)
source "$lab/guardianes-lab.sh"
out=${ULTRA_OWNER_ASM_OUT:?salida nueva obligatoria}
[[ ! -e $out ]] || exit 1
mkdir -p "$out"
exec > >(tee "$out/informe.txt") 2>&1
gcc --version | head -1
uname -srmo
src=$ULTRA_GUARD_ROOT/laboratorio/probar-owner.c
owner=$ULTRA_GUARD_ROOT/produccion/ultra_owner.c
asm=$ULTRA_GUARD_ROOT/../ASM/laboratorio
ultra_revisar "$src"
ultra_revisar "$owner"
ultra_revisar "$asm/probar-abi.c"
for mode in prod-O2 prod-O3 diag-O1 diag-O2; do
    case $mode in
        prod-*) flags=("${ULTRA_PROD[@]}" "-${mode#prod-}" -static-pie);;
        diag-*) flags=("${ULTRA_DIAG[@]}" "-${mode#diag-}");;
    esac
    printf 'PERFIL %s:' "$mode"; printf ' %q' "${ULTRA_WARN[@]}" "${flags[@]}"; printf '\n'
    gcc "${ULTRA_WARN[@]}" "${flags[@]}" "$src" "$owner" -Wl,--wrap=calloc -o "$out/owner-$mode"
    bash "$lab/aislar-mina.sh" "$out/owner-$mode" > "$out/owner-$mode.txt" 2>&1
    grep -q '^OWNER-OK:' "$out/owner-$mode.txt"
    cat "$out/owner-$mode.txt"
    gcc "${ULTRA_WARN[@]}" "${flags[@]}" "$asm/probar-abi.c" "$asm/abi.S" -o "$out/asm-$mode"
    bash "$lab/aislar-mina.sh" "$out/asm-$mode" > "$out/asm-$mode.txt" 2>&1
    grep -q '^ASM-OK:' "$out/asm-$mode.txt"
    cat "$out/asm-$mode.txt"
done
sha256sum "$src" "$owner" "$ULTRA_GUARD_ROOT/produccion/ultra_owner.h" "$asm/probar-abi.c" "$asm/abi.S" "$out"/owner-* "$out"/asm-* > "$out/SHA256SUMS"
echo 'ALCANCE: handles de un pool vivo sin concurrencia; ABI SysV x86-64 controlada; no proteccion global ni instrumentacion de interiores ASM'
