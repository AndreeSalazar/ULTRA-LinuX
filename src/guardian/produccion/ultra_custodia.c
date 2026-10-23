/* SPDX-License-Identifier: GPL-2.0-only */
#include "ultra_custodia.h"
#include "ultra_owner.h"
#include <stdbool.h>
#include <stdlib.h>
#define LIMIT 16u
struct loan { uc_token token; uint64_t subject; size_t size; unsigned pending; bool retiring; };
struct request { uc_token token; uint64_t subject; unsigned loan_index; size_t offset, size; };
struct ultra_custodia {
    struct ultra_owner *payloads, *tickets;
    struct loan loans[LIMIT];
    struct request requests[LIMIT];
    size_t max_bytes, used_bytes;
    unsigned max_requests, pending;
};
/* An internal invariant failure cannot be repaired by ignoring owner errors. */
static void owner_ok(enum ultra_owner_status status) { if (status != UO_OK) abort(); }
struct ultra_custodia *uc_create(size_t max_bytes, unsigned max_requests) {
    if (!max_bytes || !max_requests || max_requests > LIMIT) return NULL;
    struct ultra_custodia *c = calloc(1, sizeof(*c));
    if (!c) return NULL;
    c->payloads = ultra_owner_create(); c->tickets = ultra_owner_create();
    if (!c->payloads || !c->tickets) {
        ultra_owner_destroy(c->payloads); ultra_owner_destroy(c->tickets); free(c); return NULL;
    }
    c->max_bytes = max_bytes; c->max_requests = max_requests;
    return c;
}
enum uc_status uc_destroy(struct ultra_custodia *c) {
    if (!c) return UC_ARGUMENT;
    if (c->pending) return UC_BUSY;
    ultra_owner_destroy(c->payloads); ultra_owner_destroy(c->tickets); free(c);
    return UC_OK;
}
enum uc_status uc_lend(struct ultra_custodia *c, uint64_t subject,
                       const void *src, size_t size, uc_token *out) {
    if (!c || !subject || !src || !size || !out) return UC_ARGUMENT;
    if (size > c->max_bytes - c->used_bytes) return UC_QUOTA;
    unsigned i;
    for (i = 0; i < LIMIT; ++i) if (!c->loans[i].token) break;
    if (i == LIMIT) return UC_QUOTA;
    uc_token token = 0;
    enum ultra_owner_status status = ultra_owner_alloc(c->payloads, size, &token);
    if (status == UO_NOMEM) return UC_NOMEM;
    if (status == UO_FULL) return UC_QUOTA;
    owner_ok(status);
    owner_ok(ultra_owner_write_bounded(c->payloads, token, 0, src, size, size));
    c->loans[i] = (struct loan){token, subject, size, 0, false};
    c->used_bytes += size; *out = token;
    return UC_OK;
}
static struct loan *find_loan(struct ultra_custodia *c, uc_token token, unsigned *index) {
    if (!token) return NULL;
    for (unsigned i = 0; i < LIMIT; ++i) if (c->loans[i].token == token) {
        *index = i; return &c->loans[i];
    }
    return NULL;
}
static struct request *find_request(struct ultra_custodia *c, uc_token token) {
    if (!token) return NULL;
    for (unsigned i = 0; i < LIMIT; ++i) if (c->requests[i].token == token) return &c->requests[i];
    return NULL;
}
static void reap(struct ultra_custodia *c, struct loan *loan) {
    if (!loan->retiring || loan->pending) return;
    owner_ok(ultra_owner_free(c->payloads, loan->token));
    c->used_bytes -= loan->size; *loan = (struct loan){0};
}
enum uc_status uc_begin(struct ultra_custodia *c, uint64_t subject,
                        uc_token token, size_t offset, size_t size, uc_token *ticket) {
    if (!c || !subject || !ticket) return UC_ARGUMENT;
    unsigned index = 0;
    struct loan *loan = find_loan(c, token, &index);
    if (!loan) return UC_STALE;
    if (loan->subject != subject) return UC_DENIED;
    if (loan->retiring) return UC_RETIRING;
    if (offset > loan->size || size > loan->size - offset) return UC_RANGE;
    if (c->pending >= c->max_requests) return UC_QUOTA;
    unsigned i;
    for (i = 0; i < LIMIT; ++i) if (!c->requests[i].token) break;
    if (i == LIMIT) return UC_QUOTA;
    uc_token id = 0;
    enum ultra_owner_status status = ultra_owner_alloc(c->tickets, 1, &id);
    if (status == UO_NOMEM) return UC_NOMEM;
    if (status == UO_FULL) return UC_QUOTA;
    owner_ok(status);
    c->requests[i] = (struct request){id, subject, index, offset, size};
    ++loan->pending; ++c->pending; *ticket = id;
    return UC_OK;
}
enum uc_status uc_revoke(struct ultra_custodia *c, uint64_t subject, uc_token token) {
    if (!c || !subject) return UC_ARGUMENT;
    unsigned index = 0;
    struct loan *loan = find_loan(c, token, &index);
    if (!loan) return UC_STALE;
    if (loan->subject != subject) return UC_DENIED;
    loan->retiring = true; reap(c, loan);
    return UC_OK;
}
static void retire_request(struct ultra_custodia *c, struct request *r) {
    struct loan *loan = &c->loans[r->loan_index];
    owner_ok(ultra_owner_free(c->tickets, r->token));
    --loan->pending; --c->pending; *r = (struct request){0};
    reap(c, loan);
}
enum uc_status uc_complete(struct ultra_custodia *c, uint64_t subject,
                           uc_token token, void *out, size_t capacity) {
    if (!c || !subject) return UC_ARGUMENT;
    struct request *r = find_request(c, token);
    if (!r) return UC_STALE;
    if (r->subject != subject) return UC_DENIED;
    if (r->size > capacity) return UC_RANGE;
    if (!out && r->size) return UC_ARGUMENT;
    struct loan *loan = &c->loans[r->loan_index];
    owner_ok(ultra_owner_read_bounded(c->payloads, loan->token, r->offset, out, capacity, r->size));
    retire_request(c, r);
    return UC_OK;
}
enum uc_status uc_cancel(struct ultra_custodia *c, uint64_t subject, uc_token token) {
    if (!c || !subject) return UC_ARGUMENT;
    struct request *r = find_request(c, token);
    if (!r) return UC_STALE;
    if (r->subject != subject) return UC_DENIED;
    retire_request(c, r);
    return UC_OK;
}
