/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef CAJA_SECCOMP_POLITICA_H
#define CAJA_SECCOMP_POLITICA_H
#include <linux/audit.h>
#include <linux/filter.h>
#include <linux/seccomp.h>
#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>
#include <fcntl.h>
#include <asm/prctl.h>
#include <sys/ioctl.h>
#include <sys/prctl.h>
#include <sys/mman.h>
#include <sys/syscall.h>

enum { CAJA_MAX_SYSCALLS = 64, CAJA_FILTER_CAP = 256 };
/* Command arguments are int/unsigned int in these four kernel interfaces. */
static bool caja_filter_commands(struct sock_filter *prog, size_t cap, size_t *used,
                                 int nr, size_t arg, const uint32_t *values, size_t count) {
    if (arg >= 6 || count > 32 || *used + count + 4 > cap) return false;
    size_t n = *used;
#define CMD(c, k, t, f) (prog[n++] = (struct sock_filter){(__u16)(c), (__u8)(t), (__u8)(f), (__u32)(k)})
    CMD(BPF_JMP | BPF_JEQ | BPF_K, nr, 0, count + 2);
    CMD(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, args) + arg * sizeof(uint64_t), 0, 0);
    for (size_t i = 0; i < count; i++) CMD(BPF_JMP | BPF_JEQ | BPF_K, values[i], count - i, 0);
    CMD(BPF_RET | BPF_K, SECCOMP_RET_TRACE, 0, 0);
    CMD(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, nr), 0, 0);
#undef CMD
    *used = n;
    return true;
}
/* Pure builder, shared by the live box and exhaustive BPF decision tests. */
static size_t caja_filter_build(struct sock_filter *prog, size_t cap,
                                const int *allow, size_t count) {
    if (!prog || !allow || count > CAJA_MAX_SYSCALLS || cap < CAJA_FILTER_CAP) return 0;
    for (size_t i = 0; i < count; i++) {
        if (allow[i] < 0 || (unsigned int)allow[i] >= 0x40000000u || allow[i] == SYS_ptrace) return 0;
        for (size_t j = 0; j < i; j++) if (allow[i] == allow[j]) return 0;
    }
    size_t n = 0;
#define CAJA_INSN(c, k, t, f) (prog[n++] = (struct sock_filter){(__u16)(c), (__u8)(t), (__u8)(f), (__u32)(k)})
    CAJA_INSN(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, arch), 0, 0);
    CAJA_INSN(BPF_JMP | BPF_JEQ | BPF_K, AUDIT_ARCH_X86_64, 1, 0);
    CAJA_INSN(BPF_RET | BPF_K, SECCOMP_RET_KILL_PROCESS, 0, 0);
    CAJA_INSN(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, nr), 0, 0);
    CAJA_INSN(BPF_JMP | BPF_JGE | BPF_K, 0x40000000u, 0, 1);
    CAJA_INSN(BPF_RET | BPF_K, SECCOMP_RET_KILL_PROCESS, 0, 0);
    const int mapping[] = {SYS_mmap, SYS_mprotect};
    for (size_t i = 0; i < 2; i++) {
        CAJA_INSN(BPF_JMP | BPF_JEQ | BPF_K, mapping[i], 0, 3);
        CAJA_INSN(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, args[2]), 0, 0);
        CAJA_INSN(BPF_JMP | BPF_JSET | BPF_K, PROT_EXEC, 0, 1);
        CAJA_INSN(BPF_RET | BPF_K, SECCOMP_RET_TRACE, 0, 0);
        CAJA_INSN(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, nr), 0, 0);
    }
    const int identity[] = {SYS_setuid, SYS_setgid};
    for (size_t i = 0; i < 2; i++) {
        CAJA_INSN(BPF_JMP | BPF_JEQ | BPF_K, identity[i], 0, 3);
        CAJA_INSN(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, args[0]), 0, 0);
        CAJA_INSN(BPF_JMP | BPF_JEQ | BPF_K, 65534u, 1, 0);
        CAJA_INSN(BPF_RET | BPF_K, SECCOMP_RET_TRACE, 0, 0);
        CAJA_INSN(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, nr), 0, 0);
    }
    const uint32_t ioctls[] = {TCGETS, TCSETS, TCSETSW, TCSETSF, TIOCGWINSZ, TIOCGPGRP, FIONREAD};
    const uint32_t fcntls[] = {F_GETFD, F_SETFD, F_GETFL, F_SETFL, F_DUPFD, F_DUPFD_CLOEXEC};
    const uint32_t prctls[] = {PR_GET_NO_NEW_PRIVS, PR_GET_SECCOMP, PR_GET_DUMPABLE, PR_GET_NAME, PR_SET_NAME};
    const uint32_t archs[] = {ARCH_SET_FS, ARCH_GET_FS, ARCH_GET_GS};
    if (!caja_filter_commands(prog, cap, &n, SYS_ioctl, 1, ioctls, sizeof(ioctls) / sizeof(ioctls[0])) ||
        !caja_filter_commands(prog, cap, &n, SYS_fcntl, 1, fcntls, sizeof(fcntls) / sizeof(fcntls[0])) ||
        !caja_filter_commands(prog, cap, &n, SYS_prctl, 0, prctls, sizeof(prctls) / sizeof(prctls[0])) ||
        !caja_filter_commands(prog, cap, &n, SYS_arch_prctl, 0, archs, sizeof(archs) / sizeof(archs[0]))) return 0;
    if (n + 2 * count + 1 > cap) return 0;
    for (size_t i = 0; i < count; i++) {
        CAJA_INSN(BPF_JMP | BPF_JEQ | BPF_K, allow[i], 0, 1);
        CAJA_INSN(BPF_RET | BPF_K, SECCOMP_RET_ALLOW, 0, 0);
    }
    CAJA_INSN(BPF_RET | BPF_K, SECCOMP_RET_TRACE, 0, 0);
#undef CAJA_INSN
    return n;
}
#endif
