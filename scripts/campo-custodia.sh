#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
set -euo pipefail
lab=$(cd -- "$(dirname -- "$0")" && pwd)
source "$lab/guardianes-lab.sh"
out=${ULTRA_CUSTODIA_OUT:?salida nueva obligatoria}
[[ ! -e $out ]] || exit 1
mkdir -p "$out"
exec > >(tee "$out/informe.txt") 2>&1
gcc --version | head -1
uname -srmo
python "$lab/inspeccionar-asm-hex.py" --selftest
unit=$ULTRA_GUARD_ROOT/laboratorio/custodia
owner=$ULTRA_GUARD_ROOT/produccion/ultra_owner.c
for src in "$unit/ultra_custodia.c" "$unit/probar-custodia.c" "$owner"; do ultra_revisar "$src"; done
for mode in prod-O2 prod-O3 diag-O1 diag-O2; do
    case $mode in
        prod-*) flags=("${ULTRA_PROD[@]}" "-${mode#prod-}" -static-pie);;
        diag-*) flags=("${ULTRA_DIAG[@]}" "-${mode#diag-}");;
    esac
    gcc "${ULTRA_WARN[@]}" "${flags[@]}" "$unit/probar-custodia.c" "$unit/ultra_custodia.c" "$owner" -Wl,--wrap=calloc -o "$out/custodia-$mode"
    bash "$lab/aislar-mina.sh" "$out/custodia-$mode" > "$out/$mode.txt" 2>&1
    grep -q '^CUSTODIA-OK:' "$out/$mode.txt"
    printf 'PERFIL %s\n' "$mode"; cat "$out/$mode.txt"
    python "$lab/inspeccionar-custodia-hex.py" "$out/custodia-$mode" "$out/hex-$mode"
done
printf 'not an ELF\n' > "$out/no-elf"
set +e
python "$lab/inspeccionar-custodia-hex.py" "$out/no-elf" "$out/rechazado-elf" > "$out/rechazo-elf.txt" 2>&1
status=$?
set -e
[[ $status -ne 0 && ! -e $out/rechazado-elf ]]
grep -q 'requires ELF64' "$out/rechazo-elf.txt"
set +e
python "$lab/inspeccionar-custodia-hex.py" /usr/bin/true "$out/rechazado-simbolo" > "$out/rechazo-simbolo.txt" 2>&1
status=$?
set -e
[[ $status -ne 0 && ! -e $out/rechazado-simbolo ]]
grep -q 'SYMBOL-MISSING' "$out/rechazo-simbolo.txt"
echo 'CUSTODIA-HEX-NEGATIVOS-OK: formato y simbolo rechazados sin publicar salida'
sha256sum "$unit"/*.c "$unit"/*.h "$owner" "$ULTRA_GUARD_ROOT/produccion/ultra_owner.h" \
    "$ULTRA_GUARD_ROOT/produccion/perfil-gcc.sh" "$ULTRA_GUARD_ROOT/laboratorio/perfil-gcc.sh" \
    "$lab/campo-custodia.sh" "$lab/aislar-mina.sh" "$lab/guardianes.sh" "$lab/guardianes-lab.sh" \
    "$lab/inspeccionar-custodia-hex.py" "$lab/inspeccionar-asm-hex.py" "$out"/custodia-* > "$out/artefactos.sha256"
echo 'ALCANCE: modelo C serializado y copia de lectura; identidad suministrada confiable; sin IPC, kernel, prioridad ni medicion de velocidad'
