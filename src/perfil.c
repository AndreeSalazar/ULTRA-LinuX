/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * The generic kernel configures the PC's own version, and keeps a way back.
 *
 *   generic, first time: the CPU fits the tuned profile (AMD family 19h with
 *     AVX2: Zen 3 and Zen 4), ULTRA-PC.EFI and its system image are on BMO
 *     -> reuse or create the F11 entry "ULTRA LinuX (este PC)" and ask for it
 *     once with BootNext. State: probando.
 *   tuned kernel, while probando: it reached this point -> bueno.
 *   generic again, while probando and BootNext already used: the tuned kernel
 *     never said bueno -> fallido, and the entry goes if ULTRA created it.
 *
 * The state lives in /datos/perfil.estado. A file BMO\ULTRA\NO-AUTOCONFIGURAR
 * (A:\ULTRA\NO-AUTOCONFIGURAR from Windows) switches all of this off.
 */
#define _GNU_SOURCE
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <stdarg.h>
#include <sys/utsname.h>
#include <unistd.h>
#include "init.h"
/* Last: it poisons unsafe functions after the system headers are parsed. */
#include "guardian/ultra_guard.h"

#define STATE "/datos/perfil.estado"
#define TUNED_EFI "\\EFI\\ULTRA\\ULTRA-PC.EFI"
#define TUNED_TITLE "ULTRA LinuX (este PC)"

struct state {
    char estado[16]; /* probando, bueno, fallido or empty */
    int entrada;     /* Boot#### number, or -1 */
    bool creada;     /* ULTRA created it, so ULTRA may delete it */
};

static void say(const char *level, const char *fmt, ...) __attribute__((format(printf, 2, 3)));
static void say(const char *level, const char *fmt, ...) {
    char detail[320];
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(detail, sizeof(detail), fmt, ap);
    va_end(ap);
    report(level, "PERFIL", detail);
}

static bool read_small(const char *path, char *buf, size_t cap) {
    int fd = open(path, O_RDONLY | O_CLOEXEC);
    if (fd < 0) return false;
    ssize_t n = read(fd, buf, cap - 1);
    close(fd);
    if (n < 0) return false;
    buf[n] = 0;
    return true;
}

static void state_read(struct state *s) {
    char buf[256];
    s->estado[0] = 0;
    s->entrada = -1;
    s->creada = false;
    if (!read_small(STATE, buf, sizeof(buf))) return;
    char *save = NULL;
    for (char *line = strtok_r(buf, "\n", &save); line; line = strtok_r(NULL, "\n", &save)) {
        if (strncmp(line, "estado=", 7) == 0) ULTRA_COPY(s->estado, line + 7);
        else if (strncmp(line, "entrada=", 8) == 0) s->entrada = (int)strtol(line + 8, NULL, 16);
        else if (strcmp(line, "creada=si") == 0) s->creada = true;
    }
    if (s->entrada < 0 || s->entrada > 0xFFFF) s->entrada = -1;
}

static void state_write(const struct state *s) {
    char buf[128];
    int n = snprintf(buf, sizeof(buf), "estado=%s\nentrada=%04X\ncreada=%s\n", s->estado,
                     (unsigned)(s->entrada < 0 ? 0 : s->entrada), s->creada ? "si" : "no");
    int fd = n > 0 ? open(STATE ".nuevo", O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0644) : -1;
    if (fd < 0) return;
    bool ok = write(fd, buf, (size_t)n) == n && fsync(fd) == 0;
    close(fd);
    if (!ok || rename(STATE ".nuevo", STATE) < 0) say("AMARILLO", "no se pudo guardar " STATE);
}

/* AMD family 19h (Zen 3, Zen 4) with AVX2: what the pc profile was built for. */
static bool cpu_fits(char *why, size_t cap) {
    char buf[4096];
    if (!read_small("/proc/cpuinfo", buf, sizeof(buf))) return false;
    bool amd = strstr(buf, "AuthenticAMD") != NULL;
    const char *fam = strstr(buf, "cpu family");
    long family = fam ? strtol(fam + strcspn(fam, "0123456789"), NULL, 10) : -1;
    bool avx2 = strstr(buf, " avx2") != NULL;
    snprintf(why, cap, "%s, familia %ld%s", amd ? "AMD" : "no AMD", family, avx2 ? ", AVX2" : ", sin AVX2");
    return amd && family == 25 && avx2;
}

/* The tuned kernel and its own system image are installed on BMO. */
static bool tuned_installed(const char *dir) {
    char path[320];
    int n = snprintf(path, sizeof(path), "%s/../EFI/ULTRA/ULTRA-PC.EFI", dir);
    if (n < 0 || (size_t)n >= sizeof(path) || access(path, R_OK) < 0) return false;
    DIR *d = opendir(dir);
    if (!d) return false;
    bool found = false;
    struct dirent *e;
    while (!found && (e = readdir(d)) != NULL) found = strstr(e->d_name, "-pc-") && strstr(e->d_name, ".erofs");
    closedir(d);
    return found;
}

