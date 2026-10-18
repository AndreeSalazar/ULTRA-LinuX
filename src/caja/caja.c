/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * caja: runs a program inside a box built from a compiled declaration.
 * The program DECLARES paths, syscalls and limits; caja GRANTS only those;
 * the kernel CHECKS every access. Declarations are C data, not a parsed file:
 * there is no configuration parser to attack.
 *
 * Layers, applied in the child before execve:
 *   namespaces (mount, pid, uts; ipc/net when the kernel has them)
 *   rlimits (memory, processes, files, no core dumps)
 *   identity: bounding set dropped, uid/gid 65534, no_new_privs
 *   Landlock: filesystem access limited to the declared paths
 *   seccomp: undeclared syscalls stop at a ptrace event, so the flag names them
 * Any layer the kernel cannot provide is a ROJO refusal, never a weaker box.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <grp.h>
#include <linux/audit.h>
#include <linux/filter.h>
#include <linux/landlock.h>
#include <linux/seccomp.h>
#include <signal.h>
#include <stdint.h>
#include <sched.h>
#include <sys/prctl.h>
#include <sys/ptrace.h>
#include <sys/resource.h>
#include <sys/syscall.h>
#include <sys/user.h>
#include <sys/wait.h>
#include <unistd.h>
#include "../guardian/ultra_guard.h"

enum { NOBODY = 65534 };

struct caja {
    const char *name;
    const char *why;
    const char *const *read;    /* read and execute beneath these paths */
    const char *const *write;   /* also create, delete and truncate */
    const int *syscalls;
    size_t nsyscalls;
    rlim_t memory;
};

#define SYSCALL_ENTRY(n) {SYS_##n, #n}
static const struct { int nr; const char *name; } known[] = {
    SYSCALL_ENTRY(read), SYSCALL_ENTRY(write), SYSCALL_ENTRY(writev), SYSCALL_ENTRY(open),
    SYSCALL_ENTRY(openat), SYSCALL_ENTRY(close), SYSCALL_ENTRY(stat), SYSCALL_ENTRY(lstat),
    SYSCALL_ENTRY(fstat), SYSCALL_ENTRY(newfstatat), SYSCALL_ENTRY(lseek), SYSCALL_ENTRY(mmap),
    SYSCALL_ENTRY(munmap), SYSCALL_ENTRY(mprotect), SYSCALL_ENTRY(brk), SYSCALL_ENTRY(rt_sigaction),
    SYSCALL_ENTRY(rt_sigprocmask), SYSCALL_ENTRY(rt_sigreturn), SYSCALL_ENTRY(ioctl), SYSCALL_ENTRY(fcntl),
    SYSCALL_ENTRY(dup2), SYSCALL_ENTRY(getdents64), SYSCALL_ENTRY(execve), SYSCALL_ENTRY(arch_prctl),
    SYSCALL_ENTRY(set_tid_address), SYSCALL_ENTRY(prctl), SYSCALL_ENTRY(exit), SYSCALL_ENTRY(exit_group),
    SYSCALL_ENTRY(getpid), SYSCALL_ENTRY(getppid), SYSCALL_ENTRY(getuid), SYSCALL_ENTRY(getgid),
    SYSCALL_ENTRY(geteuid), SYSCALL_ENTRY(getegid), SYSCALL_ENTRY(setuid), SYSCALL_ENTRY(setgid),
    SYSCALL_ENTRY(uname), SYSCALL_ENTRY(fork), SYSCALL_ENTRY(vfork), SYSCALL_ENTRY(wait4),
    SYSCALL_ENTRY(nanosleep), SYSCALL_ENTRY(clock_nanosleep), SYSCALL_ENTRY(sendfile), SYSCALL_ENTRY(getcwd),
    SYSCALL_ENTRY(chdir), SYSCALL_ENTRY(socket), SYSCALL_ENTRY(connect), SYSCALL_ENTRY(ptrace),
    SYSCALL_ENTRY(mount), SYSCALL_ENTRY(unshare), SYSCALL_ENTRY(kill), SYSCALL_ENTRY(reboot),
    SYSCALL_ENTRY(setsid), SYSCALL_ENTRY(chmod), SYSCALL_ENTRY(mkdir), SYSCALL_ENTRY(unlink),
    SYSCALL_ENTRY(getgroups),
};

