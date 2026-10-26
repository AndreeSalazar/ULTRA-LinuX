/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * caja: runs a program inside a box built from a compiled declaration.
 * The program DECLARES paths, syscalls and limits; caja GRANTS only those;
 * the kernel CHECKS every access. Declarations are C data, not a parsed file:
 * there is no configuration parser to attack.
 *
 * Layers, applied in the child before execve:
 *   mandatory namespaces (mount, pid, uts, ipc and net)
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
#include <sys/mount.h>
#include <sys/ptrace.h>
#include <sys/resource.h>
#include <sys/syscall.h>
#include <sys/user.h>
#include <sys/wait.h>
#include <unistd.h>
#include "../guardian/ultra_guard.h"
#include "syscalls-politica.h"
#include "seccomp-politica.h"
#include "syscalls-x86_64.h"
#include "stdio-politica.h"
#include "identidad-politica.h"

enum { NOBODY = 65534 };

struct caja {
    const char *name;
    const char *why;
    const char *const *read;
    const char *const *write;   /* also create, delete and truncate */
    const char *const *execute;
    const int *syscalls;
    size_t nsyscalls;
    rlim_t memory;
};

/* Measured with strace on the static BusyBox: sh, cat, echo, ls, wc, uname, sleep; getgroups for id. */
static const int busybox_base[] = {
#define CAJA_NR(name, why) SYS_##name,
    CAJA_SYSCALLS(CAJA_NR)
#undef CAJA_NR
};

static const char *const bin_etc[] = {"/bin", "/etc", NULL};
static const char *const bin_only[] = {"/bin", NULL};
static const char *const tmp_only[] = {"/tmp", NULL};

static const struct caja cajas[] = {
    {"lector", "lee /etc y ejecuta BusyBox; no escribe nada, no tiene red", bin_etc, NULL, bin_only,
     busybox_base, ULTRA_LEN(busybox_base), 64u << 20},
    {"taller", "ejecuta BusyBox y escribe solo en /tmp; no ve /etc", bin_only, tmp_only, bin_only,
     busybox_base, ULTRA_LEN(busybox_base), 128u << 20},
};

static const char *syscall_name(long nr) {
    for (size_t i = 0; i < ULTRA_LEN(caja_syscall_names); i++)
        if (caja_syscall_names[i].nr == nr) return caja_syscall_names[i].name;
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
    __u64 read_rights = LANDLOCK_ACCESS_FS_READ_FILE | LANDLOCK_ACCESS_FS_READ_DIR;
    __u64 write_rights = read_rights | LANDLOCK_ACCESS_FS_WRITE_FILE | LANDLOCK_ACCESS_FS_REMOVE_DIR |
                         LANDLOCK_ACCESS_FS_REMOVE_FILE | LANDLOCK_ACCESS_FS_MAKE_REG |
                         LANDLOCK_ACCESS_FS_MAKE_DIR | LANDLOCK_ACCESS_FS_MAKE_SYM;
    /* Handle the filesystem rights known by these headers and available at runtime. */
    __u64 handled = (LANDLOCK_ACCESS_FS_MAKE_SYM << 1) - 1;
    if (abi >= 2) handled |= LANDLOCK_ACCESS_FS_REFER;
    if (abi >= 3) { handled |= LANDLOCK_ACCESS_FS_TRUNCATE; write_rights |= LANDLOCK_ACCESS_FS_TRUNCATE; }
#ifdef LANDLOCK_ACCESS_FS_IOCTL_DEV
    if (abi >= 5) handled |= LANDLOCK_ACCESS_FS_IOCTL_DEV;
#endif
#ifdef LANDLOCK_ACCESS_FS_RESOLVE_UNIX
    if (abi >= 9) handled |= LANDLOCK_ACCESS_FS_RESOLVE_UNIX;
#endif
    struct landlock_ruleset_attr attr = {
        .handled_access_fs = handled,
        .handled_access_net = LANDLOCK_ACCESS_NET_BIND_TCP | LANDLOCK_ACCESS_NET_CONNECT_TCP,
        .scoped = LANDLOCK_SCOPE_ABSTRACT_UNIX_SOCKET | LANDLOCK_SCOPE_SIGNAL
    };
#ifdef LANDLOCK_ACCESS_NET_BIND_UDP
    if (abi >= 10) attr.handled_access_net |= LANDLOCK_ACCESS_NET_BIND_UDP | LANDLOCK_ACCESS_NET_CONNECT_SEND_UDP;
#endif
    long fd = syscall(SYS_landlock_create_ruleset, &attr, sizeof(attr), 0);
    ULTRA_CHECK(fd >= 0, "CAJA-LANDLOCK", "no se pudo crear el conjunto de reglas");
    const char *const *lists[] = {c->read, c->write, c->execute};
    const __u64 rights[] = {read_rights, write_rights, LANDLOCK_ACCESS_FS_EXECUTE};
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
    struct sock_filter prog[CAJA_FILTER_CAP];
    size_t n = caja_filter_build(prog, ULTRA_LEN(prog), c->syscalls, c->nsyscalls);
    ULTRA_CHECK(n > 0, "CAJA-SECCOMP", "declaracion invalida: limite, duplicado, ABI o ptrace");
    struct sock_fprog fprog = {.len = (unsigned short)n, .filter = prog};
    ULTRA_CHECK(prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, &fprog) == 0, "CAJA-SECCOMP", "filtro rechazado por el kernel");
}

