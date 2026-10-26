/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef ULTRA_EFI_ORDER_H
#define ULTRA_EFI_ORDER_H
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

/* BootOrder is a little-endian UINT16 array. Reject partial entries before
 * writing output; count is always cleared on failure. No firmware access. */
static inline bool ultra_efi_order_decode(const uint8_t *bytes, size_t size,
                                         uint16_t *order, size_t capacity, size_t *count) {
    if (!count) return false;
    *count = 0;
    if (size % 2 || size / 2 > capacity || (size && (!bytes || !order))) return false;
    if (!size) return true;
    size_t n = size / 2;
    for (size_t i = 0; i < n; i++)
        order[i] = (uint16_t)((uint16_t)bytes[i * 2] | ((uint16_t)bytes[i * 2 + 1] << 8));
    *count = n;
    return true;
}
#endif
