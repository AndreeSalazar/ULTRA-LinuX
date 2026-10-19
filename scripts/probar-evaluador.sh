#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
set -eu
source "$(dirname -- "$0")/minas-evaluar.sh"
tmp=$(mktemp -d)
trap 'rm -f -- "$tmp/log" "$tmp/log.out"; rmdir -- "$tmp"' EXIT
check() {
    local expected=$1 rc=$2 kind=$3 name=$4 actual=0
    mina_detectada "$rc" "$kind" "$tmp/log" "$name" || actual=1
    [[ $actual -eq $expected ]] || { echo "FALLO: $rc $kind $name"; exit 1; }
}
printf 'MINA bool-invalido: descripcion, sin evidencia\n' > "$tmp/log"
: > "$tmp/log.out"
check 1 1 D bool-invalido
check 1 139 D bool-invalido
printf 'ROJO [GUARDIAN-UB: trampa]\n' > "$tmp/log"
for rc in 0 99 124 125 126 127 137; do check 1 "$rc" D entero-desborde; done
check 0 132 D entero-desborde
check 1 139 D retorno-secuestrado
printf 'SEGV_CPERR: pila sombra\n' > "$tmp/log"
check 0 139 D retorno-secuestrado
check 1 0 N alias-estricto
printf 'NEUTRALIZADA: resultado definido 0\n' > "$tmp/log.out"
check 0 0 N alias-estricto
check 1 1 N alias-estricto
echo 'VERDE: evaluador rechaza errores de infraestructura y exige evidencia'
