/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef ULTRA_STRING_H
#define ULTRA_STRING_H
#include <stddef.h>
#include <string.h>

enum ultra_string_status {
    ULTRA_STRING_OK, ULTRA_STRING_ARGUMENT, ULTRA_STRING_SOURCE_NUL,
    ULTRA_STRING_DESTINATION_NUL, ULTRA_STRING_CAPACITY
};

/* Contract: non-NULL live spans of dst_capacity writable bytes and src_capacity
 * readable bytes, no concurrent mutation. Extents MUST describe real objects;
 * these functions cannot validate arbitrary pointers, ownership or mappings.
 * No truncation. Every rejection leaves all destination bytes unchanged.
 * Overlap, including self-copy/append, is supported with snapshot semantics.
 * Zero destination capacity/NULL pointers: ARGUMENT. Zero source capacity with
 * non-NULL pointers: SOURCE_NUL. No byte outside the supplied spans is accessed.
 */
static inline size_t ultra_string_length(const char *text, size_t capacity) {
    size_t n = 0;
    while (n < capacity && text[n] != '\0') ++n;
    return n;
}

static inline __attribute__((warn_unused_result)) enum ultra_string_status
ultra_string_copy(char *dst, size_t dst_capacity, const char *src, size_t src_capacity) {
    if (!dst || !src || !dst_capacity) return ULTRA_STRING_ARGUMENT;
    size_t length = ultra_string_length(src, src_capacity);
    if (length == src_capacity) return ULTRA_STRING_SOURCE_NUL;
    if (length >= dst_capacity) return ULTRA_STRING_CAPACITY;
    /* length < src_capacity <= SIZE_MAX, so length+1 cannot wrap. */
    memmove(dst, src, length + 1);
    return ULTRA_STRING_OK;
}

static inline __attribute__((warn_unused_result)) enum ultra_string_status
ultra_string_append(char *dst, size_t dst_capacity, const char *src, size_t src_capacity) {
    if (!dst || !src || !dst_capacity) return ULTRA_STRING_ARGUMENT;
    size_t used = ultra_string_length(dst, dst_capacity);
    if (used == dst_capacity) return ULTRA_STRING_DESTINATION_NUL;
    size_t length = ultra_string_length(src, src_capacity);
    if (length == src_capacity) return ULTRA_STRING_SOURCE_NUL;
    if (length >= dst_capacity - used) return ULTRA_STRING_CAPACITY;
    memmove(dst + used, src, length + 1);
    return ULTRA_STRING_OK;
}
#endif
