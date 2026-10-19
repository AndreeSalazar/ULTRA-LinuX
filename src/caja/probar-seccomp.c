/* SPDX-License-Identifier: GPL-2.0-only */
#define _GNU_SOURCE
#include <errno.h>
#include <signal.h>
#include <sys/prctl.h>
#include <sys/socket.h>
#include <sys/wait.h>
#include <unistd.h>
#include "../guardian/ultra_guard.h"
#include "seccomp-politica.h"
#include "syscalls-politica.h"
#include "syscalls-x86_64.h"

#define ENTRY(name, why) SYS_##name,
static const int allowed[] = {CAJA_SYSCALLS(ENTRY)};
#undef ENTRY

/* Tiny independent interpreter for the exact cBPF subset emitted by builder. */
static uint32_t evaluate(const struct sock_filter *prog, size_t len, const struct seccomp_data *data) {
    uint32_t value = 0;
    for (size_t pc = 0; pc < len; pc++) {
        const struct sock_filter *p = &prog[pc];
        switch (p->code) {
        case BPF_LD | BPF_W | BPF_ABS:
            ULTRA_CHECK(p->k <= sizeof(*data) - sizeof(value), "TEST-BPF", "lectura fuera de seccomp_data");
            memcpy(&value, (const unsigned char *)data + p->k, sizeof(value)); break;
        case BPF_JMP | BPF_JEQ | BPF_K: pc += value == p->k ? p->jt : p->jf; break;
        case BPF_JMP | BPF_JGE | BPF_K: pc += value >= p->k ? p->jt : p->jf; break;
        case BPF_JMP | BPF_JSET | BPF_K: pc += (value & p->k) ? p->jt : p->jf; break;
        case BPF_RET | BPF_K: return p->k;
        default: ultra_fail("TEST-BPF", __FILE__, __LINE__, "instruccion desconocida");
        }
    }
    ultra_fail("TEST-BPF", __FILE__, __LINE__, "sin retorno");
}

static bool granted(int nr) {
    for (size_t i = 0; i < ULTRA_LEN(allowed); i++) if (nr == allowed[i]) return true;
    return false;
}

static void kernel_test(const struct sock_filter *prog, size_t len, int which) {
    pid_t child = fork();
    ULTRA_CHECK(child >= 0, "TEST-KERNEL", "fork fallo");
    if (child == 0) {
        struct sock_fprog filter = {.len = (unsigned short)len, .filter = (struct sock_filter *)(uintptr_t)prog};
        if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 ||
            prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, &filter) != 0) _exit(10);
        if (which == 0) _exit(syscall(SYS_getpid) > 0 ? 0 : 11);
        if (which == 1) {
            long r = syscall(SYS_socket, AF_INET, SOCK_STREAM, 0);
            _exit(r == -1 && errno == ENOSYS ? 0 : 12);
        }
        if (which == 2) {
            long r = syscall(SYS_mprotect, NULL, 0, PROT_READ | PROT_EXEC);
            _exit(r == -1 && errno == ENOSYS ? 0 : 13);
        }
        if (which == 3) {
            long r = syscall(SYS_mmap, NULL, 4096, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
            _exit(r != -1 ? 0 : 14);
        }
        if (which == 4) { (void)syscall(0x40000000u | SYS_getpid); _exit(15); }
        long r = -2;
        switch (which) {
        case 5: r = syscall(SYS_ioctl, 0, TIOCSTI, NULL); break;
        case 6: r = syscall(SYS_prctl, PR_SET_DUMPABLE, 0, 0, 0, 0); break;
        case 7: _exit(syscall(SYS_fcntl, 1, F_GETFL) >= 0 ? 0 : 16);
        case 8: r = syscall(SYS_fcntl, 1, F_SETOWN, 0); break;
        case 9: {
            unsigned long gs = 0;
            _exit(syscall(SYS_arch_prctl, ARCH_GET_GS, &gs) == 0 ? 0 : 17);
        }
        case 10: r = syscall(SYS_arch_prctl, 0x5002, 1); break; /* ARCH_SHSTK_DISABLE */
        case 11: _exit(syscall(SYS_prctl, PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0) == 1 ? 0 : 18);
        case 12: { int fds[2]; r = syscall(SYS_socketpair, AF_UNIX, SOCK_STREAM, 0, fds); break; }
        case 13: r = syscall(SYS_connect, -1, NULL, 0); break;
        case 14: r = syscall(SYS_setuid, 0); break;
        case 15: r = syscall(SYS_setgid, 0); break;
        case 16: r = syscall(SYS_capset, NULL, NULL); break;
        case 17: r = syscall(SYS_unshare, 0); break;
        case 18: r = syscall(SYS_setns, -1, 0); break;
        case 19: r = syscall(SYS_mount, NULL, NULL, NULL, 0, NULL); break;
        default: _exit(19);
        }
        _exit(r == -1 && errno == ENOSYS ? 0 : 20);
    }
    int st = 0;
    ULTRA_CHECK(waitpid(child, &st, 0) == child, "TEST-KERNEL", "wait fallo");
    ULTRA_CHECK(which == 4 ? WIFSIGNALED(st) && WTERMSIG(st) == SIGSYS : WIFEXITED(st) && WEXITSTATUS(st) == 0,
                "TEST-KERNEL", "decision del kernel distinta de la esperada");
}

