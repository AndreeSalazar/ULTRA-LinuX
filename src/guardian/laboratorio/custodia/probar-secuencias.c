/* SPDX-License-Identifier: GPL-2.0-only */
/* Black-box state-machine checks: independent ledger, deterministic seeds. */
#include "ultra_custodia.h"
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#define SLOTS 16u
struct model_loan { uc_token id; uint64_t subject; size_t size; bool retiring; unsigned char data[64]; };
struct model_request { uc_token id; uint64_t subject; unsigned loan; size_t offset, size; };
struct model { struct ultra_custodia *c; size_t max_bytes; unsigned max_requests;
    struct model_loan loans[SLOTS]; struct model_request requests[SLOTS]; };
static uint64_t rng;
static unsigned seed_now, step_now, op_now, checks;
static unsigned successes[10];
static bool fail_alloc;
void *__real_calloc(size_t n, size_t size);
void *__wrap_calloc(size_t n, size_t size);
void *__wrap_calloc(size_t n, size_t size) { return fail_alloc ? NULL : __real_calloc(n, size); }
#define CHECK(x) do { ++checks; if (!(x)) { fprintf(stderr, "FAIL seed=%u step=%u op=%u line=%d\n", seed_now, step_now, op_now, __LINE__); exit(1); } } while (0)
static uint64_t random_value(void) { rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17; return rng; }
static unsigned choose(unsigned limit) { return (unsigned)(random_value() % limit); }
static size_t used(const struct model *m) {
    size_t total = 0;
    for (unsigned i = 0; i < SLOTS; ++i) if (m->loans[i].id) total += m->loans[i].size;
    return total;
}
static unsigned pending(const struct model *m, int loan) {
    unsigned total = 0;
    for (unsigned i = 0; i < SLOTS; ++i)
        if (m->requests[i].id && (loan < 0 || m->requests[i].loan == (unsigned)loan)) ++total;
    return total;
}
static void reap_model(struct model *m) {
    for (unsigned i = 0; i < SLOTS; ++i)
        if (m->loans[i].id && m->loans[i].retiring && !pending(m, (int)i)) m->loans[i].id = 0;
}
int main(void) {
    uc_token history[128] = {0}; unsigned history_pos = 0;
    for (seed_now = 1; seed_now <= 32; ++seed_now) {
        rng = seed_now;
        struct model models[2] = {0};
        for (unsigned j = 0; j < 2; ++j) {
            models[j].max_bytes = 1u + choose(128);
            models[j].max_requests = 1u + choose(16);
            models[j].c = uc_create(models[j].max_bytes, models[j].max_requests);
            CHECK(models[j].c != NULL);
        }
        for (step_now = 0; step_now < 6000; ++step_now) {
            struct model *m = &models[choose(2)];
            uint64_t subject = choose(5);
            bool null_manager = choose(100) == 0;
            struct ultra_custodia *c = null_manager ? NULL : m->c;
            op_now = choose(10);
            uc_token token;
            switch (choose(4)) {
                case 0:
                    token = 0;
                    for (unsigned i = 0, start = choose(SLOTS); i < SLOTS; ++i)
                        if (m->loans[(start+i) % SLOTS].id) { token = m->loans[(start+i) % SLOTS].id; break; }
                    break;
                case 1:
                    token = 0;
                    for (unsigned i = 0, start = choose(SLOTS); i < SLOTS; ++i)
                        if (m->requests[(start+i) % SLOTS].id) { token = m->requests[(start+i) % SLOTS].id; break; }
                    break;
                case 2: token = history[choose(128)]; break;
                default: token = random_value(); break;
            }
            /* Balance invalid tokens with useful stateful chains. */
            if (choose(2) == 0) {
                unsigned start = choose(SLOTS);
                for (unsigned i = 0; i < SLOTS; ++i) {
                    unsigned slot = (start+i) % SLOTS;
                    uc_token candidate = op_now == 7 || op_now == 8 ? m->requests[slot].id : m->loans[slot].id;
                    if (candidate) { token = candidate; break; }
                }
            }
            int li = -1, ri = -1; unsigned free_l = SLOTS, free_r = SLOTS;
            for (unsigned i = 0; i < SLOTS; ++i) {
                if (token && m->loans[i].id == token) li = (int)i;
                if (token && m->requests[i].id == token) ri = (int)i;
                if (!m->loans[i].id && free_l == SLOTS) free_l = i;
                if (!m->requests[i].id && free_r == SLOTS) free_r = i;
            }
            if (choose(4) != 0) {
                if (li >= 0) subject = m->loans[li].subject;
                else if (ri >= 0) subject = m->requests[ri].subject;
            }
            enum uc_status want = UC_OK, got;
            uc_token result = UINT64_MAX;
            bool null_output = choose(20) == 0;
            fail_alloc = choose(5) == 0;
            if (op_now < 3) {
                unsigned char data[64];
                for (unsigned i = 0; i < 64; ++i) data[i] = (unsigned char)choose(256);
                size_t n = choose(65);
                bool null_source = choose(20) == 0;
                if (!c || !subject || null_output || null_source || !n) want = UC_ARGUMENT;
                else if (n + used(m) > m->max_bytes || free_l == SLOTS) want = UC_QUOTA;
                else if (fail_alloc) want = UC_NOMEM;
                got = uc_lend(c, subject, null_source ? NULL : data, n, null_output ? NULL : &result);
                CHECK(got == want);
                if (got == UC_OK) {
                    CHECK(free_l < SLOTS && result && result != UINT64_MAX);
                    m->loans[free_l] = (struct model_loan){.id=result, .subject=subject, .size=n};
                    memcpy(m->loans[free_l].data, data, n);
                    memset(data, 0, sizeof(data));
                } else CHECK(result == UINT64_MAX);
            } else if (op_now < 6) {
                size_t offset = choose(80), n = choose(80);
                if (li >= 0 && choose(2) == 0) {
                    offset = choose((unsigned)m->loans[li].size + 1);
                    n = choose((unsigned)(m->loans[li].size-offset) + 1);
                }
                if (choose(12) == 0) offset = SIZE_MAX;
                if (choose(12) == 0) n = SIZE_MAX;
                if (!c || !subject || null_output) want = UC_ARGUMENT;
                else if (li < 0) want = UC_STALE;
                else if (m->loans[li].subject != subject) want = UC_DENIED;
                else if (m->loans[li].retiring) want = UC_RETIRING;
                else if (offset > m->loans[li].size || n > m->loans[li].size - offset) want = UC_RANGE;
                else if (pending(m, -1) >= m->max_requests || free_r == SLOTS) want = UC_QUOTA;
                else if (fail_alloc) want = UC_NOMEM;
                got = uc_begin(c, subject, token, offset, n, null_output ? NULL : &result);
                CHECK(got == want);
                if (got == UC_OK) {
                    CHECK(li >= 0 && free_r < SLOTS && result && result != UINT64_MAX);
                    m->requests[free_r] = (struct model_request){result, subject, (unsigned)li, offset, n};
                } else CHECK(result == UINT64_MAX);
            } else if (op_now == 6) {
                if (!c || !subject) want = UC_ARGUMENT;
                else if (li < 0) want = UC_STALE;
                else if (m->loans[li].subject != subject) want = UC_DENIED;
                got = uc_revoke(c, subject, token); CHECK(got == want);
                if (got == UC_OK) m->loans[li].retiring = true;
            } else if (op_now == 7) {
                unsigned char out[80], expected[80]; memset(out, 0xa5, sizeof(out)); memcpy(expected, out, sizeof(out));
                size_t capacity = choose(65);
                if (!c || !subject) want = UC_ARGUMENT;
                else if (ri < 0) want = UC_STALE;
                else if (m->requests[ri].subject != subject) want = UC_DENIED;
                else if (m->requests[ri].size > capacity) want = UC_RANGE;
                else if (null_output && m->requests[ri].size) want = UC_ARGUMENT;
                if (want == UC_OK && m->requests[ri].size)
                    memcpy(expected + 8, m->loans[m->requests[ri].loan].data + m->requests[ri].offset, m->requests[ri].size);
                got = uc_complete(c, subject, token, null_output ? NULL : out + 8, capacity);
                CHECK(got == want); CHECK(memcmp(out, expected, sizeof(out)) == 0);
                if (got == UC_OK) m->requests[ri].id = 0;
            } else if (op_now == 8) {
                if (!c || !subject) want = UC_ARGUMENT;
                else if (ri < 0) want = UC_STALE;
                else if (m->requests[ri].subject != subject) want = UC_DENIED;
                got = uc_cancel(c, subject, token); CHECK(got == want);
                if (got == UC_OK) m->requests[ri].id = 0;
            } else {
                want = !c ? UC_ARGUMENT : pending(m, -1) ? UC_BUSY : UC_OK;
                got = uc_destroy(c); CHECK(got == want);
                if (got == UC_OK) {
                    fail_alloc = false;
                    m->c = uc_create(m->max_bytes, m->max_requests); CHECK(m->c != NULL);
                    memset(m->loans, 0, sizeof(m->loans)); memset(m->requests, 0, sizeof(m->requests));
                }
            }
            fail_alloc = false;
            if (got == UC_OK) ++successes[op_now];
            if (result != UINT64_MAX) { history[history_pos++ % 128] = result; }
            reap_model(m);
            CHECK(used(m) <= m->max_bytes && pending(m, -1) <= m->max_requests);
        }
        for (unsigned j = 0; j < 2; ++j) {
            for (unsigned i = 0; i < SLOTS; ++i) if (models[j].requests[i].id)
                CHECK(uc_cancel(models[j].c, models[j].requests[i].subject, models[j].requests[i].id) == UC_OK);
            CHECK(uc_destroy(models[j].c) == UC_OK);
        }
    }
    printf("SECUENCIAS-OK: semillas=1..32 pasos_por_semilla=6000 operaciones=192000 comprobaciones=%u; ledger independiente, fallos de calloc y reinicios\n", checks);
    printf("EXITO: prestamos=%u begin=%u revoke=%u complete=%u cancel=%u destroy=%u\n",
        successes[0]+successes[1]+successes[2], successes[3]+successes[4]+successes[5],
        successes[6], successes[7], successes[8], successes[9]);
    return 0;
}
