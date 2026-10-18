# SPDX-License-Identifier: GPL-2.0-only
# GCC guardian options for ULTRA LinuX C code. Sourced by the build and the minefield.
# Compile time: warnings that hide bugs are errors.
ULTRA_WARN=(-std=c17 -Wall -Wextra -Werror -Wconversion -Wsign-conversion -Wshadow -Wformat=2
            -Wvla -Wnull-dereference -Wimplicit-fallthrough -Wstrict-prototypes)
# Production: -fhardened (FORTIFY 3, stack protector and clash protection, CET,
# zeroed locals, PIE/RELRO) plus UB traps that need no runtime library.
ULTRA_PROD=(-O2 -fhardened -fstrict-flex-arrays=3 -fsanitize=undefined,float-cast-overflow
            -fsanitize-trap=all -fno-strict-aliasing)
# Diagnosis: ASan and full UBSan name the exact fault; slower, never shipped.
ULTRA_DIAG=(-O1 -g -fno-omit-frame-pointer -fsanitize=address,undefined,float-cast-overflow
            -fno-sanitize-recover=all -fcf-protection=full)
# glibc leaves the shadow stack off unless asked. Under WHPX its CPU probe misses
# SHSTK, so hwcaps forces it; the kernel still refuses on a CPU without CET.
export GLIBC_TUNABLES=glibc.cpu.hwcaps=SHSTK:glibc.cpu.x86_shstk=on

# Review gate, separate from the production build. GCC 16's analyzer only sees
# leaks, NULL and uninitialized reads at -O0: optimization folds them away first.
ultra_revisar() {
    local src=$1; shift
    gcc "${ULTRA_WARN[@]}" -O0 -fanalyzer "$@" -c "$src" -o /dev/null &&
    gcc "${ULTRA_WARN[@]}" -O2 "$@" -c "$src" -o /dev/null
}
