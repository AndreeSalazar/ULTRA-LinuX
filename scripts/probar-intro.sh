#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-only
# Intro de arranque bajo ASan y UBSan, con el guardian real de memcpy. Sin /dev/fb0.
set -euo pipefail
raiz=$(cd -- "$(dirname -- "$0")/.." && pwd)
salida=${1:-$(mktemp -d)}
mkdir -p "$salida"
gcc -std=c17 -O1 -g -w -fsanitize=address,undefined -fno-sanitize-recover=all -I "$raiz/src" \
    -o "$salida/arnes" "$raiz/scripts/probar_intro.c" "$raiz/src/guardian/ultra_guard.c"
resultado=$("$salida/arnes" "$raiz/src/rootfs/etc/ultra-intro.bin" "$salida")
echo "$resultado"
[[ $resultado == *"identica: SI"* ]] || { echo 'ROJO: la intro no devolvio la pantalla'; exit 1; }
echo "VERDE: intro sin errores de memoria; cuadros en $salida"
