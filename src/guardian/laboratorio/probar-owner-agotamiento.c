/* SPDX-License-Identifier: GPL-2.0-only */
/* White-box test in a separate process; no reset hook in the public API. */
#include "../produccion/ultra_owner.c"
#include <stdio.h>
#define CHECK(x) do { if (!(x)) { fprintf(stderr, "FAIL line=%d\n", __LINE__); exit(1); } } while (0)
int main(void) {
    atomic_store_explicit(&last_pool_id, POOL_ID_MAX - 2, memory_order_relaxed);
    struct ultra_owner *a = ultra_owner_create(), *b = ultra_owner_create();
    CHECK(a && b && a->id != b->id);
    CHECK(ultra_owner_create() == NULL);
    ultra_handle ha = 0, hb = 0;
    CHECK(ultra_owner_alloc(a, 1, &ha) == UO_OK);
    CHECK(ultra_owner_alloc(b, 1, &hb) == UO_OK && ha != hb);
    unsigned char out = 0xa5;
    CHECK(ultra_owner_read(a, hb, 0, &out, 1) == UO_STALE && out == 0xa5);
    CHECK(ultra_owner_read(b, ha, 0, &out, 1) == UO_STALE && out == 0xa5);
    CHECK(ultra_owner_free(a, ha) == UO_OK);
    a->slots[0].generation = UINT32_MAX;
    CHECK(ultra_owner_alloc(a, 1, &ha) == UO_OK);
    CHECK(ultra_owner_free(a, ha) == UO_OK && a->slots[0].generation == 0);
    CHECK(ultra_owner_alloc(a, 1, &hb) == UO_OK && hb != ha);
    CHECK(ultra_owner_read(a, ha, 0, &out, 1) == UO_STALE);
    CHECK(ultra_owner_read(a, hb, 0, &out, 1) == UO_OK && out == 0);
    ultra_owner_destroy(a);
    CHECK(ultra_owner_create() == NULL); /* Destruction never recycles IDs. */
    CHECK(ultra_owner_read(b, ((uint64_t)1 << 32) | (b->id << 4), 0, &out, 1) == UO_OK);
    ultra_owner_destroy(b);
    puts("AGOTAMIENTO-OK: IDs sin reciclaje ni vuelta a cero; slot retirado; pools vivos utilizables");
    return 0;
}
