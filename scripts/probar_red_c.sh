#!/usr/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
# Run unprivileged in the workshop; source layout is repo/src or VM/lab.
set -euo pipefail
src=$(realpath -- "${1:?Uso: probar_red_c.sh SRC_DIR NUEVO_OUT_DIR}")
out=$(realpath -m -- "${2:?Falta directorio nuevo de resultados}")
mkdir -- "$out" # Never overwrite previous evidence.
source "$src/guardian/produccion/perfil-gcc.sh"
gcc --version > "$out/compilador.txt"
uname -a > "$out/entorno.txt"
ultra_revisar "$src/red.c" > "$out/analizador.txt" 2>&1
ultra_revisar "$src/guardian/laboratorio/probar-red.c" >> "$out/analizador.txt" 2>&1
for profile in generico pc; do
    extra=()
    [[ $profile != pc ]] || extra=(-O3 -march=znver3 -flto=auto)
    gcc "${ULTRA_WARN[@]}" "${ULTRA_PROD[@]}" "${extra[@]}" -static-pie \
        "$src/red.c" "$src/guardian/ultra_guard.c" -o "$out/observador-$profile" 2> "$out/compilar-$profile.txt"
    gcc "${ULTRA_WARN[@]}" "${ULTRA_PROD[@]}" "${extra[@]}" -static-pie \
        "$src/guardian/laboratorio/probar-red.c" "$src/guardian/ultra_guard.c" \
        -o "$out/controles-$profile" 2>> "$out/compilar-$profile.txt"
    "$out/controles-$profile" > "$out/controles-$profile.txt"
    readelf -n "$out/observador-$profile" > "$out/elf-$profile.txt"
    grep -q 'x86 feature: IBT, SHSTK' "$out/elf-$profile.txt"
done
source "$src/guardian/laboratorio/perfil-gcc.sh"
gcc "${ULTRA_WARN[@]}" "${ULTRA_DIAG[@]}" \
    "$src/guardian/laboratorio/probar-red.c" "$src/guardian/ultra_guard.c" \
    -o "$out/controles-diagnostico" 2> "$out/compilar-diagnostico.txt"
"$out/controles-diagnostico" > "$out/controles-diagnostico.txt" 2>&1
sha256sum "$src/red.c" "$src/guardian/laboratorio/probar-red.c" > "$out/fuentes.sha256"
printf 'VERDE [RED-C]: analizador, generico, Zen 3 y ASan/UBSan; %s\n' "$out"
