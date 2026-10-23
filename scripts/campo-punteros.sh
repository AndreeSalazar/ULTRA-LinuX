#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
set -euo pipefail
lab=$(cd -- "$(dirname -- "$0")" && pwd)
source "$lab/guardianes-lab.sh"
out=${ULTRA_PUNTEROS_OUT:?salida nueva obligatoria}
[[ ! -e $out ]] || exit 1
mkdir -p "$out"
exec > >(tee "$out/informe.txt") 2>&1
gcc --version | head -1
uname -srmo
ULTRA_OWNER_ASM_OUT="$out/regresion" bash "$lab/campo-owner-asm.sh"
fixture=$ULTRA_GUARD_ROOT/laboratorio/punteros-limites.c
owner=$ULTRA_GUARD_ROOT/produccion/ultra_owner.c
# These deliberate invalid callers are NOT compiled as production applications.
gcc -std=c17 -O1 -g -Wall -Wextra -fsanitize=address,undefined -fno-omit-frame-pointer \
    "$fixture" "$owner" -o "$out/limites" > "$out/compilacion-negativos.txt" 2>&1
bash "$lab/aislar-mina.sh" "$out/limites" cruce-pool > "$out/cruce-pool.txt" 2>&1
grep -q '^LIMITACION_CONFIRMADA:' "$out/cruce-pool.txt"
cat "$out/cruce-pool.txt"
for pair in 'externo-corto:stack-buffer-overflow' 'externo-caducado:heap-use-after-free' 'pool-caducado:heap-use-after-free'; do
    name=${pair%%:*}; diagnostic=${pair#*:}
    set +e
    bash "$lab/aislar-mina.sh" "$out/limites" "$name" > "$out/$name.txt" 2>&1
    status=$?
    set -e
    [[ $status -ne 0 && $status -ne 124 && $status -ne 125 ]]
    grep -q "ERROR: AddressSanitizer: $diagnostic" "$out/$name.txt"
    printf 'LIMITACION_DIAGNOSTICADA: %s / %s / salida=%s\n' "$name" "$diagnostic" "$status"
done
sha256sum "$fixture" "$owner" "$ULTRA_GUARD_ROOT/produccion/ultra_owner.h" > "$out/fuentes.sha256"
echo 'ALCANCE: capacidades externas explicitas; no valida vida util de punteros crudos ni identidad entre pools; no despliegue en A:'
