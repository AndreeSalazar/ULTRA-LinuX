/* SPDX-License-Identifier: GPL-2.0-only */
#define _GNU_SOURCE
#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include "ultra_guard.h"

/* Linker wrappers exist only in the test executable, not production. */
int __real_sigaltstack(const stack_t *, stack_t *);
int __real_sigaction(int, const struct sigaction *, struct sigaction *);
int __real_sigemptyset(sigset_t *);
int __wrap_sigaltstack(const stack_t *, stack_t *);
int __wrap_sigaction(int, const struct sigaction *, struct sigaction *);
int __wrap_sigemptyset(sigset_t *);
static bool fail_stack, fail_mask;
static int fail_action, action_calls;

int __wrap_sigaltstack(const stack_t *ss, stack_t *old) {
    if (ss && fail_stack) { errno = EPERM; return -1; }
    return __real_sigaltstack(ss, old);
}
int __wrap_sigemptyset(sigset_t *set) {
    if (fail_mask) { errno = EINVAL; return -1; }
    return __real_sigemptyset(set);
}
int __wrap_sigaction(int sig, const struct sigaction *sa, struct sigaction *old) {
    if (sa && ++action_calls == fail_action) { errno = EPERM; return -1; }
    return __real_sigaction(sig, sa, old);
}

int main(int argc, char **argv) {
    if (argc != 2) return 2;
    bool trap = strcmp(argv[1], "trap") == 0;
    fail_stack = strcmp(argv[1], "stack") == 0;
    fail_mask = strcmp(argv[1], "mask") == 0;
    if (strlen(argv[1]) == 1 && argv[1][0] >= '1' && argv[1][0] <= '6') fail_action = argv[1][0] - '0';
    else if (!fail_stack && !fail_mask && !trap && strcmp(argv[1], "ok") != 0) return 2;
    ultra_guard_install("probar-instalacion");
    if (fail_stack || fail_mask || fail_action) {
        puts("INSTALACION-INCOMPLETA-CONTINUO");
        return 3;
    }
    stack_t stack;
    ULTRA_CHECK(sigaltstack(NULL, &stack) == 0 && !(stack.ss_flags & SS_DISABLE) &&
                stack.ss_sp != NULL && stack.ss_size >= 16384,
                "TEST-INSTALACION", "pila alternativa no instalada");
    const int sigs[] = {SIGILL, SIGSEGV, SIGBUS, SIGFPE, SIGABRT, SIGSYS};
    for (size_t i = 0; i < ULTRA_LEN(sigs); ++i) {
        struct sigaction sa;
        ULTRA_CHECK(sigaction(sigs[i], NULL, &sa) == 0 && sa.sa_sigaction != NULL &&
                    ((unsigned)sa.sa_flags & (unsigned)(SA_SIGINFO | SA_ONSTACK | SA_RESETHAND)) ==
                    (unsigned)(SA_SIGINFO | SA_ONSTACK | SA_RESETHAND),
                    "TEST-INSTALACION", "manejador no instalado con sus flags");
    }
    ULTRA_CHECK(action_calls == 6, "TEST-INSTALACION", "numero de instalaciones incorrecto");
    puts("INSTALACION-OK: pila y seis manejadores consultados en el kernel");
    fflush(stdout);
    if (trap) __builtin_trap();
    return 0;
}
