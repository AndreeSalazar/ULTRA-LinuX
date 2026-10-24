/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * ULTRA LinuX init: PID 1 mounts the base filesystems and the ULTRA partitions,
 * loads the PC's modules, then supervises one
 * BusyBox shell per active kernel console (screen and serial alike). Every outcome is reported as VERDE, AMARILLO or ROJO with a
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
#include "init.h"
/* Last: it poisons unsafe functions after the system headers are parsed. */
#include "guardian/ultra_guard.h"

enum { RESPAWN_WINDOW_S = 2, RESPAWN_LIMIT = 5, RESPAWN_PAUSE_S = 5, MAX_CONSOLES = 4 };

struct console {
    char dev[32];
    pid_t shell;
    time_t started;
    int fast_exits;
};
static struct console consoles[MAX_CONSOLES];
static size_t nconsoles;

/*
 * /dev/console is the last console on the command line: the serial port the
 * tests read. The screen of a real PC is tty1, so every report goes there too.
 */
static int screen_fd = -1;

static void write_all_fd(int fd, const char *s) {
    size_t left = strlen(s);
    while (left > 0) {
        ssize_t n = write(fd, s, left);
        if (n < 0) {
            if (errno == EINTR) continue;
            return;
        }
        s += n;
        left -= (size_t)n;
    }
}

static void emit(const char *s) {
    write_all_fd(STDOUT_FILENO, s);
    if (screen_fd >= 0) write_all_fd(screen_fd, s);
}

static void open_screen(void) {
    struct stat out, tty;
    int fd = open("/dev/tty1", O_WRONLY | O_NOCTTY | O_CLOEXEC);
    if (fd < 0) return;
    /* Already the console: one copy is enough. */
    if (fstat(STDOUT_FILENO, &out) == 0 && fstat(fd, &tty) == 0 && out.st_rdev == tty.st_rdev) {
        close(fd);
        return;
    }
    screen_fd = fd;
}

void report(const char *level, const char *code, const char *detail) {
    char line[384];
    int n = snprintf(line, sizeof(line), "%s [%s]: %s\n", level, code, detail);
    if (n < 0) return;
    /* Truncation is visible, not silent. */
    if ((size_t)n >= sizeof(line)) memcpy(line + sizeof(line) - 5, "...\n", 5);
    write_all_fd(STDOUT_FILENO, line);
    if (screen_fd < 0) return;
    /* Colour only on the screen: the serial port stays plain text for the tests. */
    const char *color = strcmp(level, "VERDE") == 0 ? "\033[1;32m" : strcmp(level, "AMARILLO") == 0 ? "\033[1;33m" : "\033[1;31m";
    char shown[416];
    n = snprintf(shown, sizeof(shown), "%s%s\033[0m [\033[1m%s\033[0m]: %s\n", color, level, code, detail);
    if (n < 0) return;
    if ((size_t)n >= sizeof(shown)) memcpy(shown + sizeof(shown) - 5, "...\n", 5);
    write_all_fd(screen_fd, shown);
}

/* Who is behind ULTRA, on every boot. Plain on the serial port, in colour on the screen. */
static void banner(void) {
    write_all_fd(STDOUT_FILENO, "ULTRA LinuX - Influencia BMO-X\n"
                                "Eddi Salazar: creador de un sistema propio, sin ser un OS\n\n");
    if (screen_fd < 0) return;
    /* The name in a gradient, letter by letter: cyan, blue, magenta and back. */
    static const char name[] = "ULTRA LinuX";
    /* Bright shades only: the dark ones vanish on a black screen. */
    static const char *const shades[] = {"\033[1;36m", "\033[1;36m", "\033[0;36m", "\033[1;34m", "\033[1;34m",
                                         "\033[1;35m", "\033[1;35m", "\033[1;34m", "\033[0;36m", "\033[1;36m", "\033[1;37m"};
    char line[256] = "";
    for (size_t i = 0; i + 1 < sizeof(name); i++) {
        char letter[2] = {name[i], 0};
        if (!ULTRA_APPEND(line, shades[i % ULTRA_LEN(shades)]) || !ULTRA_APPEND(line, letter)) return;
    }
    if (!ULTRA_APPEND(line, "\033[0m - Influencia \033[1;35mBMO-X\033[0m\n")) return;
    write_all_fd(screen_fd, line);
    write_all_fd(screen_fd, "\033[1;37mEddi Salazar\033[0m: creador de un sistema propio, sin ser un OS\n\n");
}

