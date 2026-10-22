/* SPDX-License-Identifier: GPL-2.0-only */
#include "ultra_owner.h"
#include <stdlib.h>
#include <string.h>
#define SLOTS 16u
struct slot { unsigned char *data; size_t size; uint32_t generation; };
struct ultra_owner { struct slot slots[SLOTS]; };
struct ultra_owner *ultra_owner_create(void) {
    struct ultra_owner *p = calloc(1, sizeof(*p));
    if (p) for (size_t i = 0; i < SLOTS; ++i) p->slots[i].generation = 1;
    return p;
}
void ultra_owner_destroy(struct ultra_owner *p) {
    if (!p) return;
    for (size_t i = 0; i < SLOTS; ++i) free(p->slots[i].data);
    free(p);
}
static struct slot *lookup(struct ultra_owner *p, ultra_handle h) {
    uint32_t index = (uint32_t)h;
    uint32_t generation = (uint32_t)(h >> 32);
    if (!p || index >= SLOTS || !generation) return NULL;
    struct slot *s = &p->slots[index];
    return s->data && s->generation == generation ? s : NULL;
}
enum ultra_owner_status ultra_owner_alloc(struct ultra_owner *p, size_t n, ultra_handle *out) {
    if (!p || !n || !out) return UO_ARGUMENT;
    for (uint32_t i = 0; i < SLOTS; ++i) {
        struct slot *s = &p->slots[i];
        if (s->data || !s->generation) continue;
        unsigned char *data = calloc(n, 1);
        if (!data) return UO_NOMEM;
        s->data = data; s->size = n;
        *out = ((uint64_t)s->generation << 32) | i;
        return UO_OK;
    }
    return UO_FULL;
}
enum ultra_owner_status ultra_owner_free(struct ultra_owner *p, ultra_handle h) {
    struct slot *s = lookup(p, h);
    if (!s) return UO_STALE;
    free(s->data); s->data = NULL; s->size = 0;
    /* Zero permanently retires the slot: never wrap to a previous generation. */
    s->generation = s->generation == UINT32_MAX ? 0 : s->generation + 1;
    return UO_OK;
}
enum ultra_owner_status ultra_owner_read(struct ultra_owner *p, ultra_handle h, size_t off, void *out, size_t n) {
    struct slot *s = lookup(p, h);
    if (!s) return UO_STALE;
    if (!out && n) return UO_ARGUMENT;
    if (off > s->size || n > s->size - off) return UO_RANGE;
    if (n) memcpy(out, s->data + off, n);
    return UO_OK;
}
enum ultra_owner_status ultra_owner_write(struct ultra_owner *p, ultra_handle h, size_t off, const void *src, size_t n) {
    struct slot *s = lookup(p, h);
    if (!s) return UO_STALE;
    if (!src && n) return UO_ARGUMENT;
    if (off > s->size || n > s->size - off) return UO_RANGE;
    if (n) memcpy(s->data + off, src, n);
    return UO_OK;
}
