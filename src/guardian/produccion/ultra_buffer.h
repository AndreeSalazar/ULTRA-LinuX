/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef ULTRA_BUFFER_H
#define ULTRA_BUFFER_H
#include <stddef.h>
#include <stdint.h>

enum ultra_buffer_status {
    ULTRA_BUFFER_OK, ULTRA_BUFFER_ARGUMENT, ULTRA_BUFFER_OVERFLOW, ULTRA_BUFFER_CAPACITY
};
struct ultra_buffer_span { size_t begin, length, end; };

/* Byte-layout contract: reserve every full stride, including last-row padding.
 * Does not validate a pointer, mapping, file size, format or concurrent lifetime.
 * On failure, leave *out unchanged. The caller must provide a valid out object.
 */
static inline __attribute__((warn_unused_result)) enum ultra_buffer_status
ultra_buffer_layout(size_t offset, size_t stride, size_t rows, size_t row_bytes,
                    size_t capacity, struct ultra_buffer_span *out) {
    if (!out || !rows || !row_bytes || row_bytes > stride) return ULTRA_BUFFER_ARGUMENT;
    size_t length, end;
    if (__builtin_mul_overflow(stride, rows, &length) ||
        __builtin_add_overflow(offset, length, &end)) return ULTRA_BUFFER_OVERFLOW;
    if (end > capacity) return ULTRA_BUFFER_CAPACITY;
    *out = (struct ultra_buffer_span){offset, length, end};
    return ULTRA_BUFFER_OK;
}

/* Packed four-byte pixels only. Signed dimensions are checked BEFORE casts.
 * Not a Wayland wire/protocol validator and not suitable for planar formats.
 */
static inline __attribute__((warn_unused_result)) enum ultra_buffer_status
ultra_buffer32_layout(int32_t width, int32_t height, int32_t stride, int32_t offset,
                      size_t capacity, struct ultra_buffer_span *out) {
    if (width <= 0 || height <= 0 || stride <= 0 || offset < 0) return ULTRA_BUFFER_ARGUMENT;
    size_t row_bytes;
    if (__builtin_mul_overflow((size_t)width, (size_t)4, &row_bytes)) return ULTRA_BUFFER_OVERFLOW;
    return ultra_buffer_layout((size_t)offset, (size_t)stride, (size_t)height, row_bytes, capacity, out);
}
#endif
