/* SPDX-License-Identifier: GPL-2.0-only */
#include <stdio.h>
extern unsigned ultra_abi_probe(unsigned (*fn)(void));
extern unsigned ultra_abi_good(void);
extern unsigned ultra_abi_bad(void);
extern unsigned ultra_abi_bad_alignment(void);
int main(void) {
    for (unsigned i = 0; i < 1000; ++i) {
        if (ultra_abi_probe(ultra_abi_good) != 0) return 1;
        if (ultra_abi_probe(ultra_abi_bad) != 63) return 2;
        if (ultra_abi_probe(ultra_abi_bad_alignment) != 64) return 3;
        if (ultra_abi_probe(ultra_abi_good) != 0) return 4;
    }
    puts("ASM-OK: 1000 ciclos; ABI correcta, seis registros alterados, alineacion incorrecta y recuperacion");
    return 0;
}
