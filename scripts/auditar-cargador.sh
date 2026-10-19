#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
set -euo pipefail
lab=$(cd -- "$(dirname -- "$0")" && pwd)
source "$lab/guardianes.sh"
out="${ULTRA_REVISION_OUT:?asignar un directorio por revision}"
mkdir -p "$out/fixture"
exec > >(tee "$out/informe.txt") 2>&1
gcc --version | head -1
sha256sum "$lab/hardware.c" "$lab/init.h" "$lab/guardian/produccion/ultra_guard.c" > "$out/fuentes.sha256"
cat "$out/fuentes.sha256"
printf 'kernel/ultra-audit-inert.ko:\n' > "$out/fixture/modules.dep"
gcc "${ULTRA_WARN[@]}" "${ULTRA_PROD[@]}" -ffunction-sections -fdata-sections -Wl,--gc-sections -static-pie \
    "$lab/guardian/laboratorio/revisar-cargador.c" "$lab/guardian/ultra_guard.c" -o "$out/revisar-cargador"
bash "$lab/aislar-mina.sh" "$out/revisar-cargador"
