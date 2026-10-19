# SPDX-License-Identifier: GPL-2.0-only
# GCC guardian options for ULTRA LinuX C code. Sourced by the build and the minefield.
# Compile time: warnings that hide bugs are errors.
ULTRA_WARN=(-std=c17 -Wall -Wextra -Werror -Wconversion -Wsign-conversion -Wshadow -Wformat=2
            -Wvla -Wnull-dereference -Wimplicit-fallthrough -Wstrict-prototypes
            -Walloca -Wcast-qual -Wcast-align=strict -Wtrampolines)
# Production: -fhardened (FORTIFY 3, stack protector and clash protection, CET,
# zeroed locals, PIE/RELRO) plus UB traps that need no runtime library.
ULTRA_PROD=(-O2 -fhardened -fstrict-flex-arrays=3 -fsanitize=undefined,float-cast-overflow,bounds-strict
            -fsanitize-trap=all -fno-strict-aliasing)
# Diagnosis: ASan and full UBSan name the exact fault; slower, never shipped.
ULTRA_DIAG=(-O1 -g -fno-omit-frame-pointer -fsanitize=address,undefined,float-cast-overflow,bounds-strict,pointer-compare,pointer-subtract
            -fno-sanitize-recover=all -fcf-protection=full)
export ASAN_OPTIONS=detect_stack_use_after_return=1:detect_leaks=1:detect_invalid_pointer_pairs=2
# Raw: every guardian off, including Arch's default stack protector and RELRO.
# Only for the minefield, to show what each mine does when nobody watches.
ULTRA_CRUDO=(-O2 -fno-stack-protector -fno-stack-clash-protection -fcf-protection=none
             -U_FORTIFY_SOURCE -D_FORTIFY_SOURCE=0 -Wl,-z,norelro -DULTRA_CRUDO)
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
