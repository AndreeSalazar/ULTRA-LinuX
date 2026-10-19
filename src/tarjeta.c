/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * The ULTRA boot card: one framed box on the screen, after the hardware stage
 * and before the shells. Nothing on it is assumed; everything is read now:
 *   Equipo     CPU model, its temperature (k10temp, loaded by this boot) and RAM
 *   Sistema    kernel and how many modules this PC needed
 *   Tuberia    time up to the hardware stage as a bar, against the previous boot
 *   Guardianes CET, Landlock and seccomp asked to the kernel; FORTIFY from the build
 * The serial port gets the same facts as one plain TARJETA line, for the tests.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <inttypes.h>
#include <linux/landlock.h>
#include <sys/prctl.h>
#include <sys/syscall.h>
#include <sys/utsname.h>
#include <unistd.h>
#include "init.h"
/* Last: it poisons unsafe functions after the system headers are parsed. */
#include "guardian/ultra_guard.h"

/* Columns inside the frame; the screen of the PC has 240. */
enum { INNER = 60, BAR = 24, BAR_FULL_MS = 2500 };

/* /proc and /sys report size 0, so they are read in a loop, not by fstat. */
static bool read_text(const char *path, char *out, size_t cap) {
    int fd = open(path, O_RDONLY | O_CLOEXEC);
    if (fd < 0 || cap == 0) {
        if (fd >= 0) close(fd);
        return false;
    }
    size_t got = 0;
    while (got + 1 < cap) {
        ssize_t n = read(fd, out + got, cap - 1 - got);
        if (n < 0 && errno == EINTR) continue;
        if (n <= 0) break;
        got += (size_t)n;
    }
    close(fd);
    out[got] = 0;
    return got > 0;
}

/* The text after "key" up to the end of its line, without the separator. */
static bool line_value(const char *text, const char *key, char *out, size_t cap) {
    const char *p = strstr(text, key);
    if (!p) return false;
    p += strlen(key);
    while (*p == ' ' || *p == '\t' || *p == ':') p++;
    size_t n = strcspn(p, "\n");
    char tmp[128];
    snprintf(tmp, sizeof(tmp), "%.*s", n < sizeof(tmp) ? (int)n : (int)sizeof(tmp) - 1, p);
    return ultra_copy(out, cap, tmp);
}

static void cpu_model(char *out, size_t cap) {
    char buf[4096];
    if (!read_text("/proc/cpuinfo", buf, sizeof(buf)) || !line_value(buf, "model name", out, cap)) {
        ultra_copy(out, cap, "CPU desconocida");
        return;
    }
    char *tail = strstr(out, " Processor");
    if (tail) *tail = 0;
}

/* Degrees from the Ryzen's own sensor, if this boot loaded k10temp. */
static int cpu_temperature(void) {
    for (int i = 0; i < 16; i++) {
        char path[64], name[32], value[32];
        snprintf(path, sizeof(path), "/sys/class/hwmon/hwmon%d/name", i);
        if (!read_text(path, name, sizeof(name))) continue;
        if (strncmp(name, "k10temp", 7) != 0) continue;
        snprintf(path, sizeof(path), "/sys/class/hwmon/hwmon%d/temp1_input", i);
        if (!read_text(path, value, sizeof(value))) return -1;
        long milli = strtol(value, NULL, 10);
        return milli > 0 && milli < 150000 ? (int)(milli / 1000) : -1;
    }
    return -1;
}

static long ram_mib(void) {
    char buf[512], value[64];
    if (!read_text("/proc/meminfo", buf, sizeof(buf)) || !line_value(buf, "MemTotal", value, sizeof(value))) return -1;
    return strtol(value, NULL, 10) / 1024;
}

struct guardian {
    const char *name;
    bool on;
};

static void ask_guardians(struct guardian g[4]) {
    char status[4096] = "";
    read_text("/proc/self/status", status, sizeof(status));
    g[0] = (struct guardian){"CET", strstr(status, "x86_Thread_features:") && strstr(status, "shstk")};
    long abi = syscall(SYS_landlock_create_ruleset, NULL, 0, LANDLOCK_CREATE_RULESET_VERSION);
    g[1] = (struct guardian){"Landlock", abi >= 1};
    g[2] = (struct guardian){"seccomp", prctl(PR_GET_SECCOMP, 0, 0, 0, 0) >= 0};
#if defined(_FORTIFY_SOURCE) && _FORTIFY_SOURCE >= 3
    g[3] = (struct guardian){"FORTIFY", true};
#else
    g[3] = (struct guardian){"FORTIFY", false};
#endif
}

/* Screen columns of a string: escape sequences take none, a UTF-8 character takes one. */
static size_t columns(const char *s) {
    size_t n = 0;
    for (const unsigned char *p = (const unsigned char *)s; *p; p++) {
        if (*p == 0x1b) {
            while (*p && *p != 'm') p++;
            if (!*p) break;
            continue;
        }
        if ((*p & 0xC0) != 0x80) n++;
    }
    return n;
}

#define FRAME "\033[0;36m"
#define RESET "\033[0m"

/* Pads with `fill` until the line spans `width` screen columns. */
static bool pad(char *line, size_t cap, size_t width, const char *fill) {
    while (columns(line) < width)
        if (!ultra_append(line, cap, fill)) return false;
    return true;
}

