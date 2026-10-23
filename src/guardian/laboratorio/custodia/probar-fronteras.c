/* SPDX-License-Identifier: GPL-2.0-only */
#define _GNU_SOURCE
#include "ultra_custodia.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <unistd.h>
static unsigned checks;
#define CHECK(x) do { ++checks; if (!(x)) { fprintf(stderr, "FAIL line=%d\n", __LINE__); exit(1); } } while (0)
int main(void) {
    long page_value = sysconf(_SC_PAGESIZE);
    CHECK(page_value >= 64 && (unsigned long)page_value <= SIZE_MAX / 3);
    size_t page = (size_t)page_value;
    unsigned char *source = mmap(NULL, page * 3, PROT_NONE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    unsigned char *target = mmap(NULL, page * 3, PROT_NONE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    CHECK(source != MAP_FAILED && target != MAP_FAILED);
    CHECK(mprotect(source+page, page, PROT_READ|PROT_WRITE) == 0);
    CHECK(mprotect(target+page, page, PROT_READ|PROT_WRITE) == 0);
    for (size_t size = 1; size <= 64; ++size) {
        struct ultra_custodia *c = uc_create(size, 2);
        CHECK(c != NULL);
        unsigned char *src = source + page*2 - size;
        for (size_t i = 0; i < size; ++i) src[i] = (unsigned char)(i+1);
        uc_token loan = 0, ticket = 0, sentinel = 123;
        CHECK(uc_lend(c, 7, src, size, &loan) == UC_OK);
        CHECK(mprotect(source+page, page, PROT_NONE) == 0);
        for (size_t offset = 0; offset <= size; ++offset) {
            size_t n = size-offset;
            CHECK(uc_begin(c, 7, loan, offset, n, &ticket) == UC_OK);
            if (n) {
                unsigned char *tiny = target+page*2-(n-1);
                memset(target+page, 0xa5, page);
                CHECK(uc_complete(c, 7, ticket, tiny, n-1) == UC_RANGE);
                for (size_t i = 0; i < page; ++i) CHECK(target[page+i] == 0xa5);
            }
            unsigned char *out = n ? target+page*2-n : NULL;
            CHECK(uc_complete(c, 7, ticket, out, n) == UC_OK);
            for (size_t i = 0; i < n; ++i) CHECK(out[i] == (unsigned char)(offset+i+1));
            CHECK(uc_begin(c, 7, loan, offset, n+1, &sentinel) == UC_RANGE && sentinel == 123);
        }
        CHECK(uc_begin(c, 7, loan, 0, size, &ticket) == UC_OK);
        CHECK(uc_revoke(c, 7, loan) == UC_OK);
        CHECK(uc_begin(c, 7, loan, 0, 1, &sentinel) == UC_RETIRING && sentinel == 123);
        CHECK(uc_cancel(c, 7, ticket) == UC_OK);
        CHECK(uc_destroy(c) == UC_OK);
        CHECK(mprotect(source+page, page, PROT_READ|PROT_WRITE) == 0);
    }
    CHECK(munmap(source, page*3) == 0 && munmap(target, page*3) == 0);
    printf("FRONTERAS-OK: tamanos=1..64, todos offsets, guard pages, origen inaccesible tras copia; comprobaciones=%u\n", checks);
    return 0;
}
