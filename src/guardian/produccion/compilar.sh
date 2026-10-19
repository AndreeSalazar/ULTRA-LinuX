#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
# Reusable hosted-C entry point. Never executes the resulting program.
set -euo pipefail
profile=$(cd -- "$(dirname -- "$0")" && pwd)
source "$profile/perfil-gcc.sh"
[[ $# -ge 2 ]] || { echo 'uso: bash compilar.sh SALIDA FUENTE.c [FUENTE.c ...]' >&2; exit 2; }
output=$(realpath -m -- "$1"); shift
sources=()
for src in "$@"; do
    [[ $src = *.c && -f $src ]] || { echo 'ROJO: se exigen fuentes C existentes' >&2; exit 2; }
    resolved=$(realpath -- "$src")
    [[ $resolved != "$output" ]] || { echo 'ROJO: salida coincide con un fuente' >&2; exit 2; }
    sources+=("$resolved")
done
[[ ! -e $output ]] || { echo 'ROJO: la salida ya existe; elegir otra' >&2; exit 2; }
mkdir -p -- "$(dirname -- "$output")"
for src in "${sources[@]}"; do ultra_revisar "$src"; done
stage=$(mktemp "${output}.XXXXXX")
trap 'rm -f -- "$stage"' EXIT
gcc "${ULTRA_WARN[@]}" "${ULTRA_PROD[@]}" -static-pie "${sources[@]}" -o "$stage"
ln -- "$stage" "$output"
printf 'PRODUCCION-OK: %s; GCC revisado; static-pie; sin ejecutar\n' "$output"
