#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-only
# Compila un programa CASM a un paquete FAGRAW64 verificado (x86-64, ABI metal64).
# Uso: CASM/compilar.sh [fuente.casm]   (sin argumento: examples/hello.casm)
# Cada compilacion queda en build/CASM/run-*: fuente congelada, HEX, IR, paquete y
# verified.json. Si algo se rechaza, no se publica INICIO.X86. No toca discos ni el arranque.
set -euo pipefail
casm="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
raiz="$(dirname "$casm")"
fuente="${1:-$casm/examples/hello.casm}"
[ -f "$fuente" ] && [ ! -L "$fuente" ] || { echo "RED [HOST-READ] No es un archivo regular: $fuente" >&2; exit 1; }
"$casm/construir.sh"
mkdir -p "$raiz/build/CASM"
run="$(mktemp -d "$raiz/build/CASM/run-$(date +%Y%m%d-%H%M%S)-XXXX")"
# La misma fuente congelada para el compilador y para el verificador.
cp -- "$fuente" "$run/source.casm"
realpath -- "$fuente" > "$run/origen.txt"
cp "$raiz/build/CASM/host/toolchain.txt" "$run/toolchain.txt"
# Todo lo que dicen el compilador y el verificador queda en registro.txt (el SUPER GRAFO lo lee).
registro="$run/registro.txt"
if ! "$raiz/build/CASM/host/casmc" "$run/source.casm" "$run/program.hex" "$run/program.ir.json" > >(tee -a "$registro") 2> >(tee -a "$registro" >&2); then
    wait; echo "RED [COMPILE] Fuente rechazada. No se publico ningun paquete. Run: $run" | tee -a "$registro" >&2; exit 1
fi
wait
if ! python3 -B "$casm/verify.py" "$run/source.casm" "$run/program.hex" "$run/program.ir.json" \
        --packet "$run/INICIO.X86" --report "$run/verified.json" > >(tee -a "$registro") 2> >(tee -a "$registro" >&2); then
    wait; echo "RED [COMPILE] Emision rechazada. Run: $run" | tee -a "$registro" >&2; exit 1
fi
wait
echo "GREEN [READY] Paquete verificado: $run/INICIO.X86"
echo "No se toco ningun disco, entrada de arranque ni paquete instalado."
