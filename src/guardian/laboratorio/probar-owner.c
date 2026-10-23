/* SPDX-License-Identifier: GPL-2.0-only */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../produccion/ultra_owner.h"
static int fail_alloc;
void *__real_calloc(size_t n, size_t size);
void *__wrap_calloc(size_t n, size_t size);
void *__wrap_calloc(size_t n, size_t size) {
    return fail_alloc ? NULL : __real_calloc(n, size);
}
static unsigned checks;
#define CHECK(x) do { ++checks; if (!(x)) { fprintf(stderr, "FAIL line=%d\n", __LINE__); exit(1); } } while (0)
int main(void) {
    fail_alloc = 1;
    CHECK(ultra_owner_create() == NULL);
    fail_alloc = 0;
    struct ultra_owner *p = ultra_owner_create();
    CHECK(p != NULL);
    ultra_handle h = 0, old = 0;
    unsigned char data[8] = {1,2,3,4,5,6,7,8}, out[8] = {0}, expected[8] = {0};
    CHECK(ultra_owner_alloc(p, 0, &h) == UO_ARGUMENT && h == 0);
    CHECK(ultra_owner_alloc(NULL, 8, &h) == UO_ARGUMENT);
    fail_alloc = 1;
    CHECK(ultra_owner_alloc(p, 8, &h) == UO_NOMEM && h == 0);
    fail_alloc = 0;
    for (unsigned cycle = 0; cycle < 1000; ++cycle) {
        CHECK(ultra_owner_alloc(p, 8, &h) == UO_OK);
        CHECK(h != old);
        CHECK(ultra_owner_read(p, old, 0, out, 8) == UO_STALE);
        CHECK(ultra_owner_write(p, h, 0, data, 8) == UO_OK);
        unsigned char tiny[3] = {0xa5, 0xa5, 0xa5};
        CHECK(ultra_owner_read_bounded(p, h, 0, tiny, sizeof(tiny), 8) == UO_RANGE);
        CHECK(tiny[0] == 0xa5 && tiny[1] == 0xa5 && tiny[2] == 0xa5);
        CHECK(ultra_owner_write_bounded(p, h, 0, tiny, sizeof(tiny), 8) == UO_RANGE);
        CHECK(ultra_owner_read(p, h, 0, out, 8) == UO_OK && memcmp(out, data, 8) == 0);
        CHECK(ultra_owner_read_bounded(p, h, SIZE_MAX, out, sizeof(out), 1) == UO_RANGE);
        CHECK(ultra_owner_read_bounded(p, h, 0, out, sizeof(out), SIZE_MAX) == UO_RANGE);
        CHECK(ultra_owner_read_bounded(p, h, 8, NULL, 0, 0) == UO_OK);
        CHECK(ultra_owner_write_bounded(p, h, 8, NULL, 0, 0) == UO_OK);
        CHECK(ultra_owner_read_bounded(p, h, 0, NULL, 1, 1) == UO_ARGUMENT);
        CHECK(ultra_owner_write_bounded(p, h, 0, data, sizeof(data), 8) == UO_OK);
        CHECK(ultra_owner_read_bounded(p, h, 0, out, sizeof(out), 8) == UO_OK && memcmp(out, data, 8) == 0);
        for (size_t off = 0; off <= 9; ++off) for (size_t n = 0; n <= 9; ++n) {
            memset(out, 0xa5, sizeof(out));
            enum ultra_owner_status want = off <= 8 && n <= 8 - off ? UO_OK : UO_RANGE;
            /* Never supply a too-short external span, even for rejection cases. */
            if (n > sizeof(out)) continue;
            memset(expected, 0xa5, sizeof(expected));
            if (want == UO_OK && n) memcpy(expected, data + off, n);
            CHECK(ultra_owner_read(p, h, off, out, n) == want);
            CHECK(memcmp(out, expected, sizeof(out)) == 0);
            CHECK(ultra_owner_write(p, h, 0, data, 8) == UO_OK);
            memcpy(expected, data, sizeof(expected));
            if (want == UO_OK && n) memcpy(expected + off, data, n);
            CHECK(ultra_owner_write(p, h, off, data, n) == want);
            CHECK(ultra_owner_read(p, h, 0, out, 8) == UO_OK);
            CHECK(memcmp(out, expected, sizeof(out)) == 0);
            CHECK(ultra_owner_write(p, h, 0, data, 8) == UO_OK);
        }
        CHECK(ultra_owner_write(p, h, SIZE_MAX, data, 1) == UO_RANGE);
        CHECK(ultra_owner_write(p, h, 0, data, SIZE_MAX) == UO_RANGE);
        CHECK(ultra_owner_read(p, h, 0, NULL, 1) == UO_ARGUMENT);
        CHECK(ultra_owner_write(p, h, 0, NULL, 1) == UO_ARGUMENT);
        CHECK(ultra_owner_read(p, h, 0, out, 8) == UO_OK && memcmp(out, data, 8) == 0);
        CHECK(ultra_owner_free(p, h) == UO_OK);
        memset(out, 0xa5, sizeof(out));
        CHECK(ultra_owner_read(p, h, 0, out, 8) == UO_STALE && out[0] == 0xa5);
        CHECK(ultra_owner_write(p, h, 0, data, 8) == UO_STALE);
        CHECK(ultra_owner_read_bounded(p, h, 0, out, sizeof(out), 8) == UO_STALE);
        CHECK(ultra_owner_free(p, h) == UO_STALE);
        CHECK(ultra_owner_read(p, old, 0, out, 8) == UO_STALE);
        old = h;
    }
    ultra_handle handles[16] = {0};
    for (size_t i = 0; i < 16; ++i) CHECK(ultra_owner_alloc(p, 8, &handles[i]) == UO_OK);
    h = 123;
    CHECK(ultra_owner_alloc(p, 8, &h) == UO_FULL && h == 123);
    CHECK(ultra_owner_free(p, UINT64_MAX) == UO_STALE);
    ultra_owner_destroy(p); /* Also releases still-live objects. */
    puts("OWNER-OK: 1000 ciclos, matriz de limites, handles caducados, doble liberacion, agotamiento y fallo de asignacion");
    printf("COMPROBACIONES: %u\n", checks);
    return 0;
}
