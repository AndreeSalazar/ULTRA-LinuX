#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
set -euo pipefail
lab=$(cd -- "$(dirname -- "$0")" && pwd)
source "$lab/guardianes.sh"
out=${ULTRA_CUSTODIA_BENCH_OUT:?salida nueva obligatoria}
base=${ULTRA_CUSTODIA_BASE:?baseline previa obligatoria}
[[ ! -e $out ]] || exit 1
mkdir -p "$out"
exec > >(tee "$out/compilacion.txt") 2>&1
gcc --version | head -1; uname -srmo
unit=$ULTRA_GUARD_ROOT/laboratorio/custodia
ultra_revisar "$unit/medir-custodia.c"
ultra_revisar "$unit/medir-custodia.c" -DCOUNT_ALLOC
for optimization in O2 O3; do
    flags=("${ULTRA_WARN[@]}" "${ULTRA_PROD[@]}" "-$optimization" -static-pie)
    printf 'FLAGS %s:' "$optimization"; printf ' %q' "${flags[@]}"; printf '\n'
    for route in before after; do
        root=$ULTRA_GUARD_ROOT
        [[ $route != before ]] || root=$base/guardian
        inputs=("$unit/medir-custodia.c" "$root/laboratorio/custodia/ultra_custodia.c" "$root/produccion/ultra_owner.c")
        gcc "${flags[@]}" "${inputs[@]}" -o "$out/$route-$optimization"
        gcc "${flags[@]}" -DCOUNT_ALLOC "${inputs[@]}" -Wl,--wrap=calloc -Wl,--wrap=free -o "$out/$route-$optimization-count"
        sha256sum "$root/laboratorio/custodia/ultra_custodia.c" "$root/laboratorio/custodia/ultra_custodia.h" "$root/produccion/ultra_owner.c" "$root/produccion/ultra_owner.h" > "$out/fuentes-$route-$optimization.sha256"
        if [[ -f $root/produccion/ultra_custodia.c ]]; then
            sha256sum "$root/produccion/ultra_custodia.c" "$root/produccion/ultra_custodia.h" >> "$out/fuentes-$route-$optimization.sha256"
        fi
    done
done
python "$lab/comparar-custodia.py" "$out" "$lab/aislar-mina.sh"
sha256sum "$unit/medir-custodia.c" "$lab/medir-custodia.sh" "$lab/comparar-custodia.py" "$lab/aislar-mina.sh" "$lab/guardianes.sh" "$ULTRA_GUARD_ROOT/produccion/perfil-gcc.sh" > "$out/herramientas.sha256"
