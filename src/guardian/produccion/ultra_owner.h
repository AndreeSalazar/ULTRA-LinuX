/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef ULTRA_OWNER_H
#define ULTRA_OWNER_H
#include <stddef.h>
#include <stdint.h>
/* Single-threaded pool. Pool must remain alive; do not mix handles between pools.
 * No raw aliases escape. External buffers must be real, live spans. */
struct ultra_owner;
typedef uint64_t ultra_handle;
enum ultra_owner_status { UO_OK, UO_ARGUMENT, UO_STALE, UO_RANGE, UO_FULL, UO_NOMEM };
struct ultra_owner *ultra_owner_create(void);
void ultra_owner_destroy(struct ultra_owner *pool);
enum ultra_owner_status ultra_owner_alloc(struct ultra_owner *pool, size_t size, ultra_handle *out) __attribute__((warn_unused_result));
enum ultra_owner_status ultra_owner_free(struct ultra_owner *pool, ultra_handle handle) __attribute__((warn_unused_result));
enum ultra_owner_status ultra_owner_read(struct ultra_owner *pool, ultra_handle handle, size_t offset, void *out, size_t size) __attribute__((warn_unused_result));
enum ultra_owner_status ultra_owner_write(struct ultra_owner *pool, ultra_handle handle, size_t offset, const void *src, size_t size) __attribute__((warn_unused_result));
#endif
