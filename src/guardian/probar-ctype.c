/* SPDX-License-Identifier: GPL-2.0-only */
#include "ultra_guard.h"

static void valid(int c) {
    ULTRA_CHECK(ULTRA_TOLOWER(c) == tolower(c) && ULTRA_TOUPPER(c) == toupper(c) &&
                !!ULTRA_ISALPHA(c) == !!isalpha(c), "TEST-CTYPE", "cambio la semantica de una entrada valida");
}

int main(int argc, char **argv) {
    ultra_guard_install("dominio-ctype");
    if (argc == 1) {
        for (int c = 0; c <= UCHAR_MAX; c++) valid(c);
        valid(EOF);
        int byte = 'A';
        ULTRA_CHECK(ULTRA_TOLOWER(byte++) == 'a' && byte == 'A' + 1,
                    "TEST-CTYPE", "argumento evaluado mas de una vez");
        byte = 'a';
        ULTRA_CHECK(ULTRA_TOUPPER(byte++) == 'A' && byte == 'a' + 1,
                    "TEST-CTYPE", "argumento evaluado mas de una vez");
        byte = 'A';
        ULTRA_CHECK(ULTRA_ISALPHA(byte++) && byte == 'A' + 1,
                    "TEST-CTYPE", "argumento evaluado mas de una vez");
        printf("CTYPE-VALIDO: %u bytes y EOF; tres operaciones; evaluacion unica\n", (unsigned int)UCHAR_MAX + 1u);
        return 0;
    }
    if (argc != 3) return 2;
    int c;
    if (!strcmp(argv[2], "negativo")) c = EOF == -2 ? -3 : -2;
    else if (!strcmp(argv[2], "grande")) c = UCHAR_MAX + 1;
    else if (!strcmp(argv[2], "extremo")) c = INT_MIN;
    else return 2;
    if (!strcmp(argv[1], "tolower")) (void)ULTRA_TOLOWER(c);
    else if (!strcmp(argv[1], "toupper")) (void)ULTRA_TOUPPER(c);
    else if (!strcmp(argv[1], "isalpha")) (void)ULTRA_ISALPHA(c);
    else return 2;
    fputs("MINA NO DETECTADA: llamada invalida aceptada\n", stderr);
    return 99;
}
