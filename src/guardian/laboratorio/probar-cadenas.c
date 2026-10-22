/* SPDX-License-Identifier: GPL-2.0-only */
#define _GNU_SOURCE
#include <signal.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/mman.h>
#include <unistd.h>
#include "../produccion/ultra_string.h"
#include "ultra_guard.h"

static size_t checks, accepted, rejected;

/* Independent snapshot oracle: fixed arrays, index loops, no memmove or API. */
static void matrix(bool append, size_t dst, size_t dc, size_t src, size_t sc,
                   size_t z1, size_t z2) {
    char data[48] = {0}, snapshot[48] = {0}, expected[48] = {0};
    for (size_t i = 0; i < sizeof(data); ++i) data[i] = (char)('a' + (int)(i % 26));
    if (z1 < sizeof(data)) data[z1] = 0;
    if (z2 < sizeof(data)) data[z2] = 0;
    memcpy(snapshot, data, sizeof(data));
    memcpy(expected, data, sizeof(data));
    enum ultra_string_status want = ULTRA_STRING_OK;
    size_t used = 0, length = 0;
    if (!dc) want = ULTRA_STRING_ARGUMENT;
    else {
        if (append) while (used < dc && snapshot[dst + used] != 0) ++used;
        if (append && used == dc) want = ULTRA_STRING_DESTINATION_NUL;
        else {
            while (length < sc && snapshot[src + length] != 0) ++length;
            if (length == sc) want = ULTRA_STRING_SOURCE_NUL;
            else if (length >= dc - used) want = ULTRA_STRING_CAPACITY;
            else for (size_t i = 0; i <= length; ++i) expected[dst + used + i] = snapshot[src + i];
        }
    }
    enum ultra_string_status got = append
        ? ultra_string_append(data + dst, dc, data + src, sc)
        : ultra_string_copy(data + dst, dc, data + src, sc);
    ULTRA_CHECK(got == want, "TEST-CADENA", "estado distinto del oraculo");
    ULTRA_CHECK(memcmp(data, expected, sizeof(data)) == 0,
                "TEST-CADENA", "bytes/canarios modificados contra el contrato");
    ++checks;
    if (got == ULTRA_STRING_OK) ++accepted; else ++rejected;
}

static char *guarded(size_t *page) {
    long value = sysconf(_SC_PAGESIZE);
    ULTRA_CHECK(value > 0 && (uintmax_t)value <= SIZE_MAX / 2,
                "TEST-CADENA", "tamano de pagina inesperado");
    *page = (size_t)value;
    char *p = mmap(NULL, *page * 2, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    ULTRA_CHECK(p != MAP_FAILED, "TEST-CADENA", "mmap fallo");
    ULTRA_CHECK(mprotect(p + *page, *page, PROT_NONE) == 0, "TEST-CADENA", "mprotect fallo");
    return p;
}

static void pages(bool legacy) {
    size_t page;
    char *map = guarded(&page), *source = map + page - 4;
    memcpy(source, "ABCD", 4);
    char dst[8] = "keep", before[8] = {0};
    memcpy(before, dst, sizeof(dst));
    if (legacy) {
        puts("LEGACY: origen sin NUL junto a PROT_NONE; se espera fallo");
        fflush(stdout);
        bool ok = ultra_copy(dst, sizeof(dst), source);
        printf("LEGACY-SOBREVIVIO: %d\n", (int)ok);
        return;
    }
    ULTRA_CHECK(ultra_string_copy(dst, sizeof(dst), source, 4) == ULTRA_STRING_SOURCE_NUL &&
                memcmp(dst, before, sizeof(dst)) == 0, "TEST-CADENA", "origen sin NUL no rechazado");
    source[3] = 0;
    ULTRA_CHECK(ultra_string_copy(dst, sizeof(dst), source, 4) == ULTRA_STRING_OK &&
                strcmp(dst, "ABC") == 0, "TEST-CADENA", "NUL en ultimo byte rechazado");
    memcpy(source, "keep", 4);
    ULTRA_CHECK(ultra_string_copy(source, 4, "ABCD", 5) == ULTRA_STRING_CAPACITY &&
                memcmp(source, "keep", 4) == 0, "TEST-CADENA", "destino corto modificado");
    ULTRA_CHECK(ultra_string_append(source, 4, "X", 2) == ULTRA_STRING_DESTINATION_NUL &&
                memcmp(source, "keep", 4) == 0, "TEST-CADENA", "destino sin NUL no rechazado");
    ULTRA_CHECK(ultra_string_copy(source, 4, "ABC", 4) == ULTRA_STRING_OK &&
                memcmp(source, "ABC", 4) == 0, "TEST-CADENA", "destino exacto rechazado");
    ULTRA_CHECK(munmap(map, page * 2) == 0, "TEST-CADENA", "munmap fallo");
    puts("PAGINAS-OK: 5 controles con frontera PROT_NONE real");
}

int main(int argc, char **argv) {
#ifndef __SANITIZE_ADDRESS__
    ultra_guard_install("probar-cadenas");
#endif
    if (argc == 2 && strcmp(argv[1], "--legacy-source") == 0) { pages(true); return 2; }
    ULTRA_CHECK(argc == 1, "TEST-CADENA", "argumentos inesperados");
    const size_t offsets[] = {0, 3, 9}, capacities[] = {0, 1, 4, 8}, zeros[] = {0, 3, 7, 12, 47, 48};
    for (size_t a = 0; a < ULTRA_LEN(offsets); ++a)
        for (size_t b = 0; b < ULTRA_LEN(offsets); ++b)
            for (size_t c = 0; c < ULTRA_LEN(capacities); ++c)
                for (size_t d = 0; d < ULTRA_LEN(capacities); ++d)
                    for (size_t e = 0; e < ULTRA_LEN(zeros); ++e)
                        for (size_t f = 0; f < ULTRA_LEN(zeros); ++f) {
                            matrix(false, offsets[a], capacities[c], offsets[b], capacities[d], zeros[e], zeros[f]);
                            matrix(true, offsets[a], capacities[c], offsets[b], capacities[d], zeros[e], zeros[f]);
                        }
    char dst[8] = "keep";
    ULTRA_CHECK(ultra_string_copy(NULL, 0, "", 1) == ULTRA_STRING_ARGUMENT &&
                ultra_string_copy(dst, sizeof(dst), NULL, 0) == ULTRA_STRING_ARGUMENT &&
                ultra_string_append(NULL, 0, "", 1) == ULTRA_STRING_ARGUMENT &&
                ultra_string_append(dst, sizeof(dst), NULL, 0) == ULTRA_STRING_ARGUMENT &&
                strcmp(dst, "keep") == 0, "TEST-CADENA", "contrato de argumentos nulos");
    pages(false);
    printf("CADENAS-OK: matriz=%zu validos=%zu rechazos=%zu; nulos=4; alias y canarios comprobados\n",
           checks, accepted, rejected);
    return 0;
}
