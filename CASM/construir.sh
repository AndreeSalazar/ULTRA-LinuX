#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-only
# Construye casmc (el compilador de CASM) con GCC. Sin LLVM: si CC no es GCC, se niega.
# Salida: build/CASM/host/casmc y build/CASM/host/toolchain.txt (GCC, flags y huellas).
set -euo pipefail
casm="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
raiz="$(dirname "$casm")"
cc="${CC:-gcc}"
if ! "$cc" -v 2>&1 | grep -q '^gcc version'; then
    echo "RED [TOOLCHAIN] CASM se construye solo con GCC: '$cc' no es GCC." >&2
    exit 1
fi
salida="$raiz/build/CASM/host"
mkdir -p "$salida"
# Avisos como errores, -fanalyzer de GCC y el endurecimiento normal de un programa de Linux.
flags=(-std=c11 -Wall -Wextra -Werror -pedantic -O2 -fanalyzer
       -D_FORTIFY_SOURCE=3 -fstack-protector-strong -fstack-clash-protection -fcf-protection
       -fPIE -pie -Wl,-z,relro,-z,now,-z,noexecstack)
fuentes=("$casm/src/main.c" "$casm/src/frontend.c" "$casm/src/emitter.c")
"$cc" "${flags[@]}" "${fuentes[@]}" -o "$salida/casmc.nuevo"
mv -f "$salida/casmc.nuevo" "$salida/casmc"
{
    "$cc" --version | head -1
    echo "flags: ${flags[*]}"
    (cd "$casm" && sha256sum src/casm.h src/main.c src/frontend.c src/emitter.c)
    sha256sum "$salida/casmc" | sed "s|$raiz/||"
} > "$salida/toolchain.txt"
echo "GREEN [BOOTSTRAP] casmc construido con $("$cc" --version | head -1)"
