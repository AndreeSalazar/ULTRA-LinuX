/* SPDX-License-Identifier: GPL-2.0-only */
#include "ultra_custodia.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static unsigned checks;
static int fail_after = -1;
void *__real_calloc(size_t n, size_t size);
void *__wrap_calloc(size_t n, size_t size);
void *__wrap_calloc(size_t n, size_t size) {
    if (fail_after == 0) return NULL;
    if (fail_after > 0) --fail_after;
    return __real_calloc(n, size);
}
#define CHECK(x) do { ++checks; if (!(x)) { fprintf(stderr, "FAIL line=%d\n", __LINE__); exit(1); } } while (0)
int main(void) {
    CHECK(uc_create(0, 1) == NULL);
    CHECK(uc_create(8, 0) == NULL);
    CHECK(uc_create(8, 17) == NULL);
    CHECK(uc_destroy(NULL) == UC_ARGUMENT);
    for (int i = 0; i < 3; ++i) {
        fail_after = i;
        CHECK(uc_create(8, 2) == NULL);
    }
    fail_after = -1;
    const unsigned char expected[8] = {1,2,3,4,5,6,7,8};
    for (unsigned cycle = 0; cycle < 1000; ++cycle) {
        struct ultra_custodia *c = uc_create(8, 2), *other = uc_create(8, 1);
        CHECK(c && other);
        uc_token h = 123, foreign = 0, a = 123, b = 0, fresh = 0;
        unsigned char source[8], out[8], tiny[3] = {0xa5,0xa5,0xa5};
        memcpy(source, expected, 8);
        CHECK(uc_lend(c, 0, source, 8, &h) == UC_ARGUMENT && h == 123);
        fail_after = 0;
        CHECK(uc_lend(c, 7, source, 8, &h) == UC_NOMEM && h == 123);
        fail_after = -1;
        CHECK(uc_lend(c, 7, source, 8, &h) == UC_OK);
        CHECK(uc_lend(other, 7, source, 8, &foreign) == UC_OK && foreign != h);
        memset(source, 0, 8); /* The resource owns a snapshot, not caller memory. */
        CHECK(uc_lend(c, 7, source, 1, &fresh) == UC_QUOTA && fresh == 0);
        CHECK(uc_lend(c, 7, source, SIZE_MAX, &fresh) == UC_QUOTA && fresh == 0);
        CHECK(uc_begin(c, 8, h, 0, 8, &a) == UC_DENIED && a == 123);
        CHECK(uc_begin(c, 7, foreign, 0, 8, &a) == UC_STALE && a == 123);
        CHECK(uc_begin(c, 7, h, SIZE_MAX, 1, &a) == UC_RANGE && a == 123);
        CHECK(uc_begin(c, 7, h, 0, SIZE_MAX, &a) == UC_RANGE && a == 123);
        fail_after = 0;
        CHECK(uc_begin(c, 7, h, 0, 8, &a) == UC_NOMEM && a == 123);
        fail_after = -1;
        CHECK(uc_begin(c, 7, h, 0, 8, &a) == UC_OK);
        CHECK(uc_begin(c, 7, h, 8, 0, &b) == UC_OK);
        CHECK(uc_begin(c, 7, h, 0, 1, &fresh) == UC_QUOTA && fresh == 0);
        CHECK(uc_destroy(c) == UC_BUSY);
        CHECK(uc_revoke(c, 8, h) == UC_DENIED);
        CHECK(uc_revoke(c, 7, h) == UC_OK);
        CHECK(uc_revoke(c, 7, h) == UC_OK); /* Still retiring, two requests alive. */
        CHECK(uc_begin(c, 7, h, 0, 1, &fresh) == UC_RETIRING && fresh == 0);
        CHECK(uc_lend(c, 7, source, 8, &fresh) == UC_QUOTA);
        memset(out, 0xa5, 8);
        CHECK(uc_complete(c, 8, a, out, 8) == UC_DENIED);
        CHECK(uc_complete(other, 7, a, out, 8) == UC_STALE);
        CHECK(uc_complete(c, 7, a, tiny, 3) == UC_RANGE);
        CHECK(tiny[0] == 0xa5 && tiny[1] == 0xa5 && tiny[2] == 0xa5);
        CHECK(uc_complete(c, 7, a, NULL, 8) == UC_ARGUMENT);
        for (size_t i = 0; i < 8; ++i) CHECK(out[i] == 0xa5);
        CHECK(uc_complete(c, 7, a, out, 8) == UC_OK && memcmp(out, expected, 8) == 0);
        CHECK(uc_complete(c, 7, a, out, 8) == UC_STALE);
        CHECK(uc_cancel(c, 7, a) == UC_STALE);
        CHECK(uc_lend(c, 7, source, 8, &fresh) == UC_QUOTA); /* b still owns a reference. */
        CHECK(uc_cancel(c, 8, b) == UC_DENIED);
        CHECK(uc_cancel(c, 7, b) == UC_OK);
        CHECK(uc_cancel(c, 7, b) == UC_STALE);
        CHECK(uc_revoke(c, 7, h) == UC_STALE);
        CHECK(uc_lend(c, 7, expected, 8, &fresh) == UC_OK && fresh != h);
        CHECK(uc_begin(c, 7, h, 0, 1, &a) == UC_STALE);
        CHECK(uc_begin(c, 7, fresh, 8, 0, &a) == UC_OK);
        CHECK(uc_complete(c, 7, a, NULL, 0) == UC_OK);
        CHECK(uc_revoke(c, 7, fresh) == UC_OK);
        CHECK(uc_destroy(c) == UC_OK);
        CHECK(uc_destroy(other) == UC_OK);
    }
    struct ultra_custodia *c = uc_create(64, 16);
    CHECK(c != NULL);
    uc_token loans[16], tickets[16], extra = 123;
    for (unsigned i = 0; i < 16; ++i) CHECK(uc_lend(c, 7, expected, 1, &loans[i]) == UC_OK);
    CHECK(uc_lend(c, 7, expected, 1, &extra) == UC_QUOTA && extra == 123);
    for (unsigned i = 0; i < 16; ++i) CHECK(uc_begin(c, 7, loans[0], 0, 1, &tickets[i]) == UC_OK);
    CHECK(uc_begin(c, 7, loans[0], 0, 1, &extra) == UC_QUOTA && extra == 123);
    for (unsigned i = 0; i < 16; ++i) CHECK(uc_revoke(c, 7, loans[i]) == UC_OK);
    CHECK(uc_destroy(c) == UC_BUSY);
    for (unsigned i = 0; i < 16; ++i) CHECK(uc_cancel(c, 7, tickets[i]) == UC_OK);
    CHECK(uc_destroy(c) == UC_OK);
    printf("CUSTODIA-OK: comprobaciones=%u ciclos=1000; copia, titular, cuotas, retirada, drenaje, replay y fallos de asignacion\n", checks);
    return 0;
}
