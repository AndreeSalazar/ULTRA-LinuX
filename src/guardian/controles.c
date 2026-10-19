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
    ULTRA_CHECK(ULTRA_DIV_INT(INT_MIN, 1) == INT_MIN &&
                ULTRA_DIV_INT(-7, 3) == -2 && ULTRA_DIV_INT(7, -3) == -2 &&
                ULTRA_REM_INT(INT_MIN, 1) == 0 && ULTRA_REM_INT(-7, 3) == -1 &&
                ULTRA_REM_INT(7, -3) == 1,
                "CONTROL", "division y resto definidos alterados");
    ULTRA_CHECK(ULTRA_SHL_INT(INT_MAX, 0) == INT_MAX &&
                ULTRA_SHL_INT(1, (int)(sizeof(int) * CHAR_BIT) - 2) == INT_MAX / 2 + 1 &&
                ULTRA_SHL_INT(0, (int)(sizeof(int) * CHAR_BIT) - 1) == 0,
                "CONTROL", "desplazamientos en limites validos");
    int dividend = 12, divisor = 3;
    int quotient = ULTRA_DIV_INT(dividend++, divisor++);
    ULTRA_CHECK(quotient == 4 && dividend == 13 && divisor == 4,
                "CONTROL", "division evalua argumentos mas de una vez");
    int numerator = 13, denominator = 3;
    int remainder = ULTRA_REM_INT(numerator++, denominator++);
    ULTRA_CHECK(remainder == 1 && numerator == 14 && denominator == 4,
                "CONTROL", "resto evalua argumentos mas de una vez");
    int base = 2, count = 1;
    int shifted = ULTRA_SHL_INT(base++, count++);
    ULTRA_CHECK(shifted == 4 && base == 3 && count == 2,
                "CONTROL", "desplazamiento evalua argumentos mas de una vez");
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
    puts("CONTROL-OK: limites, aritmetica, division, resto, desplazamiento, copia y truncamiento");
    return 0;
}
