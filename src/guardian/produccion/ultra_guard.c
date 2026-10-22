/* SPDX-License-Identifier: GPL-2.0-only */
#define _GNU_SOURCE
#include <errno.h>
#include <signal.h>
#include <stdint.h>
#include <sys/ucontext.h>
#include <unistd.h>
#include "ultra_guard.h"

/* Linker symbol at the start of this program's image; pc offsets go through addr2line. */
extern const char __executable_start[];
static const char *guarded_program = "?";
static char alt_stack[16384];

/* Async-signal-safe output: no stdio, no malloc inside a dying process. */
static void put(const char *s) {
    size_t left = strlen(s);
    while (left > 0) {
        ssize_t n = write(STDERR_FILENO, s, left);
        if (n <= 0) {
            if (n < 0 && errno == EINTR) continue;
            return;
        }
        s += n;
        left -= (size_t)n;
    }
}

static void put_hex(uintptr_t v) {
    char out[2 + 16 + 1];
    char *p = out + sizeof(out) - 1;
    *p = 0;
    do {
        *--p = "0123456789abcdef"[v & 0xf];
        v >>= 4;
    } while (v && p > out + 2);
    *--p = 'x';
    *--p = '0';
    put(p);
}

static void put_dec(long v) {
    char out[24];
    char *p = out + sizeof(out) - 1;
    unsigned long u = v < 0 ? 0UL - (unsigned long)v : (unsigned long)v;
    *p = 0;
    do {
        *--p = (char)('0' + u % 10);
        u /= 10;
    } while (u && p > out + 1);
    if (v < 0) *--p = '-';
    put(p);
}

const char *ultra_explain_signal(int sig) {
    switch (sig) {
    case SIGILL:  return "GUARDIAN-UB: trampa de comportamiento indefinido (desborde, indice, desplazamiento, alineacion...); la build de diagnostico da el motivo exacto";
    case SIGSEGV: return "FRONTERA-MEMORIA: acceso a memoria fuera de las reglas (ver code)";
    case SIGBUS:  return "FRONTERA-MEMORIA: acceso a memoria mal mapeada o desalineada";
    case SIGFPE:  return "UB-ARITMETICA: division por cero o desborde aritmetico";
    case SIGABRT: return "ABORTO-GUARDIAN: FORTIFY, protector de pila o glibc detecto corrupcion (ver la linea anterior)";
    case SIGSYS:  return "CAJA: llamada al sistema no declarada";
    case SIGKILL: return "MATADO: SIGKILL (limite de memoria, OOM o apagado)";
    default:      return "terminado por senal";
    }
}

static void flag_handler(int sig, siginfo_t *info, void *context) {
    put("ROJO [");
    put(ultra_explain_signal(sig));
    put("]\n  programa=");
    put(guarded_program);
    put(" senal=");
    put_dec(sig);
    put(" code=");
    put_dec(info->si_code);
    if (sig == SIGSEGV && info->si_code == SEGV_MAPERR) put(" (SEGV_MAPERR: direccion sin mapear: pagina de guarda, NULL o puntero salvaje)");
    if (sig == SIGSEGV && info->si_code == SEGV_ACCERR) put(" (SEGV_ACCERR: memoria protegida: solo lectura, RELRO o pila no ejecutable)");
    if (sig == SIGSEGV && info->si_code == 10) put(" (SEGV_CPERR: la pila sombra CET bloqueo un retorno falso)");
    if (sig == SIGSEGV || sig == SIGBUS) {
        put(" direccion=");
        put_hex((uintptr_t)info->si_addr);
    }
    if (sig == SIGSYS) {
        put(" syscall=");
        put_dec(info->si_syscall);
    }
    const ucontext_t *uc = context;
    uintptr_t pc = (uintptr_t)uc->uc_mcontext.gregs[REG_RIP];
    put(" pc=inicio+");
    put_hex(pc - (uintptr_t)__executable_start);
    put("\n");
    /* Namespace PID 1 has special default-signal rules. Never re-execute a
     * fault there: exit_group ends the namespace and its descendants. */
    if (getpid() == 1) _exit(128 + sig);
    /* SA_RESETHAND restored the default action: re-raise so the parent sees the real signal. */
    if (sig == SIGABRT || sig == SIGSYS) raise(sig);
    /* Faults re-execute the instruction and now die with the default action. */
}

static _Noreturn void install_failure(const char *step) {
    int saved_errno = errno;
    put("ROJO [GUARDIAN-INSTALACION] programa=");
    put(guarded_program);
    put(" paso=");
    put(step);
    put(" errno=");
    put_dec(saved_errno);
    put("; no se continua con proteccion parcial\n");
    /* No reliance on a possibly half-installed signal handler, including PID 1. */
    _exit(78);
}

void ultra_guard_install(const char *program) {
    guarded_program = program;
    stack_t ss = {.ss_sp = alt_stack, .ss_size = sizeof(alt_stack), .ss_flags = 0};
    /* The alternate stack lets the flag print even after a stack overflow. */
    if (sigaltstack(&ss, NULL) < 0) install_failure("sigaltstack");
    struct sigaction sa;
    memset(&sa, 0, sizeof(sa));
    sa.sa_sigaction = flag_handler;
    /* SA_RESETHAND is bit 31: an unsigned pattern stored in the int sa_flags. */
    sa.sa_flags = (int)(SA_SIGINFO | SA_ONSTACK | SA_RESETHAND);
    if (sigemptyset(&sa.sa_mask) < 0) install_failure("sigemptyset");
    const int sigs[] = {SIGILL, SIGSEGV, SIGBUS, SIGFPE, SIGABRT, SIGSYS};
    for (size_t i = 0; i < ULTRA_LEN(sigs); i++)
        if (sigaction(sigs[i], &sa, NULL) < 0) install_failure("sigaction");
}

_Noreturn void ultra_fail(const char *code, const char *file, int line, const char *why) {
    put("ROJO [");
    put(code);
    put("] ");
    put(file);
    put(":");
    put_dec(line);
    put(": ");
    put(why);
    put("\n");
    if (getpid() == 1) _exit(128 + SIGABRT);
    abort();
}

_Noreturn void ultra_fail_overlap(const char *file, int line) {
    ultra_fail("FRONTERA-SOLAPE", file, line, "memcpy con regiones solapadas es UB; usar memmove");
}

bool ultra_copy(char *dst, size_t cap, const char *src) {
    ULTRA_CHECK(cap > 0, "FRONTERA-COPIA", "destino de tamano cero");
    dst[0] = 0;
    return ultra_append(dst, cap, src);
}

bool ultra_append(char *dst, size_t cap, const char *src) {
    size_t used = strnlen(dst, cap);
    ULTRA_CHECK(used < cap, "FRONTERA-COPIA", "destino sin terminador dentro de su tamano");
    size_t need = strlen(src);
    if (need >= cap - used) {
        /* Keep what fits, terminated, and let the caller decide; never write past cap. */
        memcpy(dst + used, src, cap - used - 1);
        dst[cap - 1] = 0;
        return false;
    }
    memcpy(dst + used, src, need + 1);
    return true;
}
