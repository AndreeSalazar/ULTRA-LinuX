#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
set -euo pipefail
lab=$(cd -- "$(dirname -- "$0")" && pwd)
source "$lab/guardianes.sh"
out="${ULTRA_PERFILES_OUT:-$HOME/linux-reconstruido/build/perfiles}"
mkdir -p "$out"
exec > >(tee "$out/informe.txt") 2>&1
env -u ASAN_OPTIONS -u TSAN_OPTIONS -u UBSAN_OPTIONS bash -eu -c '
    source "$1"
    [[ ${#ULTRA_PROD[@]} -gt 0 && ${#ULTRA_WARN[@]} -gt 0 ]]
    [[ ! -v ULTRA_DIAG && ! -v ULTRA_CRUDO && ! -v ASAN_OPTIONS && ! -v TSAN_OPTIONS ]]
' _ "$lab/guardianes.sh"
echo 'PERFIL-OK: produccion no carga ASan, TSan ni perfil crudo'
bash -eu -c '
    source "$1/guardianes-lab.sh"
    [[ ${#ULTRA_DIAG[@]} -gt 0 && ${#ULTRA_CRUDO[@]} -gt 0 && -v ASAN_OPTIONS ]]
    source "$1/guardianes.sh"
    [[ ! -v ULTRA_DIAG && ! -v ULTRA_CRUDO && ! -v ASAN_OPTIONS && ! -v TSAN_OPTIONS ]]
' _ "$lab"
echo 'PERFIL-OK: volver de laboratorio a produccion limpia el estado experimental'
printf 'int main(void) { return 0; }\n' > "$out/valido.c"
printf 'int main(void) { return 1 / 0; }\n' > "$out/invalido.c"
compiler="$ULTRA_GUARD_ROOT/produccion/compilar.sh"
bash "$compiler" "$out/valido" "$out/valido.c"
bash "$lab/aislar-mina.sh" "$out/valido"
if bash "$compiler" "$out/invalido" "$out/invalido.c" > "$out/rechazo.txt" 2>&1; then
    echo 'ROJO: fuente invalido aceptado'; exit 1
fi
grep -q 'division by zero' "$out/rechazo.txt"
[[ ! -e "$out/invalido" ]]
before=$(sha256sum "$out/valido")
if bash "$compiler" "$out/valido" "$out/invalido.c" > "$out/colision.txt" 2>&1; then
    echo 'ROJO: sobrescribio una salida existente'; exit 1
fi
grep -q 'salida ya existe' "$out/colision.txt"
[[ $(sha256sum "$out/valido") = "$before" ]]
# The current boot consumer still compiles the compatibility source.
gcc "${ULTRA_PROD[@]}" -w -static-pie "$ULTRA_GUARD_ROOT/minas.c" "$ULTRA_GUARD_ROOT/ultra_guard.c" -o "$out/compat-minas"
bash "$lab/aislar-mina.sh" "$out/compat-minas" --lista > "$out/compat-lista.txt"
[[ $(wc -l < "$out/compat-lista.txt") -eq 62 ]]
echo 'PERFILES-OK: separacion, compilador reutilizable, rechazo con causa, salida protegida y consumidor de arranque verificados'