/* Measured with strace on the static BusyBox: sh, cat, echo, ls, wc, uname, sleep; getgroups for id. */
static const int busybox_base[] = {
    SYS_read, SYS_write, SYS_writev, SYS_open, SYS_openat, SYS_close, SYS_stat, SYS_lstat,
    SYS_fstat, SYS_newfstatat, SYS_lseek, SYS_mmap, SYS_munmap, SYS_mprotect, SYS_brk,
    SYS_rt_sigaction, SYS_rt_sigprocmask, SYS_rt_sigreturn, SYS_ioctl, SYS_fcntl, SYS_dup2,
    SYS_getdents64, SYS_execve, SYS_arch_prctl, SYS_set_tid_address, SYS_prctl, SYS_exit,
    SYS_exit_group, SYS_getpid, SYS_getppid, SYS_getuid, SYS_getgid, SYS_geteuid, SYS_getegid,
    SYS_setuid, SYS_setgid, SYS_uname, SYS_fork, SYS_vfork, SYS_wait4, SYS_nanosleep,
    SYS_clock_nanosleep, SYS_sendfile, SYS_getcwd, SYS_chdir, SYS_getgroups,
};

static const char *const bin_etc[] = {"/bin", "/etc", NULL};
static const char *const bin_only[] = {"/bin", NULL};
static const char *const tmp_only[] = {"/tmp", NULL};

static const struct caja cajas[] = {
    {"lector", "lee /etc y ejecuta BusyBox; no escribe nada, no tiene red", bin_etc, NULL,
     busybox_base, ULTRA_LEN(busybox_base), 64u << 20},
    {"taller", "ejecuta BusyBox y escribe solo en /tmp; no ve /etc", bin_only, tmp_only,
     busybox_base, ULTRA_LEN(busybox_base), 128u << 20},
};

static const char *syscall_name(long nr) {
    for (size_t i = 0; i < ULTRA_LEN(known); i++)
        if (known[i].nr == nr) return known[i].name;
    return "desconocida";
}

static void say(const char *level, const char *code, const char *detail) {
    fprintf(stderr, "%s [%s]: %s\n", level, code, detail);
}

static int landlock_abi(void) {
    long abi = syscall(SYS_landlock_create_ruleset, NULL, 0, LANDLOCK_CREATE_RULESET_VERSION);
    return abi < 0 ? -1 : (int)abi;
}

