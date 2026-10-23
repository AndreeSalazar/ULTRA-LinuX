/* SPDX-License-Identifier: GPL-2.0-only */
/* Deliberate contract violations. Run only in the isolated diagnostic runner. */
#include "../produccion/ultra_owner.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(int argc, char **argv) {
    if (argc != 2) return 2;
    struct ultra_owner *a = ultra_owner_create(), *b = ultra_owner_create();
    assert(a && b);
    ultra_handle ha = 0, hb = 0;
    assert(ultra_owner_alloc(a, 8, &ha) == UO_OK);
    assert(ultra_owner_alloc(b, 8, &hb) == UO_OK);
    const unsigned char data[8] = {1,2,3,4,5,6,7,8};
    assert(ultra_owner_write_bounded(a, ha, 0, data, sizeof(data), 8) == UO_OK);
    if (strcmp(argv[1], "cruce-pool") == 0) {
        unsigned char out[8] = {0};
        assert(ha == hb);
        assert(ultra_owner_read_bounded(b, ha, 0, out, sizeof(out), 8) == UO_OK);
        assert(out[0] == 0); /* Reads b, not a: identity of pool is absent. */
        puts("LIMITACION_CONFIRMADA: handle ajeno coincide; falta identidad del pool");
    } else if (strcmp(argv[1], "externo-corto") == 0) {
        unsigned char tiny[3] = {0};
        assert(ultra_owner_read(a, ha, 0, tiny, 8) == UO_OK);
    } else if (strcmp(argv[1], "externo-caducado") == 0) {
        unsigned char *dead = malloc(8);
        assert(dead);
        free(dead);
        assert(ultra_owner_write_bounded(a, ha, 0, dead, 8, 8) == UO_OK);
    } else if (strcmp(argv[1], "pool-caducado") == 0) {
        unsigned char out[8] = {0};
        ultra_owner_destroy(a);
        assert(ultra_owner_read_bounded(a, ha, 0, out, sizeof(out), 8) == UO_OK);
    } else return 2;
    ultra_owner_destroy(b);
    ultra_owner_destroy(a);
    return 0;
}
