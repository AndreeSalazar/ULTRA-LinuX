#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
set -euo pipefail
scripts=$(cd -- "$(dirname -- "$0")" && pwd)
root=$(cd -- "$scripts/.." && pwd)
source "$scripts/guardianes-lab.sh"
out=${ULTRA_RECURSOS_OUT:?salida nueva obligatoria}
[[ ! -e $out ]] || exit 1
mkdir -p "$out"
exec > >(tee "$out/informe.txt") 2>&1
gcc --version | head -1
units=("$root/src/recursos.c" "$ULTRA_GUARD_ROOT/produccion/ultra_custodia.c" "$ULTRA_GUARD_ROOT/produccion/ultra_owner.c")
for unit in "${units[@]}"; do ultra_revisar "$unit"; done
for mode in prod-O2 prod-O3 diag-O1 diag-O2; do
    case $mode in
        prod-*) flags=("${ULTRA_PROD[@]}" "-${mode#prod-}" -static-pie);;
        diag-*) flags=("${ULTRA_DIAG[@]}" "-${mode#diag-}");;
    esac
    gcc "${ULTRA_WARN[@]}" "${flags[@]}" "$scripts/probar-recursos-c.c" "${units[@]:1}" -Wl,--wrap=malloc -Wl,--wrap=calloc -o "$out/test-$mode"
    bash "$scripts/aislar-mina.sh" "$out/test-$mode" > "$out/$mode.txt" 2>&1
    cat "$out/$mode.txt"
    gcc "${ULTRA_WARN[@]}" "${flags[@]}" "${units[@]}" -o "$out/lector-$mode"
    python "$scripts/inspeccionar-custodia-hex.py" "$out/lector-$mode" "$out/hex-$mode" main uc_begin uc_complete ultra_owner_alloc ultra_owner_free
done
python "$scripts/probar_recursos.py" "$root/src/rootfs/bin/ultra-medir-recursos"
python "$scripts/probar_medicion.py" "$root/src/rootfs/bin/ultra-medir-sesion" "$root/src/rootfs/bin/ultra-medir-recursos"
"$out/lector-prod-O2" > "$out/captura-qemu.txt"
set +e
"$out/lector-prod-O2" --apply > "$out/argumento.txt" 2>&1
status=$?
set -e
[[ $status == 2 ]]
set +e
"$out/lector-prod-O2" > /dev/full 2> "$out/salida-fallida.txt"
status=$?
set -e
[[ $status == 1 ]]
echo 'VERDE: error de escritura diferida no devuelve exito'
sha256sum "${units[@]}" "$scripts/probar-recursos-c.c" "$scripts/verificar-recursos-c.sh" "$out"/lector-* "$out"/test-* > "$out/artefactos.sha256"
echo 'RECURSOS-OK: lector local sin privilegios, no ajusta RAM ni discos; QEMU no es medida de SSD fisico'