static void apply_landlock(const struct caja *c, int abi) {
    __u64 read_rights = LANDLOCK_ACCESS_FS_EXECUTE | LANDLOCK_ACCESS_FS_READ_FILE | LANDLOCK_ACCESS_FS_READ_DIR;
    __u64 write_rights = read_rights | LANDLOCK_ACCESS_FS_WRITE_FILE | LANDLOCK_ACCESS_FS_REMOVE_DIR |
                         LANDLOCK_ACCESS_FS_REMOVE_FILE | LANDLOCK_ACCESS_FS_MAKE_REG |
                         LANDLOCK_ACCESS_FS_MAKE_DIR | LANDLOCK_ACCESS_FS_MAKE_SYM;
    /* Every right this kernel knows is handled, so anything not granted is denied. */
    __u64 handled = (LANDLOCK_ACCESS_FS_MAKE_SYM << 1) - 1;
    if (abi >= 2) handled |= LANDLOCK_ACCESS_FS_REFER;
    if (abi >= 3) { handled |= LANDLOCK_ACCESS_FS_TRUNCATE; write_rights |= LANDLOCK_ACCESS_FS_TRUNCATE; }
#ifdef LANDLOCK_ACCESS_FS_IOCTL_DEV
    if (abi >= 5) handled |= LANDLOCK_ACCESS_FS_IOCTL_DEV;
#endif
    struct landlock_ruleset_attr attr = {.handled_access_fs = handled};
    long fd = syscall(SYS_landlock_create_ruleset, &attr, sizeof(attr), 0);
    ULTRA_CHECK(fd >= 0, "CAJA-LANDLOCK", "no se pudo crear el conjunto de reglas");
    const char *const *lists[] = {c->read, c->write};
    const __u64 rights[] = {read_rights, write_rights};
    for (size_t l = 0; l < ULTRA_LEN(lists); l++) {
        for (const char *const *p = lists[l]; p && *p; p++) {
            int dir = open(*p, O_PATH | O_CLOEXEC | O_DIRECTORY);
            ULTRA_CHECK(dir >= 0, "CAJA-LANDLOCK", "una ruta declarada no existe o no es un directorio");
            struct landlock_path_beneath_attr rule = {.allowed_access = rights[l] & handled, .parent_fd = dir};
            ULTRA_CHECK(syscall(SYS_landlock_add_rule, fd, LANDLOCK_RULE_PATH_BENEATH, &rule, 0) == 0,
                        "CAJA-LANDLOCK", "regla de ruta rechazada por el kernel");
            close(dir);
        }
    }
    ULTRA_CHECK(syscall(SYS_landlock_restrict_self, fd, 0) == 0, "CAJA-LANDLOCK", "restrict_self rechazado");
    close((int)fd);
}

static void apply_seccomp(const struct caja *c) {
    /* arch check, x32 rejection, allowlist; anything else stops for the tracer. */
    struct sock_filter prog[6 + 64 + 2];
    ULTRA_CHECK(c->nsyscalls <= 64, "CAJA-SECCOMP", "demasiadas llamadas declaradas para el filtro");
    size_t n = 0;
#define INSN(c_, k_, jt_, jf_) (prog[n++] = (struct sock_filter){(__u16)(c_), (__u8)(jt_), (__u8)(jf_), (__u32)(k_)})
    INSN(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, arch), 0, 0);
    INSN(BPF_JMP | BPF_JEQ | BPF_K, AUDIT_ARCH_X86_64, 1, 0);
    INSN(BPF_RET | BPF_K, SECCOMP_RET_KILL_PROCESS, 0, 0);
    INSN(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, nr), 0, 0);
    INSN(BPF_JMP | BPF_JGE | BPF_K, 0x40000000u, 0, 1);
    INSN(BPF_RET | BPF_K, SECCOMP_RET_KILL_PROCESS, 0, 0);
    for (size_t i = 0; i < c->nsyscalls; i++) INSN(BPF_JMP | BPF_JEQ | BPF_K, (__u32)c->syscalls[i], c->nsyscalls - i, 0);
    INSN(BPF_RET | BPF_K, SECCOMP_RET_TRACE, 0, 0);
    INSN(BPF_RET | BPF_K, SECCOMP_RET_ALLOW, 0, 0);
#undef INSN
    struct sock_fprog fprog = {.len = (unsigned short)n, .filter = prog};
    ULTRA_CHECK(prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, &fprog) == 0, "CAJA-SECCOMP", "filtro rechazado por el kernel");
}

static void limit(int resource, rlim_t value) {
    struct rlimit r = {value, value};
    ULTRA_CHECK(setrlimit(resource, &r) == 0, "CAJA-LIMITE", "setrlimit rechazado");
}