/* │ label     content                                   │ */
static void row(const char *label, const char *content) {
    char line[768];
    if (!ULTRA_COPY(line, FRAME "│" RESET " \033[1;37m") || !ULTRA_APPEND(line, label) || !ULTRA_APPEND(line, RESET)) return;
    /* Padding by columns, not bytes: "Tubería" has an accent. */
    if (!pad(line, sizeof(line), 13, " ") || !ULTRA_APPEND(line, content)) return;
    if (pad(line, sizeof(line), INNER + 1, " ") && ULTRA_APPEND(line, FRAME "│" RESET "\n")) screen_write(line);
}

static void rule(const char *left, const char *title, const char *right) {
    char line[768];
    if (!ULTRA_COPY(line, FRAME) || !ULTRA_APPEND(line, left) || !ULTRA_APPEND(line, title)) return;
    if (pad(line, sizeof(line), INNER + 1, "─") && ULTRA_APPEND(line, right) && ULTRA_APPEND(line, RESET "\n"))
        screen_write(line);
}

void show_card(void) {
    char cpu[96], text[512], plain[384];
    cpu_model(cpu, sizeof(cpu));
    int temp = cpu_temperature();
    long ram = ram_mib();
    struct utsname u;
    if (uname(&u) < 0) ultra_copy(u.release, sizeof(u.release), "?");
    int64_t now, before;
    size_t modules, loaded;
    boot_measure(&now, &before, &modules, &loaded);
    struct guardian g[4];
    ask_guardians(g);
    bool all_on = true;
    for (size_t i = 0; i < ULTRA_LEN(g); i++) all_on = all_on && g[i].on;

    /* The plain line first: the serial port and the tests read this one. */
    char degrees[16] = "sin sensor";
    if (temp >= 0) snprintf(degrees, sizeof(degrees), "%d C", temp);
    int n = snprintf(plain, sizeof(plain), "%s | %s | %ld MiB | %s | %" PRId64 " ms (antes %" PRId64 ") | %s%s %s%s %s%s %s%s",
                     cpu, degrees, ram, u.release, now, before, g[0].on ? "" : "sin-", g[0].name, g[1].on ? "" : "sin-",
                     g[1].name, g[2].on ? "" : "sin-", g[2].name, g[3].on ? "" : "sin-", g[3].name);
    if (n > 0) report(all_on ? "VERDE" : "AMARILLO", "TARJETA", plain);
    if (!screen_available()) return;

    screen_write("\n");
    rule("┌─ ", "\033[1;36mULTRA LinuX " FRAME "· \033[1;35mBMO-X " FRAME, "┐");
    char temp_text[48] = "";
    if (temp >= 0)
        snprintf(temp_text, sizeof(temp_text), " · %s%d°C" RESET, temp < 60 ? "\033[1;32m" : temp < 80 ? "\033[1;33m" : "\033[1;31m", temp);
    snprintf(text, sizeof(text), "%.40s%s · %.1f GiB", cpu, temp_text, ram > 0 ? (double)ram / 1024.0 : 0.0);
    row("Equipo", text);
    /* The short name: 7.3.0-rc7-pc, without the -g<commit> the build appends. */
    char kernel[65];
    ultra_copy(kernel, sizeof(kernel), u.release);
    char *commit = strstr(kernel, "-g");
    if (commit) *commit = 0;
    snprintf(text, sizeof(text), "%s · %zu módulos de este PC (%zu cargados)", kernel, modules, loaded);
    row("Sistema", text);

    /* The pipe: the shorter the bar, the wider the pipe. */
    if (now >= 0) {
        int64_t cells = now * BAR / BAR_FULL_MS;
        if (cells < 1) cells = 1;
        if (cells > BAR) cells = BAR;
        bool better = before < 0 || now <= before;
        char bar[256];
        ULTRA_COPY(bar, better ? "\033[1;32m" : "\033[1;33m");
        for (int64_t i = 0; i < BAR; i++) ULTRA_APPEND(bar, i < cells ? "█" : "\033[0;90m░");
        ULTRA_APPEND(bar, RESET);
        char delta[96] = "";
        if (before >= 0 && now < before) snprintf(delta, sizeof(delta), "  \033[1;32m▼ %" PRId64 " ms" RESET, before - now);
        else if (before >= 0 && now > before) snprintf(delta, sizeof(delta), "  \033[1;33m▲ %" PRId64 " ms" RESET, now - before);
        else if (before < 0) ULTRA_COPY(delta, "  \033[0;90mprimera medida" RESET);
        snprintf(text, sizeof(text), "%s %" PRId64 " ms%s", bar, now, delta);
        row("Tubería", text);
    }

    text[0] = 0;
    for (size_t i = 0; i < ULTRA_LEN(g); i++) {
        char one[64];
        snprintf(one, sizeof(one), "%s■" RESET " %s  ", g[i].on ? "\033[1;32m" : "\033[1;31m", g[i].name);
        ULTRA_APPEND(text, one);
    }
    row("Guardianes", text);
    /* Which version this is, and where the PC's own version stands (perfil.c). */
    if (profile_status()[0]) row("Perfil", profile_status());
    rule("└", "", "┘");
    screen_write("\n");
}
