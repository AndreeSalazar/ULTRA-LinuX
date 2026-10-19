/* SPDX-License-Identifier: GPL-2.0-only */
/* Defined operations must survive every guardian combination unchanged. */
#include <limits.h>
#include "ultra_guard.h"

int main(void) {
    ultra_guard_install("controles");
    int values[4] = {11, 22, 33, 44};
    size_t i = 0;
    ULTRA_CHECK(ULTRA_AT(values, i++) == 11 && i == 1,
                "CONTROL", "indice evaluado mas de una vez");
    ULTRA_CHECK(ULTRA_AT(values, 3) == 44, "CONTROL", "ultimo indice valido");
    ULTRA_CHECK(ULTRA_ADD(INT_MAX - 1, 1) == INT_MAX &&
                ULTRA_SUB(INT_MIN + 1, 1) == INT_MIN &&
                ULTRA_MUL(12, 13) == 156,
                "CONTROL", "aritmetica definida alterada");
    unsigned int wrap = UINT_MAX;
    ULTRA_CHECK(wrap + 1U == 0U, "CONTROL", "unsigned debe envolver");
    char dst[8];
    const char src[8] = "1234567";
    memcpy(dst, src, sizeof(dst));
    ULTRA_CHECK(!strcmp(dst, src), "CONTROL", "copia exacta rechazada");
    char adjacent[8] = "abc";
    memcpy(adjacent + 4, adjacent, 4);
    ULTRA_CHECK(!strcmp(adjacent + 4, "abc"), "CONTROL", "regiones adyacentes");
    memmove(adjacent + 1, adjacent, 3);
    ULTRA_CHECK(adjacent[1] == 'a', "CONTROL", "memmove solapado valido");
    ULTRA_CHECK(ULTRA_COPY(dst, "a") && ULTRA_APPEND(dst, "bc") &&
                !strcmp(dst, "abc"), "CONTROL", "copia y append validos");
    ULTRA_CHECK(!ULTRA_COPY(dst, "123456789") && dst[7] == 0,
                "CONTROL", "truncamiento debe informar y terminar");
    puts("CONTROL-OK: limites, aritmetica, copia, solape permitido y truncamiento");
    return 0;
}
