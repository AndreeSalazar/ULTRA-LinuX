# SPDX-License-Identifier: GPL-2.0-only
# Evidence rules shared by the minefield and its regression checks.
mina_infra_error() {
    case $1 in 98|124|125|126|127|137) return 0;; esac
    grep -Eq '^bwrap:|^prlimit:|^timeout: failed|LeakSanitizer has encountered a fatal error|AddressSanitizer failed to allocate|FATAL: ThreadSanitizer:' "$2"
}
mina_detectada() {
    local rc=$1 kind=$2 file=$3 name=$4 pattern
    mina_infra_error "$rc" "$file" && return 1
    # Timeouts, missing executables and escaped mines never count as protection.
    case $rc in 0) [[ $kind = N ]] && grep -q '^NEUTRALIZADA:' "$file.out"; return;;
        98|99|124|125|126|127|137) return 1;; esac
    [[ $kind = D ]] || return 1
    case $name in
        retorno-secuestrado) pattern='SEGV_CPERR:';;
        ejecutar-pila|escribir-literal|secuestrar-arranque) pattern='SEGV_ACCERR:';;
        recursion-infinita|alloca-gigante) pattern='SEGV_MAPERR:|ERROR: AddressSanitizer: stack-overflow';;
        ultra-suma|ultra-resta|ultra-producto|tamano-producto) pattern='^ROJO \[UB-DESBORDE\]';;
        ultra-indice) pattern='^ROJO \[FRONTERA-INDICE\]';;
        ultra-division-cero|ultra-division-min) pattern='^ROJO \[UB-DIVISION\]';;
        ultra-resto-cero|ultra-resto-min) pattern='^ROJO \[UB-RESTO\]';;
        ultra-shl-*) pattern='^ROJO \[UB-DESPLAZAMIENTO\]';;
        memcpy-solapado) pattern='^ROJO \[FRONTERA-SOLAPE\]|ERROR: AddressSanitizer: memcpy-param-overlap';;
        copia-pila|desborde-heap|tamano-vuelta|signo-tamano) pattern='^\*\*\* buffer overflow detected|ERROR: AddressSanitizer:.*buffer-overflow';;
        pila-destruida) pattern='^\*\*\* stack smashing detected|ERROR: AddressSanitizer: stack-buffer-overflow';;
        formato-n) pattern='^\*\*\* %n in writable segments detected';;
        doble-free|free-pila|free-medio|heap-subdesborde) pattern='^free\(\):|^double free or corruption|^munmap_chunk\(\):|ERROR: AddressSanitizer:';;
        uso-tras-free|lectura-fuera|uso-tras-retorno) pattern='ERROR: AddressSanitizer:';;
        cadena-sin-nul) pattern='ERROR: AddressSanitizer: heap-buffer-overflow';;
        uso-tras-scope) pattern='ERROR: AddressSanitizer: stack-use-after-scope';;
        comparar-punteros-ajenos|restar-punteros-ajenos) pattern='ERROR: AddressSanitizer: invalid-pointer-pair';;
        *) pattern='^ROJO \[GUARDIAN-UB:|runtime error:';;
    esac
    grep -Eq "$pattern" "$file"
}
