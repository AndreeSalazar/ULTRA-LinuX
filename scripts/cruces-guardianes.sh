#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
# Positive controls find interference; negative mines find missing coverage.
set -euo pipefail
ulimit -c 0
lab=$(cd -- "$(dirname -- "$0")" && pwd)
source "$lab/guardianes.sh"
g="$lab/guardian"
out="${ULTRA_CRUCES_OUT:-$HOME/linux-reconstruido/build/cruces}"
mkdir -p "$out"
exec > >(tee "$out/informe.txt") 2>&1
gcc --version | head -1
gcc --help=hardened > "$out/hardened.txt"
ultra_revisar "$g/controles.c"
failed=0
for profile in prod-O0 prod-O2 prod-O3 diag-O1 diag-O2; do
    case $profile in
        prod-*) flags=("${ULTRA_PROD[@]}" "-${profile#prod-}" -static-pie);;
        diag-*) flags=("${ULTRA_DIAG[@]}" "-${profile#diag-}");;
    esac
    printf 'PERFIL %s:' "$profile"; printf ' %q' "${flags[@]}"; printf '\n'
    # FORTIFY needs optimization. O0 must be rejected, never silently weakened.
    if [[ $profile = prod-O0 ]]; then
        if gcc "${ULTRA_WARN[@]}" "${flags[@]}" "$g/controles.c" "$g/ultra_guard.c" -o "$out/$profile" > "$out/$profile.build.txt" 2>&1; then
            echo 'ROJO prod-O0: aceptado sin garantizar FORTIFY'
            failed=$((failed + 1))
        elif grep -q "_FORTIFY_SOURCE.*optimizations are turned off" "$out/$profile.build.txt"; then
            echo 'VERDE prod-O0: incompatibilidad FORTIFY/O0 rechazada como se esperaba'
        else
            echo 'ROJO prod-O0: fallo distinto del rechazo esperado'
            failed=$((failed + 1))
        fi
        continue
    fi
    if gcc "${ULTRA_WARN[@]}" "${flags[@]}" "$g/controles.c" "$g/ultra_guard.c" -o "$out/$profile" > "$out/$profile.build.txt" 2>&1 &&
       bash "$lab/aislar-mina.sh" "$out/$profile" > "$out/$profile.run.txt" 2>&1 &&
       grep -q '^CONTROL-OK:' "$out/$profile.run.txt"; then
        echo "VERDE $profile: operaciones validas conservadas"
    else
        echo "ROJO $profile: revisar registros; no bajar protecciones automaticamente"
        failed=$((failed + 1))
    fi
done
echo "RESUMEN cruces: ROJO=$failed (5 perfiles)"
[[ $failed -eq 0 ]]