void screen_write(const char *s) {
    if (screen_fd >= 0) write_all_fd(screen_fd, s);
}

bool screen_available(void) { return screen_fd >= 0; }

static void mount_fs_data(const char *type, const char *target, unsigned long flags, const char *data) {
    char detail[160];
    if (mkdir(target, 0755) < 0 && errno != EEXIST) {
        snprintf(detail, sizeof(detail), "mkdir %s: %s", target, strerror(errno));
        report("ROJO", "MONTAJE", detail);
        return;
    }
    /* devtmpfs may already be mounted by CONFIG_DEVTMPFS_MOUNT. */
    if (mount(type, target, type, flags, data) < 0 && errno != EBUSY) {
        snprintf(detail, sizeof(detail), "%s en %s: %s", type, target, strerror(errno));
        report("ROJO", "MONTAJE", detail);
        return;
    }
    snprintf(detail, sizeof(detail), "%s en %s", type, target);
    report("VERDE", "MONTAJE", detail);
}

static void mount_fs(const char *type, const char *target, unsigned long flags) {
    mount_fs_data(type, target, flags, NULL);
}

/* A console descriptor that landed on 0..2 stays open for the life of the process. */
static int console_fd = -1;

static void attach_tty(const char *dev) {
    int fd = open(dev, O_RDWR);
    if (fd < 0) return;
    for (int i = 0; i < 3; i++) (void)dup2(fd, i);
    if (fd > 2) close(fd);
    else console_fd = fd;
}

static void attach_console(void) { attach_tty("/dev/console"); }

/*
 * One shell per console the kernel writes to, read from sysfs. Without a boot
 * loader the command line is built in, so the same image must serve the
 * screen of a real PC and the serial port of the tests.
 */
static void find_consoles(void) {
    char buf[128] = {0};
    int fd = open("/sys/class/tty/console/active", O_RDONLY | O_CLOEXEC);
    ssize_t n = fd >= 0 ? read(fd, buf, sizeof(buf) - 1) : -1;
    if (fd >= 0) close(fd);
    if (n > 0) {
        buf[(size_t)n] = 0;
        char *save = NULL;
        for (char *tok = strtok_r(buf, " \n", &save); tok && nconsoles < MAX_CONSOLES; tok = strtok_r(NULL, " \n", &save)) {
            /* tty0 means "whichever virtual terminal is current": pin the shell to tty1. */
            const char *name = strcmp(tok, "tty0") ? tok : "tty1";
            struct console *c = &consoles[nconsoles];
            if (ULTRA_COPY(c->dev, "/dev/") && ULTRA_APPEND(c->dev, name)) nconsoles++;
            else report("AMARILLO", "CONSOLA", "nombre de consola demasiado largo; se omite");
        }
    }
    if (nconsoles == 0) {
        report("AMARILLO", "CONSOLA", "sin lista de consolas activas; una sola shell en /dev/console");
        if (ULTRA_COPY(consoles[0].dev, "/dev/console")) nconsoles = 1;
    }
}

static pid_t spawn_shell(const char *dev) {
    pid_t pid = fork();
    if (pid != 0) return pid;
    sigset_t none;
    sigemptyset(&none);
    sigprocmask(SIG_SETMASK, &none, NULL);
    /* Own session and controlling terminal, so Ctrl-C reaches the shell, not PID 1. */
    setsid();
    attach_tty(dev);
    (void)ioctl(STDIN_FILENO, TIOCSCTTY, 1);
    static char arg0[] = "-sh";
    static char home[] = "HOME=/root", path[] = "PATH=/bin:/sbin", term[] = "TERM=linux";
    static char tunables[160];
    char *const argv[] = {arg0, NULL};
    char *envp[] = {home, path, term, NULL, NULL};
    /* The kernel command line hands GLIBC_TUNABLES to PID 1; pass it on so CET reaches every program. */
    const char *t = getenv("GLIBC_TUNABLES");
    if (t && ULTRA_COPY(tunables, "GLIBC_TUNABLES=") && ULTRA_APPEND(tunables, t)) envp[3] = tunables;
    else if (t) report("AMARILLO", "CET", "GLIBC_TUNABLES demasiado largo; la shell arranca sin pila sombra");
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
    release_system();
    unmount_data();
    reboot(how);
    report("ROJO", "APAGADO", strerror(errno));
}

