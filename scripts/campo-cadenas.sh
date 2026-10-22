#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
set -euo pipefail
lab=$(cd -- "$(dirname -- "$0")" && pwd)
source "$lab/guardianes-lab.sh"
out=${ULTRA_CADENAS_OUT:?usar una carpeta nueva por ejecucion}
[[ ! -e $out ]] || { echo 'ROJO: salida ya existente; no reutilizar evidencia'; exit 1; }
mkdir -p "$out"
exec > >(tee "$out/informe.txt") 2>&1
gcc --version | head -1
uname -srmo
ulimit -c 0
src=$ULTRA_GUARD_ROOT/laboratorio/probar-cadenas.c
ultra_revisar "$src"
ultra_revisar "$ULTRA_GUARD_ROOT/laboratorio/probar-cadena-tarjeta.c"
if ultra_revisar "$ULTRA_GUARD_ROOT/laboratorio/cadena-ignorada.c" > "$out/ignorada.txt" 2>&1; then
    echo 'ROJO: resultado ignorado no rechazado'; exit 1
fi
grep -q 'warn_unused_result' "$out/ignorada.txt"
echo 'REVISION-OK: resultado obligatorio rechazado por warn_unused_result'
for mode in prod-O2 prod-O3 diag-O1 diag-O2; do
    case $mode in
        prod-*) flags=("${ULTRA_PROD[@]}" "-${mode#prod-}" -static-pie);;
        diag-*) flags=("${ULTRA_DIAG[@]}" "-${mode#diag-}");;
    esac
    printf 'PERFIL %s:' "$mode"; printf ' %q' "${ULTRA_WARN[@]}" "${flags[@]}"; printf '\n'
    binary=$out/cadenas-$mode
    gcc "${ULTRA_WARN[@]}" "${flags[@]}" "$src" "$ULTRA_GUARD_ROOT/ultra_guard.c" -o "$binary"
    bash "$lab/aislar-mina.sh" "$binary" > "$out/$mode.txt" 2>&1
    grep -q '^CADENAS-OK: matriz=10368 ' "$out/$mode.txt"
    grep -q '^PAGINAS-OK:' "$out/$mode.txt"
    cat "$out/$mode.txt"
    gcc "${ULTRA_WARN[@]}" "${flags[@]}" -ffunction-sections -fdata-sections -Wl,--gc-sections \
        "$ULTRA_GUARD_ROOT/laboratorio/probar-cadena-tarjeta.c" "$ULTRA_GUARD_ROOT/ultra_guard.c" -o "$out/tarjeta-$mode"
    bash "$lab/aislar-mina.sh" "$out/tarjeta-$mode" > "$out/tarjeta-$mode.txt" 2>&1
    grep -q '^TARJETA-OK:' "$out/tarjeta-$mode.txt"
    cat "$out/tarjeta-$mode.txt"
    rc=0
    bash "$lab/aislar-mina.sh" "$binary" --legacy-source > "$out/legacy-$mode.txt" 2>&1 || rc=$?
    ((rc != 0 && rc != 124 && rc != 125 && rc != 137)) || { cat "$out/legacy-$mode.txt"; echo 'ROJO: contraste no produjo el fallo esperado'; exit 1; }
    if [[ $mode == prod-* ]]; then pattern='FRONTERA-MEMORIA'; else pattern='AddressSanitizer.*(DEADLYSIGNAL|SEGV)|ERROR: AddressSanitizer'; fi
    grep -Eq "$pattern" "$out/legacy-$mode.txt"
    printf 'CONTRASTE-OK: %s legacy sin NUL termina con rc=%s; nueva API rechaza y conserva destino\n' "$mode" "$rc"
done
sha256sum "$ULTRA_GUARD_ROOT/produccion/ultra_string.h" "$src" "$ULTRA_GUARD_ROOT/../tarjeta.c" "$out"/cadenas-* "$out"/tarjeta-* > "$out/SHA256SUMS"
echo 'ALCANCE: spans reales y vivos; no valida punteros arbitrarios ni vida util/concurrencia; parser tarjeta adoptado en fuentes; otras llamadas antiguas sin migrar'
