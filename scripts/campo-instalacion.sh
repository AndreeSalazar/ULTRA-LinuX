#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
set -euo pipefail
lab=$(cd -- "$(dirname -- "$0")" && pwd)
source "$lab/guardianes-lab.sh"
out=${ULTRA_INSTALACION_OUT:?usar una carpeta nueva por ejecucion}
[[ ! -e $out ]] || exit 1
mkdir -p "$out"
exec > >(tee "$out/informe.txt") 2>&1
gcc --version | head -1
uname -srmo
src=$ULTRA_GUARD_ROOT/laboratorio/probar-instalacion.c
ultra_revisar "$src"
for mode in prod-O2 prod-O3 diag-O1 diag-O2; do
    case $mode in
        prod-*) flags=("${ULTRA_PROD[@]}" "-${mode#prod-}" -static-pie);;
        diag-*) flags=("${ULTRA_DIAG[@]}" "-${mode#diag-}");;
    esac
    printf 'PERFIL %s:' "$mode"; printf ' %q' "${ULTRA_WARN[@]}" "${flags[@]}"; printf '\n'
    binary=$out/instalacion-$mode
    gcc "${ULTRA_WARN[@]}" "${flags[@]}" -Wl,--wrap=sigaltstack,--wrap=sigaction,--wrap=sigemptyset \
        "$src" "$ULTRA_GUARD_ROOT/ultra_guard.c" -o "$binary"
    bash "$lab/aislar-mina.sh" "$binary" ok > "$out/$mode-ok.txt" 2>&1
    grep -q '^INSTALACION-OK:' "$out/$mode-ok.txt"
    for scenario in stack mask 1 2 3 4 5 6; do
        rc=0
        bash "$lab/aislar-mina.sh" "$binary" "$scenario" > "$out/$mode-$scenario.txt" 2>&1 || rc=$?
        [[ $rc == 78 ]] || { cat "$out/$mode-$scenario.txt"; echo "ROJO: fallo no detenido: $scenario rc=$rc"; exit 1; }
        case $scenario in stack) expected='paso=sigaltstack errno=1';; mask) expected='paso=sigemptyset errno=22';; *) expected='paso=sigaction errno=1';; esac
        grep -Fq "$expected" "$out/$mode-$scenario.txt"
        ! grep -q 'INSTALACION-INCOMPLETA-CONTINUO' "$out/$mode-$scenario.txt"
    done
    rc=0
    bash "$lab/aislar-mina.sh" "$binary" trap > "$out/$mode-trap.txt" 2>&1 || rc=$?
    [[ $rc == 132 ]]
    grep -q '^INSTALACION-OK:' "$out/$mode-trap.txt"
    grep -q 'ROJO \[GUARDIAN-UB' "$out/$mode-trap.txt"
    echo "INSTALACION-PERFIL-OK: $mode; normal=1 fallos=8 trampa=1"
done
sha256sum "$src" "$ULTRA_GUARD_ROOT/produccion/ultra_guard.c" "$ULTRA_GUARD_ROOT/produccion/ultra_guard.h" \
    "$out"/instalacion-* > "$out/SHA256SUMS"
echo 'ALCANCE: inyeccion por wrappers solo de pruebas; sin hooks en produccion ni certificacion global de senales/threads'
