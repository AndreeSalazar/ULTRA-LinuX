/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * ULTRA LinuX init: PID 1 mounts the base filesystems, then supervises a
 * BusyBox shell. Every outcome is reported as VERDE, AMARILLO or ROJO with a
 * code, so a failure is never silent. PID 1 must not exit: the kernel panics.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/mount.h>
#include <sys/reboot.h>
#include <sys/stat.h>
#include <sys/utsname.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

enum { RESPAWN_WINDOW_S = 2, RESPAWN_LIMIT = 5, RESPAWN_PAUSE_S = 5 };

static void emit(const char *s) {
    size_t left = strlen(s);
    while (left > 0) {
        ssize_t n = write(STDOUT_FILENO, s, left);
        if (n < 0) {
            if (errno == EINTR) continue;
            return;
        }
        s += n;
        left -= (size_t)n;
    }
}

static void report(const char *level, const char *code, const char *detail) {
    char line[256];
    int n = snprintf(line, sizeof(line), "%s [%s]: %s\n", level, code, detail);
    if (n < 0) return;
    /* Truncation is visible, not silent. */
    if ((size_t)n >= sizeof(line)) memcpy(line + sizeof(line) - 5, "...\n", 5);
    emit(line);
}

static void mount_fs(const char *type, const char *target, unsigned long flags) {
    char detail[160];
    if (mkdir(target, 0755) < 0 && errno != EEXIST) {
        snprintf(detail, sizeof(detail), "mkdir %s: %s", target, strerror(errno));
        report("ROJO", "MONTAJE", detail);
        return;
    }
    /* devtmpfs may already be mounted by CONFIG_DEVTMPFS_MOUNT. */
    if (mount(type, target, type, flags, NULL) < 0 && errno != EBUSY) {
        snprintf(detail, sizeof(detail), "%s en %s: %s", type, target, strerror(errno));
        report("ROJO", "MONTAJE", detail);
        return;
    }
    snprintf(detail, sizeof(detail), "%s en %s", type, target);
    report("VERDE", "MONTAJE", detail);
}

static void attach_console(void) {
    int fd = open("/dev/console", O_RDWR);
    if (fd < 0) return;
    for (int i = 0; i < 3; i++) (void)dup2(fd, i);
    if (fd > 2) close(fd);
}

static pid_t spawn_shell(void) {
    pid_t pid = fork();
    if (pid != 0) return pid;
    sigset_t none;
    sigemptyset(&none);
    sigprocmask(SIG_SETMASK, &none, NULL);
    /* Own session and controlling terminal, so Ctrl-C reaches the shell, not PID 1. */
    setsid();
    attach_console();
    (void)ioctl(STDIN_FILENO, TIOCSCTTY, 1);
    static char arg0[] = "-sh";
    static char home[] = "HOME=/root", path[] = "PATH=/bin:/sbin", term[] = "TERM=linux";
    char *const argv[] = {arg0, NULL};
    char *const envp[] = {home, path, term, NULL};
    if (chdir("/root") < 0 && chdir("/") < 0) report("AMARILLO", "DIRECTORIO", strerror(errno));
    execve("/bin/sh", argv, envp);
    report("ROJO", "SHELL-AUSENTE", strerror(errno));
    _exit(127);
}

static void shut_down(int sig) {
    /* The reboot magic values are unsigned bit patterns; reboot(2) takes them as int. */
    int how = (int)RB_POWER_OFF;
    const char *what = "apagado";
    if (sig == SIGTERM) { how = (int)RB_AUTOBOOT; what = "reinicio"; }
    else if (sig == SIGUSR1) { how = (int)RB_HALT_SYSTEM; what = "detencion"; }
    report("AMARILLO", "APAGADO", what);
    kill(-1, SIGTERM);
    sleep(1);
    kill(-1, SIGKILL);
    sync();
    reboot(how);
    report("ROJO", "APAGADO", strerror(errno));
}

static void wake(int sig) { (void)sig; }

int main(void) {
    if (getpid() != 1) {
        report("ROJO", "NO-PID1", "este programa solo arranca como init del kernel");
        return 1;
    }
    mount_fs("devtmpfs", "/dev", MS_NOSUID);
    attach_console();
    mount_fs("proc", "/proc", MS_NOSUID | MS_NODEV | MS_NOEXEC);
    mount_fs("sysfs", "/sys", MS_NOSUID | MS_NODEV | MS_NOEXEC);
    mount_fs("tmpfs", "/tmp", MS_NOSUID | MS_NODEV);
    mount_fs("tmpfs", "/run", MS_NOSUID | MS_NODEV);

    struct utsname u;
    char detail[200];
    if (uname(&u) == 0) snprintf(detail, sizeof(detail), "%s %s %s", u.sysname, u.release, u.machine);
    else snprintf(detail, sizeof(detail), "uname: %s", strerror(errno));
    emit("\nULTRA LinuX: INIT C READY\n");
    report("VERDE", "KERNEL", detail);

    /* Signals are taken synchronously: no handler code runs at arbitrary points. */
    sigset_t watched;
    sigemptyset(&watched);
    const int signals[] = {SIGCHLD, SIGUSR1, SIGUSR2, SIGTERM, SIGINT};
    struct sigaction sa;
    memset(&sa, 0, sizeof(sa));
    sa.sa_handler = wake;
    for (size_t i = 0; i < sizeof(signals) / sizeof(signals[0]); i++) {
        sigaddset(&watched, signals[i]);
        sigaction(signals[i], &sa, NULL);
    }
    sigprocmask(SIG_BLOCK, &watched, NULL);
    /* Ctrl-Alt-Del arrives as SIGINT instead of an instant reboot. */
    reboot(RB_DISABLE_CAD);

    pid_t shell = spawn_shell();
    time_t started = time(NULL);
    int fast_exits = 0;
    for (;;) {
        siginfo_t info;
        int sig = sigwaitinfo(&watched, &info);
        if (sig < 0) {
            if (errno != EINTR) report("ROJO", "SENALES", strerror(errno));
            continue;
        }
        if (sig != SIGCHLD) {
            shut_down(sig == SIGINT ? SIGTERM : sig);
            continue;
        }
        int status;
        pid_t dead;
        while ((dead = waitpid(-1, &status, WNOHANG)) > 0) {
            if (dead != shell) continue; /* Orphans are reaped here as well. */
            if (WIFSIGNALED(status)) snprintf(detail, sizeof(detail), "shell terminada por senal %d", WTERMSIG(status));
            else snprintf(detail, sizeof(detail), "shell salio con codigo %d", WEXITSTATUS(status));
            report("AMARILLO", "SHELL", detail);
            fast_exits = time(NULL) - started < RESPAWN_WINDOW_S ? fast_exits + 1 : 0;
            if (fast_exits >= RESPAWN_LIMIT) {
                report("ROJO", "SHELL-BUCLE", "la shell muere al arrancar; pausa antes de reintentar");
                sleep(RESPAWN_PAUSE_S);
                fast_exits = 0;
            }
            shell = spawn_shell();
            started = time(NULL);
        }
    }
}