static _Noreturn void enter_box(const struct caja *c, int ready_fd, char **argv, int abi) {
    char go;
    /* Wait until the tracer is attached: an undeclared syscall must never go unseen. */
    ULTRA_CHECK(read(ready_fd, &go, 1) == 1, "CAJA-TRAZA", "el supervisor no confirmo la traza");
    close(ready_fd);
    ULTRA_CHECK(sethostname("caja", 4) == 0, "CAJA-NS", "sethostname rechazado en el namespace UTS");
    limit(RLIMIT_AS, c->memory);
    limit(RLIMIT_NPROC, 32);
    limit(RLIMIT_NOFILE, 64);
    limit(RLIMIT_CORE, 0);
    for (int cap = 0; prctl(PR_CAPBSET_DROP, cap, 0, 0, 0) == 0; cap++) {}
    ULTRA_CHECK(setgroups(0, NULL) == 0, "CAJA-IDENTIDAD", "setgroups rechazado");
    ULTRA_CHECK(setresgid(NOBODY, NOBODY, NOBODY) == 0, "CAJA-IDENTIDAD", "setresgid rechazado");
    ULTRA_CHECK(setresuid(NOBODY, NOBODY, NOBODY) == 0, "CAJA-IDENTIDAD", "setresuid rechazado");
    ULTRA_CHECK(prctl(PR_CAPBSET_READ, 0, 0, 0, 0) == 0, "CAJA-IDENTIDAD", "quedan capacidades en el conjunto limite");
    ULTRA_CHECK(prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) == 0, "CAJA-IDENTIDAD", "no_new_privs rechazado");
    apply_landlock(c, abi);
    apply_seccomp(c);
    char path[256];
    if (strchr(argv[0], '/')) ULTRA_CHECK(ULTRA_COPY(path, argv[0]), "CAJA-RUTA", "ruta demasiado larga");
    else ULTRA_CHECK(ULTRA_COPY(path, "/bin/") && ULTRA_APPEND(path, argv[0]), "CAJA-RUTA", "nombre demasiado largo");
    static char env_path[] = "PATH=/bin", env_home[] = "HOME=/tmp";
    char *const envp[] = {env_path, env_home, NULL};
    execve(path, argv, envp);
    ultra_fail("CAJA-EXEC", __FILE__, __LINE__, "execve fallo dentro de la caja");
}

static pid_t fork_box(int *flags) {
    /*
     * The supervisor enters the new namespaces itself, then forks: the child is
     * PID 1 of the new pid namespace. A raw clone() would leave glibc's cached
     * thread id stale, and abort() inside the box would signal the wrong task.
     */
    if (unshare(*flags) < 0) {
        if (errno != EINVAL || !(*flags & (CLONE_NEWNET | CLONE_NEWIPC))) return -1;
        say("AMARILLO", "CAJA-NS", "el kernel no ofrece namespaces de red/ipc; la caja sigue sin ellos (sin red en el kernel)");
        *flags &= ~(CLONE_NEWNET | CLONE_NEWIPC);
        if (unshare(*flags) < 0) return -1;
    }
    return fork();
}

/* Supervises the whole box tree; returns the exit status caja should report. */
static int supervise(pid_t box) {
    int result = 0;
    bool killed_for_syscall = false;
    for (;;) {
        int st;
        pid_t w = waitpid(-1, &st, __WALL);
        if (w < 0) {
            if (errno == EINTR) continue;
            return result; /* ECHILD: the box and every descendant are gone. */
        }
        if (WIFEXITED(st) || WIFSIGNALED(st)) {
            if (w != box) continue;
            char detail[256];
            if (killed_for_syscall) result = 3;
            else if (WIFSIGNALED(st)) {
                snprintf(detail, sizeof(detail), "senal %d: %s", WTERMSIG(st), ultra_explain_signal(WTERMSIG(st)));
                say("ROJO", "CAJA-MUERTE", detail);
                result = 3;
            } else {
                result = WEXITSTATUS(st);
                snprintf(detail, sizeof(detail), "el programa salio con codigo %d", result);
                say(result ? "AMARILLO" : "VERDE", "CAJA-SALIDA", detail);
            }
            continue;
        }
        if (!WIFSTOPPED(st)) continue;
        int event = st >> 16, sig = WSTOPSIG(st), pass = 0;
        if (event == PTRACE_EVENT_SECCOMP) {
            struct user_regs_struct regs;
            long nr = ptrace(PTRACE_GETREGS, w, NULL, &regs) == 0 ? (long)regs.orig_rax : -1;
            char detail[256];
            snprintf(detail, sizeof(detail), "llamada al sistema no declarada: %s (%ld) en pid %d; la caja se destruye",
                     syscall_name(nr), nr, (int)w);
            say("ROJO", "CAJA-SECCOMP", detail);
            killed_for_syscall = true;
            kill(box, SIGKILL); /* PID 1 of the namespace: its death ends every process inside. */
        } else if (event == 0 && sig != SIGTRAP && sig != SIGSTOP) {
            pass = sig; /* An ordinary signal for the boxed program: deliver it. */
        }
        (void)ptrace(PTRACE_CONT, w, NULL, (void *)(intptr_t)pass);
    }
}

