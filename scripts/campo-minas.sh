#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
# Walks the minefield: every mine must wake a guardian, at compile time or at run time.
set -uo pipefail
lab=$(cd -- "$(dirname -- "$0")" && pwd)
source "$lab/guardianes.sh"
g="$lab/guardian"
out="$HOME/linux-reconstruido/build/minas"
mkdir -p "$out"
report="$out/informe.txt"
: > "$report"
log() { printf '%s\n' "$*" | tee -a "$report"; }
first() { grep -m1 -E "$1" "$2" | sed 's/^[[:space:]]*//' | cut -c1-150; }

log "$(gcc --version | head -n 1)"
# The guardian library itself must pass every compile-time guardian.
{ ultra_revisar "$g/ultra_guard.c" && gcc "${ULTRA_WARN[@]}" "${ULTRA_PROD[@]}" -c "$g/ultra_guard.c" -o "$out/ultra_guard.o"; } || { log 'ROJO: ultra_guard.c no pasa sus propios guardianes'; exit 1; }
# Mines are wrong on purpose, so only their compile-time warnings are muted.
gcc "${ULTRA_PROD[@]}" -w -static "$g/minas.c" "$g/ultra_guard.c" -o "$out/minas-prod" || exit 1
gcc "${ULTRA_DIAG[@]}" -w "$g/minas.c" "$g/ultra_guard.c" -o "$out/minas-diag" || exit 1

verde=0 amarillo=0 rojo=0
log ''
log '== Minas de compilacion: el codigo no debe compilar =='
for f in "$g"/minas-compilacion/*.c; do
    name=$(basename "$f" .c)
    why=$(sed -n 's|^/\* BANDERA: \(.*\) \*/$|\1|p' "$f")
    if ultra_revisar "$f" 2> "$out/c-$name.txt"; then
        log "ROJO     $name: compilo sin que ningun guardian despertara ($why)"; rojo=$((rojo+1))
    else
        log "VERDE    $name: $why"
        log "         bandera: $(first 'error' "$out/c-$name.txt" | sed 's|^.*minas-compilacion/||')"
        verde=$((verde+1))
    fi
done

log ''
log '== Minas de ejecucion: produccion (trampas, FORTIFY, CET) / diagnostico (ASan+UBSan) =='
log "pila sombra en produccion: $("$out/minas-prod" --cet)"
while IFS='|' read -r name why; do
    { "$out/minas-prod" "$name" > /dev/null 2> "$out/p-$name.txt"; } 2> /dev/null; prod=$?
    { "$out/minas-diag" "$name" > /dev/null 2> "$out/d-$name.txt"; } 2> /dev/null; diag=$?
    caught() { [[ $1 -ne 0 && $1 -ne 99 ]]; }
    # Production is what ships: it decides VERDE. Diagnosis-only is a known gap.
    if caught $prod; then state=VERDE; verde=$((verde+1))
    elif caught $diag; then state=AMARILLO; amarillo=$((amarillo+1))
    else state=ROJO; rojo=$((rojo+1)); fi
    log "$(printf '%-8s %s: %s' "$state" "$name" "$why")"
    if caught $prod; then
        log "         produccion: $(first '^\*\*\*|ROJO|free\(\)|double' "$out/p-$name.txt")"
        cperr=$(first 'SEGV_CPERR' "$out/p-$name.txt"); [[ -n $cperr ]] && log "         $cperr"
    else
        log "         produccion: NO DETECTADA (salida $prod)"
    fi
    if caught $diag; then
        log "         diagnostico: $(first 'runtime error|ERROR: AddressSanitizer|ROJO|^\*\*\*' "$out/d-$name.txt" | sed 's|^.*guardian/||')"
    else
        log "         diagnostico: NO DETECTADA (salida $diag)"
    fi
done < <("$out/minas-prod" --lista)

log ''
log "RESUMEN: VERDE=$verde AMARILLO=$amarillo ROJO=$rojo"
[[ $rojo -eq 0 ]]
