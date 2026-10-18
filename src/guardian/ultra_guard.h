/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * ULTRA LinuX guardians for C. Include this header LAST: it poisons unsafe
 * functions, so system headers that mention them must already be parsed.
 *
 * Every guardian that wakes up prints a flag (bandera) before stopping:
 *   ROJO [GUARDIAN-CODE] where: why
 * A guardian never lets the program continue in an undefined state.
 */
#ifndef ULTRA_GUARD_H
#define ULTRA_GUARD_H

#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Mina: these copy or format without knowing the destination size. */
#pragma GCC poison strcpy strcat sprintf vsprintf gets strtok

/* Stops the program with a flag. Never returns. */
_Noreturn void ultra_fail(const char *code, const char *file, int line, const char *why);

/*
 * Installs the signal flags: a crash, a UB trap, a FORTIFY abort or a
 * seccomp kill is explained before the process dies with the same signal.
 */
void ultra_guard_install(const char *program);

/* Human explanation of a death signal, shared with the PID 1 supervisor. */
const char *ultra_explain_signal(int sig);

#define ULTRA_CHECK(cond, code, why) \
    do { if (!(cond)) ultra_fail((code), __FILE__, __LINE__, (why)); } while (0)

/* Element count that refuses pointers: sizeof(p)/sizeof(*p) on a pointer is a classic mine. */
#define ULTRA_LEN(a) \
    (sizeof(a) / sizeof((a)[0]) + 0 * sizeof(struct { \
        _Static_assert(!__builtin_types_compatible_p(__typeof__(a), __typeof__(&(a)[0])), \
                       "ULTRA_LEN recibio un puntero, no un arreglo"); int ok; }))

/* Checked index: the access happens only after the bound is proven. */
#define ULTRA_AT(a, i) \
    (*({ size_t ultra_i_ = (size_t)(i); \
         ULTRA_CHECK(ultra_i_ < ULTRA_LEN(a), "FRONTERA-INDICE", "indice fuera del arreglo"); \
         &(a)[ultra_i_]; }))

/* Checked arithmetic: overflow is a flag, not a wrapped value. */
#define ULTRA_ADD(a, b) \
    ({ __typeof__((a) + (b)) ultra_r_; \
       ULTRA_CHECK(!__builtin_add_overflow((a), (b), &ultra_r_), "UB-DESBORDE", "suma desborda su tipo"); \
       ultra_r_; })
#define ULTRA_SUB(a, b) \
    ({ __typeof__((a) - (b)) ultra_r_; \
       ULTRA_CHECK(!__builtin_sub_overflow((a), (b), &ultra_r_), "UB-DESBORDE", "resta desborda su tipo"); \
       ultra_r_; })
#define ULTRA_MUL(a, b) \
    ({ __typeof__((a) * (b)) ultra_r_; \
       ULTRA_CHECK(!__builtin_mul_overflow((a), (b), &ultra_r_), "UB-DESBORDE", "multiplicacion desborda su tipo"); \
       ultra_r_; })

/*
 * Bounded copy and append: the destination size travels with the call.
 * Truncation is a guardian decision, never a silent cut.
 */
bool ultra_copy(char *dst, size_t cap, const char *src);
bool ultra_append(char *dst, size_t cap, const char *src);
#define ULTRA_COPY(dst, src) ultra_copy((dst), ULTRA_LEN(dst), (src))
#define ULTRA_APPEND(dst, src) ultra_append((dst), ULTRA_LEN(dst), (src))

#endif
