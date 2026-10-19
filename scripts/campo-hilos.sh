#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
set -euo pipefail
lab=$(cd -- "$(dirname -- "$0")" && pwd)
source "$lab/guardianes-lab.sh"
out="${ULTRA_HILOS_OUT:-$HOME/linux-reconstruido/build/hilos}"
mkdir -p "$out"
exec > >(tee "$out/informe.txt") 2>&1
gcc --version | head -1
src="$ULTRA_GUARD_ROOT/laboratorio/minas-hilos.c"
flags=("${ULTRA_WARN[@]}" -O1 -g -fno-omit-frame-pointer -pthread -fsanitize=thread)
gcc "${flags[@]}" "$src" -o "$out/hilos" > "$out/build.txt" 2>&1
# Incompatible sanitizers must not be accidentally merged into one profile.
if gcc -std=c17 -pthread -fsanitize=address,thread -c "$src" -o /dev/null > "$out/incompatibilidad.txt" 2>&1; then
    echo 'ROJO: GCC acepto ASan y TSan juntos'; exit 1
fi
grep -q 'incompatible with.*-fsanitize=' "$out/incompatibilidad.txt" ||
    { echo 'ROJO: rechazo por causa inesperada'; exit 1; }
echo 'VERDE: ASan/TSan rechazados juntos; diagnosticos en perfiles separados'
if bash "$lab/aislar-mina.sh" "$out/hilos" atomic > "$out/atomic.txt" 2>&1 &&
   grep -q '^HILOS-OK:' "$out/atomic.txt"; then
    echo 'VERDE: control atomico sin avisos de carrera'
else
    echo 'ROJO: control atomico o runtime TSan fallo; revisar atomic.txt'; exit 1
fi
rc=0
bash "$lab/aislar-mina.sh" "$out/hilos" race > "$out/race.txt" 2>&1 || rc=$?
if [[ $rc -eq 66 ]] && grep -q 'WARNING: ThreadSanitizer: data race' "$out/race.txt"; then
    echo 'AMARILLO: carrera detectada por TSan; produccion no instrumenta carreras'
else
    echo "ROJO: carrera sin evidencia TSan valida (rc=$rc)"; exit 1
fi
echo 'RESUMEN hilos: control atomico VERDE, incompatibilidad VERDE, carrera AMARILLO'
