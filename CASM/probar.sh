#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-only
# Pruebas de CASM: rechazos de fuente, mutaciones de la emision y el contrato de bytes del
# saludo. El paquete del saludo debe ser identico al .hex escrito a mano (misma huella).
set -euo pipefail
casm="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
raiz="$(dirname "$casm")"
python3 -B "$casm/bootstrap.py" asegurar
(cd "$raiz" && python3 -B "$casm/test.py")
oro="b3fd9814bcce6881f9cf3ad10b299d759f4f5b801a6526c59131a4d1c84e4b33"
hecho="$(sha256sum "$raiz/build/CASM/hello.x86" | cut -d' ' -f1)"
if [ "$hecho" != "$oro" ]; then echo "RED [GOLDEN] hello.x86 $hecho no es el paquete de oro $oro" >&2; exit 1; fi
echo "GREEN [GOLDEN] hello.x86 identico al .hex escrito a mano: $oro"
