#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
set -euo pipefail
lab=$(cd -- "$(dirname -- "$0")" && pwd)
source "$lab/guardianes-lab.sh"
source "$lab/minas-evaluar.sh"
out="${ULTRA_DOMINIOS_OUT:-$HOME/linux-reconstruido/build/dominios}"
mkdir -p "$out"
exec > >(tee "$out/informe.txt") 2>&1
gcc --version | head -1
ultra_revisar "$ULTRA_GUARD_ROOT/laboratorio/probar-ctype.c"
gcc "${ULTRA_WARN[@]}" "${ULTRA_PROD[@]}" -static-pie "$ULTRA_GUARD_ROOT/laboratorio/probar-ctype.c" "$lab/guardian/ultra_guard.c" -o "$out/ctype-prod"
gcc "${ULTRA_WARN[@]}" "${ULTRA_DIAG[@]}" "$ULTRA_GUARD_ROOT/laboratorio/probar-ctype.c" "$lab/guardian/ultra_guard.c" -o "$out/ctype-diag"
count=0
for mode in prod diag; do
    bash "$lab/aislar-mina.sh" "$out/ctype-$mode" > "$out/validos-$mode.txt" 2>&1
    grep -q '^CTYPE-VALIDO: 256 bytes y EOF;' "$out/validos-$mode.txt"
    cat "$out/validos-$mode.txt"
    for op in tolower toupper isalpha; do
        for input in negativo grande extremo; do
            file="$out/$mode-$op-$input.txt"
            rc=0
            { bash "$lab/aislar-mina.sh" "$out/ctype-$mode" "$op" "$input" > "$file" 2>&1; } 2>/dev/null || rc=$?
            if mina_infra_error "$rc" "$file" || [[ $rc != 134 ]] || ! grep -q '^ROJO \[UB-CTYPE\]' "$file"; then
                echo "ROJO: $mode $op $input rc=$rc"; cat "$file"; exit 1
            fi
            echo "VERDE: $mode $op $input rechazado ANTES de llamar a ctype (UB-CTYPE)"
            count=$((count+1))
        done
    done
done
echo "DOMINIOS-OK: $count/18 rechazos; 2 perfiles con 257 entradas validas x 3 operaciones; no se ejecuta ctype fuera de dominio"
