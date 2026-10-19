#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
# Walks the minefield: every mine must wake a guardian, at compile time or at run time.
# Each runtime mine runs three ways: raw (no guardian), production and diagnosis.
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
{ ultra_revisar "$g/ultra_guard.c" && gcc "${ULTRA_WARN[@]}" "${ULTRA_PROD[@]}" -c "$g/ultra_guard.c" -o "$out/ultra_guard.o"; } ||
    { log 'ROJO: ultra_guard.c no pasa sus propios guardianes'; exit 1; }
# Mines are wrong on purpose, so only their compile-time warnings are muted.
gcc "${ULTRA_PROD[@]}" -w -static-pie "$g/minas.c" "$g/ultra_guard.c" -o "$out/minas-prod" || exit 1
gcc "${ULTRA_DIAG[@]}" -w "$g/minas.c" "$g/ultra_guard.c" -o "$out/minas-diag" || exit 1
gcc "${ULTRA_CRUDO[@]}" -w "$g/minas.c" "$g/ultra_guard.c" -o "$out/minas-crudo" || exit 1

c_ok=0 c_bad=0 stopped=0 neutralized=0 diag_only=0 missed=0 raw_silent=0
log ''
log '== Minas de compilacion: el codigo no debe compilar =='
for f in "$g"/minas-compilacion/*.c; do
    name=$(basename "$f" .c)
    why=$(sed -n 's|^/\* BANDERA: \(.*\) \*/$|\1|p' "$f")
    if ultra_revisar "$f" 2> "$out/c-$name.txt"; then
        log "ROJO     $name: compilo sin que ningun guardian despertara ($why)"; c_bad=$((c_bad+1))
    else
        log "VERDE    $name: $why"
        log "         bandera: $(first 'error' "$out/c-$name.txt" | sed 's|^.*minas-compilacion/||')"
        c_ok=$((c_ok+1))
    fi
done

# Describes one run: how the mine ended and the flag it left, if any.
describe() {
    local rc=$1 file=$2 kind=$3
    if [[ $kind = N ]]; then
        if [[ $rc -eq 0 ]]; then echo "$(first 'NEUTRALIZADA' "$file.out")"
        else echo "$(first 'UB VISIBLE' "$file")"; fi
        return
    fi
    if [[ $rc -eq 99 ]]; then echo 'explota en silencio: el programa siguio como si nada'; return; fi
    if [[ $rc -eq 0 ]]; then echo 'termino normal sin detectar nada'; return; fi
    local flag
    flag=$(first 'ROJO|^\*\*\*|runtime error|ERROR: AddressSanitizer|ERROR: LeakSanitizer|free\(\)|double|corrupted|munmap|invalid' "$file" | sed 's|^.*guardian/||')
    if [[ -n $flag ]]; then echo "$flag"
    elif [[ $rc -gt 128 ]]; then echo "muere sin explicacion (senal $((rc-128)))"
    else echo "sale con codigo $rc sin explicacion"; fi
}
caught() { [[ $1 -ne 99 && ( $2 = N && $1 -eq 0 || $2 = D && $1 -ne 0 ) ]]; }

log ''
log '== Minas de ejecucion: sin guardianes / produccion (trampas, FORTIFY, CET) / diagnostico (ASan+UBSan) =='
log "pila sombra en produccion: $("$out/minas-prod" --cet)"
while IFS='|' read -r name kind why twin; do
    for mode in crudo prod diag; do
        { "$out/minas-$mode" "$name" > "$out/$mode-$name.txt.out" 2> "$out/$mode-$name.txt"; } 2> /dev/null
        printf -v "rc_$mode" '%d' $?
    done
    if caught "$rc_prod" "$kind"; then
        if [[ $kind = N ]]; then state=NEUTRA; neutralized=$((neutralized+1)); else state=VERDE; stopped=$((stopped+1)); fi
    elif caught "$rc_diag" "$kind"; then state=AMARILLO; diag_only=$((diag_only+1))
    elif [[ -n $twin ]] && ! ultra_revisar "$g/minas-compilacion/$twin.c" 2> /dev/null; then
        state=AMARILLO; diag_only=$((diag_only+1)) # only the review gate stops it, never the running program
    else state=ROJO; missed=$((missed+1)); fi
    # Raw counts as silent when the mine ran on (99/0) or died with no explanation.
    raw=$(describe "$rc_crudo" "$out/crudo-$name.txt" "$kind")
    case $raw in explota*|termino*|UB*|muere*|sale*) raw_silent=$((raw_silent+1));; esac
    log "$(printf '%-8s %s: %s' "$state" "$name" "$why")"
    log "         sin guardianes: $raw"
    log "         produccion:     $(describe "$rc_prod" "$out/prod-$name.txt" "$kind")"
    cause=$(first 'SEGV_(CPERR|MAPERR|ACCERR)' "$out/prod-$name.txt" | grep -o '(SEGV_.*)'); [[ -n $cause ]] && log "                         $cause"
    log "         diagnostico:    $(describe "$rc_diag" "$out/diag-$name.txt" "$kind")"
    [[ -n $twin ]] && log "         gemela de compilacion: $twin (la revision la para antes de ejecutar)"
done < <("$out/minas-prod" --lista)

total_run=$((stopped + neutralized + diag_only + missed))
log ''
log "RESUMEN compilacion: VERDE=$c_ok ROJO=$c_bad"
log "RESUMEN ejecucion:   VERDE=$stopped NEUTRA=$neutralized AMARILLO=$diag_only ROJO=$missed (de $total_run)"
log "SIN GUARDIANES:      $raw_silent de $total_run minas explotan en silencio o mueren sin explicar por que"
[[ $c_bad -eq 0 && $missed -eq 0 ]]
