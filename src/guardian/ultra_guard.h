/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * ULTRA LinuX guardians for C. Include this header LAST: it poisons unsafe
 * functions, so system headers that mention them must already be parsed.
 *
 * Every guardian that wakes up prints a flag (bandera) before stopping:
 *   ROJO [GUARDIAN-CODE] where: why
 * Explicit guards stop covered invalid operations before performing them.
 * Ordinary C outside these APIs still needs review, sanitizers and isolation.
 */
#ifndef ULTRA_GUARD_H
#define ULTRA_GUARD_H

#include <stdbool.h>
#include <limits.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/*
 * Mines: strcpy/strcat/sprintf/gets write without knowing the destination size;
 * strncpy/strncat may leave the string without its terminator; strtok keeps
 * hidden global state; atoi/atol/atoll are UB when the number does not fit.
 */
#pragma GCC poison strcpy strcat sprintf vsprintf gets strtok strncpy strncat atoi atol atoll

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

/* Explicit int API: operands are evaluated once; checks precede the operation.
 * Callers must supply int values: these functions cannot validate an earlier
 * narrowing conversion, dangling pointer or UB used to calculate an operand.
 */
static inline int ultra_div_int(int a, int b, const char *file, int line) {
    if (b == 0) ultra_fail("UB-DIVISION", file, line, "divisor cero");
    if (a == INT_MIN && b == -1)
        ultra_fail("UB-DIVISION", file, line, "INT_MIN / -1 no cabe en int");
    return a / b;
}
static inline int ultra_rem_int(int a, int b, const char *file, int line) {
    if (b == 0) ultra_fail("UB-RESTO", file, line, "divisor cero en resto");
    if (a == INT_MIN && b == -1)
        ultra_fail("UB-RESTO", file, line, "INT_MIN % -1 tambien es UB");
    return a % b;
}
static inline int ultra_shl_int(int a, int count, const char *file, int line) {
    /* This Linux/GCC API accepts nonnegative results representable in int.
     * Checking the count first keeps the guard's own right shift defined.
     */
    if (count < 0 || (unsigned int)count >= sizeof(int) * CHAR_BIT)
        ultra_fail("UB-DESPLAZAMIENTO", file, line, "contador fuera del ancho de int");
    if (a < 0 || a > (INT_MAX >> count))
        ultra_fail("UB-DESPLAZAMIENTO", file, line, "base negativa o resultado fuera de int");
    return a << count;
}
#define ULTRA_DIV_INT(a, b) ultra_div_int((a), (b), __FILE__, __LINE__)
#define ULTRA_REM_INT(a, b) ultra_rem_int((a), (b), __FILE__, __LINE__)
#define ULTRA_SHL_INT(a, n) ultra_shl_int((a), (n), __FILE__, __LINE__)

/*
 * Bounded copy and append: the destination size travels with the call.
 * Truncation is a guardian decision, never a silent cut.
 */
bool ultra_copy(char *dst, size_t cap, const char *src);
bool ultra_append(char *dst, size_t cap, const char *src);
#ifndef ULTRA_CRUDO
/*
 * memcpy with overlapping regions is UB, and neither glibc nor FORTIFY checks
 * it. Every memcpy after this header checks the overlap, then keeps FORTIFY's
 * destination-size check. The destination is evaluated once.
 */
_Noreturn void ultra_fail_overlap(const char *file, int line);
static inline void *ultra_memcpy(void *d, const void *s, size_t n, size_t dsize, const char *file, int line) {
    /* Size first: an overflowing copy also overlaps its neighbour, and the flag must name the real cause. */
    if (n > dsize) return __builtin___memcpy_chk(d, s, n, dsize);
    uintptr_t a = (uintptr_t)d, b = (uintptr_t)s;
    if (n && (a < b ? b - a < n : a - b < n)) ultra_fail_overlap(file, line);
    return __builtin___memcpy_chk(d, s, n, dsize);
}
#define memcpy(d, s, n) \
    ({ void *ultra_d_ = (d); \
       ultra_memcpy(ultra_d_, (s), (n), __builtin_dynamic_object_size(ultra_d_, 0), __FILE__, __LINE__); })
#endif

#define ULTRA_COPY(dst, src) ultra_copy((dst), ULTRA_LEN(dst), (src))
#define ULTRA_APPEND(dst, src) ultra_append((dst), ULTRA_LEN(dst), (src))

#endif