int main(int argc, char **argv) {
    ultra_guard_install("caja");
    const struct caja *c = NULL;
    for (size_t i = 0; argc >= 3 && i < ULTRA_LEN(cajas); i++)
        if (!strcmp(argv[1], cajas[i].name)) c = &cajas[i];
    if (!c) {
        fputs("uso: caja NOMBRE PROGRAMA [ARGS...]\ncajas:\n", stderr);
        for (size_t i = 0; i < ULTRA_LEN(cajas); i++) fprintf(stderr, "  %-8s %s\n", cajas[i].name, cajas[i].why);
        return 2;
    }
    int abi = landlock_abi();
    if (abi < 1) {
        say("ROJO", "CAJA-LANDLOCK", "el kernel no tiene Landlock activo; no se ejecuta una caja mas debil");
        return 4;
    }
    if (prctl(PR_GET_SECCOMP, 0, 0, 0, 0) < 0) {
        say("ROJO", "CAJA-SECCOMP", "el kernel no tiene seccomp; no se ejecuta una caja mas debil");
        return 4;
    }
    int pipefd[2];
    ULTRA_CHECK(pipe2(pipefd, O_CLOEXEC) == 0, "CAJA-TRAZA", "pipe2 fallo");
    int flags = CLONE_NEWNS | CLONE_NEWPID | CLONE_NEWUTS | CLONE_NEWIPC | CLONE_NEWNET;
    pid_t box = fork_box(&flags);
    if (box < 0) {
        say("ROJO", "CAJA-NS", strerror(errno));
        return 4;
    }
    if (box == 0) {
        close(pipefd[1]);
        enter_box(c, pipefd[0], argv + 2, abi);
    }
    close(pipefd[0]);
    long opts = PTRACE_O_TRACESECCOMP | PTRACE_O_EXITKILL | PTRACE_O_TRACEFORK | PTRACE_O_TRACEVFORK |
                PTRACE_O_TRACECLONE | PTRACE_O_TRACEEXEC;
    if (ptrace(PTRACE_SEIZE, box, NULL, (void *)(intptr_t)opts) != 0) {
        say("ROJO", "CAJA-TRAZA", "no se pudo vigilar la caja; se destruye sin ejecutar nada");
        kill(box, SIGKILL);
        return 4;
    }
    char detail[256];
    snprintf(detail, sizeof(detail), "%s: %s (landlock ABI %d, %zu syscalls declaradas, %s)", c->name, c->why, abi,
             c->nsyscalls, (flags & CLONE_NEWNET) ? "ns mnt/pid/uts/ipc/net" : "ns mnt/pid/uts");
    say("VERDE", "CAJA", detail);
    ULTRA_CHECK(write(pipefd[1], "y", 1) == 1, "CAJA-TRAZA", "no se pudo liberar la caja");
    close(pipefd[1]);
    return supervise(box);
}
