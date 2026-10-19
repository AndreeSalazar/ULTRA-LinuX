#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
set -euo pipefail
lab=$(cd -- "$(dirname -- "$0")" && pwd)
source "$lab/guardianes-lab.sh"
out="${ULTRA_BUFFERS_OUT:-$HOME/linux-reconstruido/build/buffers}"
mkdir -p "$out"
exec > >(tee "$out/informe.txt") 2>&1
gcc --version | head -1
src="$ULTRA_GUARD_ROOT/laboratorio/probar-buffers.c"
ultra_revisar "$src"
if ultra_revisar "$ULTRA_GUARD_ROOT/laboratorio/buffer-ignorado.c" > "$out/ignorado.txt" 2>&1; then
    echo 'ROJO: GCC permitio ignorar el estado del buffer'; exit 1
fi
grep -q 'warn_unused_result' "$out/ignorado.txt"
echo 'REVISION-OK: ignorar el resultado del buffer rechazado con causa'
for mode in prod diag; do
    if [[ $mode = prod ]]; then flags=("${ULTRA_PROD[@]}" -static-pie); else flags=("${ULTRA_DIAG[@]}"); fi
    gcc "${ULTRA_WARN[@]}" "${flags[@]}" "$src" "$ULTRA_GUARD_ROOT/ultra_guard.c" -o "$out/buffers-$mode"
    bash "$lab/aislar-mina.sh" "$out/buffers-$mode" > "$out/$mode.txt" 2>&1
    grep -q '^BUFFERS-OK: matriz=16807 ' "$out/$mode.txt"
    printf '%s: ' "$mode"; cat "$out/$mode.txt"
done
echo 'ALCANCE: contrato aritmetico probado; sin compositor, GPU, fd ni proteccion contra truncamiento concurrente'
