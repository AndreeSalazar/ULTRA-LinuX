/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef ULTRA_CUSTODIA_H
#define ULTRA_CUSTODIA_H
#include <stddef.h>
#include <stdint.h>
/* Serialized local callers, live manager and real external spans required.
 * subject is trusted test/broker input, NOT authenticated client identity.
 * Tokens are process-local; no IPC/kernel authority or hostile-code isolation.
 * Read loans copy their input; no raw aliases escape. No writes, mmap or I/O. */
struct ultra_custodia;
typedef uint64_t uc_token;
enum uc_status { UC_OK, UC_ARGUMENT, UC_DENIED, UC_STALE, UC_RANGE,
                 UC_QUOTA, UC_RETIRING, UC_NOMEM, UC_BUSY };
#define UC_RESULT __attribute__((warn_unused_result))
/* Limits cover copied payload bytes and outstanding requests, not metadata.
 * At most 16 loans and 16 requests; requested max_requests must be 1..16. */
struct ultra_custodia *uc_create(size_t max_bytes, unsigned max_requests);
/* BUSY leaves manager alive. Otherwise destroys all loans; no pending requests. */
enum uc_status uc_destroy(struct ultra_custodia *c) UC_RESULT;
enum uc_status uc_lend(struct ultra_custodia *c, uint64_t subject,
    const void *src, size_t size, uc_token *out) UC_RESULT;
enum uc_status uc_begin(struct ultra_custodia *c, uint64_t subject,
    uc_token loan, size_t offset, size_t size, uc_token *ticket) UC_RESULT;
/* Revoke rejects new requests; accepted ones may complete or cancel.
 * Data/quota stay charged until the last accepted request retires. */
enum uc_status uc_revoke(struct ultra_custodia *c, uint64_t subject,
    uc_token loan) UC_RESULT;
/* RANGE/ARGUMENT failures keep the ticket live for retry/cancel, out unchanged.
 * Supplied capacity must correspond to a real live destination. */
enum uc_status uc_complete(struct ultra_custodia *c, uint64_t subject,
    uc_token ticket, void *out, size_t capacity) UC_RESULT;
enum uc_status uc_cancel(struct ultra_custodia *c, uint64_t subject,
    uc_token ticket) UC_RESULT;
#undef UC_RESULT
#endif