int main(int argc, char **argv) {
    struct sock_filter prog[CAJA_FILTER_CAP];
    size_t len = caja_filter_build(prog, ULTRA_LEN(prog), allowed, ULTRA_LEN(allowed));
    ULTRA_CHECK(len > 0, "TEST-BPF", "filtro no construido");
    if (argc == 2 && !strcmp(argv[1], "--dump")) {
        _Static_assert(sizeof(struct sock_filter) == 8, "formato BPF inesperado");
        return fwrite(prog, sizeof(prog[0]), len, stdout) == len ? 0 : 2;
    }
    if (argc != 1) return 2;
    struct seccomp_data d = {.arch = AUDIT_ARCH_X86_64, .args = {65534u}};
    size_t yes = 0, no = 0;
    for (size_t i = 0; i < ULTRA_LEN(caja_syscall_names); i++) {
        int nr = caja_syscall_names[i].nr;
        memset(d.args, 0, sizeof(d.args));
        if (nr == SYS_setuid || nr == SYS_setgid) d.args[0] = 65534u;
        if (nr == SYS_ioctl) d.args[1] = TCGETS;
        if (nr == SYS_fcntl) d.args[1] = F_GETFL;
        if (nr == SYS_prctl) d.args[0] = PR_GET_NO_NEW_PRIVS;
        if (nr == SYS_arch_prctl) d.args[0] = ARCH_SET_FS;
        d.nr = nr;
        uint32_t want = granted(nr) ? SECCOMP_RET_ALLOW : SECCOMP_RET_TRACE;
        ULTRA_CHECK(evaluate(prog, len, &d) == want, "TEST-BPF", "decision nativa incorrecta");
        if (granted(nr)) yes++; else no++;
        d.nr = nr | 0x40000000;
        ULTRA_CHECK(evaluate(prog, len, &d) == SECCOMP_RET_KILL_PROCESS, "TEST-BPF", "x32 aceptada");
    }
    d.nr = SYS_read; d.arch = AUDIT_ARCH_I386;
    ULTRA_CHECK(evaluate(prog, len, &d) == SECCOMP_RET_KILL_PROCESS, "TEST-BPF", "ABI i386 aceptada");
    d.arch = AUDIT_ARCH_X86_64; d.nr = 999999;
    ULTRA_CHECK(evaluate(prog, len, &d) == SECCOMP_RET_TRACE, "TEST-BPF", "numero desconocido aceptado");
    for (int i = 0; i < 2; i++) {
        d.nr = i == 0 ? SYS_mmap : SYS_mprotect;
        d.args[2] = PROT_READ | PROT_EXEC;
        ULTRA_CHECK(evaluate(prog, len, &d) == SECCOMP_RET_TRACE, "TEST-BPF", "PROT_EXEC aceptado");
    }
    memset(d.args, 0, sizeof(d.args));
    const uint32_t ioctls[] = {TCGETS, TCSETS, TCSETSW, TCSETSF, TIOCGWINSZ, TIOCGPGRP, FIONREAD};
    for (uint32_t cmd = 0; cmd <= 0xffffu; cmd++) {
        bool expected = false;
        for (size_t i = 0; i < ULTRA_LEN(ioctls); i++) if (cmd == ioctls[i]) expected = true;
        d.nr = SYS_ioctl; d.args[1] = cmd;
        ULTRA_CHECK(evaluate(prog, len, &d) == (expected ? SECCOMP_RET_ALLOW : SECCOMP_RET_TRACE),
                    "TEST-BPF", "comando ioctl distinto del contrato");
    }
    const int constrained[] = {SYS_fcntl, SYS_prctl, SYS_arch_prctl};
    for (size_t i = 0; i < ULTRA_LEN(constrained); i++) {
        memset(d.args, 0, sizeof(d.args));
        d.nr = constrained[i];
        d.args[d.nr == SYS_fcntl ? 1 : 0] = 0xdeadbeefu;
        ULTRA_CHECK(evaluate(prog, len, &d) == SECCOMP_RET_TRACE, "TEST-BPF", "comando desconocido aceptado");
    }
    int bad[] = {SYS_read, SYS_read};
    for (int i = 0; i < 2; i++) {
        d.nr = i == 0 ? SYS_setuid : SYS_setgid;
        d.args[0] = 0;
        ULTRA_CHECK(evaluate(prog, len, &d) == SECCOMP_RET_TRACE, "TEST-BPF", "cambio de identidad aceptado");
    }
    ULTRA_CHECK(caja_filter_build(prog, ULTRA_LEN(prog), bad, 2) == 0, "TEST-BPF", "duplicados aceptados");
    bad[0] = SYS_ptrace;
    ULTRA_CHECK(caja_filter_build(prog, ULTRA_LEN(prog), bad, 1) == 0, "TEST-BPF", "ptrace aceptado");
    len = caja_filter_build(prog, ULTRA_LEN(prog), allowed, ULTRA_LEN(allowed));
    for (int i = 0; i < 20; i++) kernel_test(prog, len, i);
    printf("SECCOMP-OK: nativas=%zu permitidas=%zu denegadas=%zu; ioctl=65536 decisiones; comandos desconocidos rechazados; kernel=20/20 (incluye UID/GID 0, capset, unshare, setns, mount)\n",
           ULTRA_LEN(caja_syscall_names), yes, no);
    return 0;
}