/* A clean restart before any shell exists: data unmounted, then the firmware runs again. */
void restart_now(void) {
    report("AMARILLO", "APAGADO", "reinicio hacia tu version propia");
    sync();
    release_system();
    unmount_data();
    reboot((int)RB_AUTOBOOT);
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
    open_screen();
    /* If PID 1 itself trips a guardian, the flag prints before the kernel panics. */
    ultra_guard_install("init");
    mount_fs("proc", "/proc", MS_NOSUID | MS_NODEV | MS_NOEXEC);
    mount_fs("sysfs", "/sys", MS_NOSUID | MS_NODEV | MS_NOEXEC);
    mount_fs("tmpfs", "/tmp", MS_NOSUID | MS_NODEV);
    mount_fs("tmpfs", "/run", MS_NOSUID | MS_NODEV);
    /* Pseudo-terminals and shared memory: a terminal like foot and every Wayland client need them. */
    mount_fs_data("devpts", "/dev/pts", MS_NOSUID | MS_NOEXEC, "gid=5,mode=620,ptmxmode=666");
    mount_fs_data("tmpfs", "/dev/shm", MS_NOSUID | MS_NODEV, "mode=1777");
    /* A name for the machine: without it, shells and Arch's prompt show "(none)". */
    if (sethostname("ultra", 5) < 0) report("AMARILLO", "NOMBRE", strerror(errno));

    struct utsname u;
    char detail[320];
    if (uname(&u) == 0) snprintf(detail, sizeof(detail), "%s %s %s", u.sysname, u.release, u.machine);
    else snprintf(detail, sizeof(detail), "uname: %s", strerror(errno));
    emit("\nULTRA LinuX: INIT C READY\n");
    banner();
    report("VERDE", "KERNEL", detail);
    bool data = mount_partitions();
    prepare_hardware(data);
    /* The profile first, so its state shows on the card. */
    choose_profile(data);
    /* After the profile: a jump to the tuned kernel reboots before anything else is mounted. */
    prepare_system(data);
    /* The author's Tux, animated for about 1.2 s; the screen underneath is put back. */
    show_intro();
    show_card();
    show_logo();

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

    find_consoles();
    for (size_t i = 0; i < nconsoles; i++) {
        consoles[i].shell = spawn_shell(consoles[i].dev);
        consoles[i].started = time(NULL);
        snprintf(detail, sizeof(detail), "shell en %s", consoles[i].dev);
        report("VERDE", "CONSOLA", detail);
    }
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
            /* Orphans are reaped here too; a guardian death is flagged even for them. */
            int sig_dead = WIFSIGNALED(status) ? WTERMSIG(status) : 0;
            bool guardian = sig_dead == SIGILL || sig_dead == SIGSEGV || sig_dead == SIGBUS ||
                            sig_dead == SIGFPE || sig_dead == SIGABRT || sig_dead == SIGSYS;
            struct console *c = NULL;
            for (size_t i = 0; i < nconsoles; i++)
                if (consoles[i].shell == dead) c = &consoles[i];
            const char *who = c ? c->dev : "huerfano";
            if (sig_dead) snprintf(detail, sizeof(detail), "%s pid %d senal %d: %s", who, (int)dead, sig_dead, ultra_explain_signal(sig_dead));
            else snprintf(detail, sizeof(detail), "%s pid %d salio con codigo %d", who, (int)dead, WEXITSTATUS(status));
            if (guardian || c) report(guardian ? "ROJO" : "AMARILLO", guardian ? "GUARDIAN" : "SHELL", detail);
            if (!c) continue;
            c->fast_exits = time(NULL) - c->started < RESPAWN_WINDOW_S ? c->fast_exits + 1 : 0;
            if (c->fast_exits >= RESPAWN_LIMIT) {
                report("ROJO", "SHELL-BUCLE", "la shell muere al arrancar; pausa antes de reintentar");
                sleep(RESPAWN_PAUSE_S);
                c->fast_exits = 0;
            }
            c->shell = spawn_shell(c->dev);
            c->started = time(NULL);
        }
    }
}