static void limit(__rlimit_resource_t resource, rlim_t value) {
    struct rlimit r = {value, value};
    ULTRA_CHECK(setrlimit(resource, &r) == 0, "CAJA-LIMITE", "setrlimit rechazado");
}

static _Noreturn void enter_box(const struct caja *c, int ready_fd, char **argv, int abi) {
    char go;
    /* Wait until the tracer is attached: an undeclared syscall must never go unseen. */
    ULTRA_CHECK(read(ready_fd, &go, 1) == 1, "CAJA-TRAZA", "el supervisor no confirmo la traza");
    close(ready_fd);
    ULTRA_CHECK(caja_stdio_valid(), "CAJA-FD", "stdio exige archivo, pipe, terminal o /dev/null; sockets y otros dispositivos rechazados");
    ULTRA_CHECK(syscall(SYS_close_range, 3u, ~0u, 0u) == 0,
                "CAJA-FD", "no se pudieron cerrar descriptores heredados");
    ULTRA_CHECK(sethostname("caja", 4) == 0, "CAJA-NS", "sethostname rechazado en el namespace UTS");
    limit(RLIMIT_AS, c->memory);
    limit(RLIMIT_NPROC, 32);
    limit(RLIMIT_NOFILE, 64);
    limit(RLIMIT_CORE, 0);
    limit(RLIMIT_CPU, 30);
    limit(RLIMIT_FSIZE, 16u << 20);
    ULTRA_CHECK(caja_drop_identity(NOBODY, NOBODY), "CAJA-IDENTIDAD",
                "se exigen UID/GID real, efectivo, guardado y FS 65534, cero grupos/capacidades y no_new_privs");
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
    if (unshare(*flags) < 0) return -1;
    if (mount(NULL, "/", NULL, MS_REC | MS_PRIVATE, NULL) < 0) return -1;
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
            ULTRA_CHECK(errno == ECHILD, "CAJA-TRAZA", "waitpid fallo antes de recoger la caja");
            return result;
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
            ULTRA_CHECK(kill(box, SIGKILL) == 0 || errno == ESRCH,
                        "CAJA-TRAZA", "no se pudo destruir la caja");
            /* Never resume an offending thread: the forbidden syscall has not run. */
            continue;
        } else if (event == 0 && sig != SIGSTOP) {
            pass = sig; /* An ordinary signal for the boxed program: deliver it. */
        }
        ULTRA_CHECK(ptrace(PTRACE_CONT, w, NULL, (void *)(intptr_t)pass) == 0 || errno == ESRCH,
                    "CAJA-TRAZA", "no se pudo reanudar el proceso trazado");
    }
}

int main(int argc, char **argv) {
    if (!caja_stdio_valid()) {
        /* Never send even a diagnostic to a rejected socket/device on stderr. */
        if (caja_stdio_fd_valid(STDERR_FILENO)) say("ROJO", "CAJA-FD", "stdio no autorizado; programa no ejecutado");
        return 4;
    }
    ultra_guard_install("caja");
    if (argc == 2 && !strcmp(argv[1], "--dump-filter")) {
        struct sock_filter prog[CAJA_FILTER_CAP];
        size_t n = caja_filter_build(prog, ULTRA_LEN(prog), busybox_base, ULTRA_LEN(busybox_base));
        ULTRA_CHECK(n > 0 && sizeof(prog[0]) == 8, "CAJA-SECCOMP", "filtro no exportable");
        return fwrite(prog, sizeof(prog[0]), n, stdout) == n ? 0 : 2;
    }
    const struct caja *c = NULL;
    for (size_t i = 0; argc >= 3 && i < ULTRA_LEN(cajas); i++)
        if (!strcmp(argv[1], cajas[i].name)) c = &cajas[i];
    if (!c) {
        fputs("uso: caja NOMBRE PROGRAMA [ARGS...]\ncajas:\n", stderr);
        for (size_t i = 0; i < ULTRA_LEN(cajas); i++) fprintf(stderr, "  %-8s %s\n", cajas[i].name, cajas[i].why);
        return 2;
    }
    int abi = landlock_abi();
    if (abi < 6) {
        say("ROJO", "CAJA-LANDLOCK", "se exige Landlock ABI >= 6: archivos, TCP y aislamiento IPC; no se ejecuta una caja mas debil");
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