void choose_profile(bool data) {
    const char *dir = windows_dir();
    const char *dev = bmo_device();
    struct utsname u;
    if (!data || !dir || !dev || uname(&u) < 0) return;
    char off[320];
    int n = snprintf(off, sizeof(off), "%s/NO-AUTOCONFIGURAR", dir);
    if (n > 0 && (size_t)n < sizeof(off) && access(off, F_OK) == 0) {
        say("AMARILLO", "autoconfiguracion desactivada por A:\\ULTRA\\NO-AUTOCONFIGURAR");
        return;
    }
    struct state s;
    state_read(&s);
    bool tuned = strstr(u.release, "-pc-") != NULL;

    if (tuned) {
        if (strcmp(s.estado, "probando") == 0) {
            ULTRA_COPY(s.estado, "bueno");
            state_write(&s);
            say("VERDE", "tu version propia arranco bien: queda lista en F11 (Boot%04X)", (unsigned)s.entrada);
        }
        return;
    }

    /*
     * The state is settled first, whatever else changed: a test in flight must be judged
     * even if the tuned kernel is gone, because a missing kernel is exactly how it fails.
     */
    bool installed = tuned_installed(dir);
    if (s.estado[0] && !efi_ready()) {
        say("AMARILLO", "sin acceso a las variables EFI: no se puede revisar tu version propia");
        return;
    }
    if (strcmp(s.estado, "bueno") == 0 && !installed) {
        /* Uninstalled after it worked: its entry would point at nothing. */
        bool removed = s.creada && s.entrada >= 0 && efi_delete_entry(s.entrada);
        s.estado[0] = 0;
        state_write(&s);
        say("AMARILLO", "tu version propia ya no esta instalada en A:\\%s",
            s.creada ? (removed ? "; su entrada F11 se borro" : "; su entrada F11 no se pudo borrar") : "");
        return;
    }
    if (strcmp(s.estado, "bueno") == 0) {
        say("VERDE", "tu version propia esta lista: eligela en F11 (Boot%04X)", (unsigned)s.entrada);
        return;
    }
    if (strcmp(s.estado, "fallido") == 0) {
        say("AMARILLO", "tu version propia fallo antes; borra /datos/perfil.estado para reintentar");
        return;
    }
    if (strcmp(s.estado, "probando") == 0) {
        if (efi_boot_next_pending()) {
            say("AMARILLO", "prueba pendiente: el proximo reinicio arranca tu version propia (Boot%04X)", (unsigned)s.entrada);
            return;
        }
        /* BootNext was used, yet the tuned kernel never reached its init. */
        ULTRA_COPY(s.estado, "fallido");
        bool removed = s.creada && s.entrada >= 0 && efi_delete_entry(s.entrada);
        state_write(&s);
        say("ROJO", "tu version propia no llego a arrancar; se queda el generico%s",
            s.creada ? (removed ? " y su entrada F11 se borro" : "; su entrada F11 no se pudo borrar") : "");
        return;
    }

    /* First time: only for a CPU that fits and a tuned kernel that is really there. */
    char why[96];
    if (!cpu_fits(why, sizeof(why))) {
        say("AMARILLO", "este PC (%s) no encaja con la version Zen 3: el generico es su version", why);
        return;
    }
    if (!installed) {
        say("AMARILLO", "no hay version propia instalada en A:\\ (ULTRA-PC.EFI y su sistema)");
        return;
    }
    if (!efi_ready()) {
        say("AMARILLO", "sin acceso a las variables EFI: no se puede preparar la entrada F11");
        return;
    }
    /* Reuse an entry that already starts the tuned kernel, or create one. */
    int entry = efi_find_entry(dev, TUNED_EFI);
    s.creada = false;
    if (entry < 0) {
        entry = efi_create_entry(dev, TUNED_EFI, TUNED_TITLE);
        s.creada = entry >= 0;
    }
    if (entry < 0) {
        say("AMARILLO", "no se pudo preparar la entrada F11 de tu version propia");
        return;
    }
    s.entrada = entry;
    if (!efi_set_boot_next(entry)) {
        say("AMARILLO", "entrada Boot%04X lista, pero no se pudo pedir la prueba (BootNext)", (unsigned)entry);
        return;
    }
    ULTRA_COPY(s.estado, "probando");
    state_write(&s);
    say("VERDE", "%s tu version propia (%s): Boot%04X; reinicia (reboot) para probarla una vez",
        s.creada ? "preparada" : "encontrada", why, (unsigned)entry);
}
