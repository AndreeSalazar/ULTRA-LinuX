/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * Hardware detection with memory. First boot on a PC: every PCI and USB
 * modalias is matched against modules.alias, those modules are loaded and the
 * result is written to /datos/PC.configuración. Later boots read that file and
 * load only its modules, skipping the match. A PCI fingerprint in the file
 * notices a different PC (the USB stick moved) and detects again.
 *
 * The file only chooses: modules are loaded from ULTRA-SISTEMA-A, which is
 * read-only, so a tampered file cannot bring in code of its own.
 */
#define _GNU_SOURCE
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <fnmatch.h>
#include <inttypes.h>
#include <stdarg.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <sys/utsname.h>
#include <time.h>
#include <unistd.h>
#include "init.h"
/* Last: it poisons unsafe functions after the system headers are parsed. */
#include "guardian/ultra_guard.h"

#define CONFIG_PATH "/datos/PC.configuración"
#define CONFIG_TMP "/datos/.PC.configuración.nuevo"
#define BOOT_LOG "/datos/arranques.log"

enum { MAX_MODULES = 64, MAX_TRIED = 128, MAX_DEVICES = 256, NAME_LEN = 64, DEP_DEPTH = 8 };

struct device {
    char bus[4], id[64], alias[256], what[96], driver[NAME_LEN], modules[160];
};
static struct device devices[MAX_DEVICES];
static size_t ndevices;

/* The modules this PC needs, in the order they were found. */
static char wanted[MAX_MODULES][NAME_LEN];
static size_t nwanted;

struct dep {
    char name[NAME_LEN];
    const char *path, *deps;
};
static struct dep *deps;
static size_t ndeps;
static char tried[MAX_TRIED][NAME_LEN];
static size_t ntried, nloaded, nfailed;
static char moddir[192];

static int64_t now_us(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (int64_t)ts.tv_sec * 1000000 + ts.tv_nsec / 1000;
}

static void say(const char *level, const char *fmt, ...) __attribute__((format(printf, 2, 3)));
static void say(const char *level, const char *fmt, ...) {
    char detail[320];
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(detail, sizeof(detail), fmt, ap);
    va_end(ap);
    report(level, "HARDWARE", detail);
}

/* A whole regular file in memory, NUL-terminated. */
static char *slurp(const char *path) {
    int fd = open(path, O_RDONLY | O_CLOEXEC);
    if (fd < 0) return NULL;
    struct stat st;
    char *buf = NULL;
    if (fstat(fd, &st) == 0 && st.st_size >= 0 && st.st_size < (64L << 20)) {
        size_t size = (size_t)st.st_size, got = 0;
        buf = malloc(size + 1);
        while (buf && got < size) {
            ssize_t n = read(fd, buf + got, size - got);
            if (n < 0 && errno == EINTR) continue;
            if (n <= 0) break;
            got += (size_t)n;
        }
        if (buf) buf[got] = 0;
    }
    close(fd);
    return buf;
}

/* A short sysfs attribute without its trailing newline. */
static bool read_attr(const char *dir, const char *attr, char *out, size_t cap) {
    char path[512];
    int len = snprintf(path, sizeof(path), "%s/%s", dir, attr);
    if (len < 0 || (size_t)len >= sizeof(path) || cap == 0) return false;
    int fd = open(path, O_RDONLY | O_CLOEXEC);
    if (fd < 0) return false;
    ssize_t n = read(fd, out, cap - 1);
    close(fd);
    if (n <= 0) return false;
    out[n] = 0;
    char *nl = strchr(out, '\n');
    if (nl) *nl = 0;
    return true;
}

/* snd-hda-intel.ko and snd_hda_intel are the same module to the kernel. */
static bool module_name(const char *path, char *out, size_t cap) {
    const char *base = strrchr(path, '/');
    base = base ? base + 1 : path;
    if (!ultra_copy(out, cap, base)) return false;
    char *ko = strstr(out, ".ko");
    if (ko) *ko = 0;
    for (char *c = out; *c; c++)
        if (*c == '-') *c = '_';
    return out[0] != 0;
}

/* The configuration file is data written on a writable disk: names are checked, not trusted. */
static bool valid_name(const char *name) {
    size_t n = strlen(name);
    if (n == 0 || n >= NAME_LEN) return false;
    for (const char *c = name; *c; c++)
        if (!((*c >= 'a' && *c <= 'z') || (*c >= '0' && *c <= '9') || *c == '_')) return false;
    return true;
}

static void want(const char *name) {
    for (size_t i = 0; i < nwanted; i++)
        if (strcmp(wanted[i], name) == 0) return;
    if (nwanted == MAX_MODULES) {
        say("AMARILLO", "mas de %d modulos; se omite %s", MAX_MODULES, name);
        return;
    }
    if (ultra_copy(wanted[nwanted], NAME_LEN, name)) nwanted++;
}

static const struct { const char *code, *what; } vendors[] = {
    {"0x1022", "AMD"}, {"0x10de", "NVIDIA"}, {"0x10ec", "Realtek"}, {"0x8086", "Intel"},
    {"0x1b21", "ASMedia"}, {"0x1af4", "virtio"}, {"0x1234", "QEMU"}, {"0x1b36", "QEMU"},
};
static const struct { const char *code, *what; } classes[] = {
    {"0x0300", "pantalla"}, {"0x0302", "grafica 3D"}, {"0x0403", "sonido HD"}, {"0x0401", "sonido"},
    {"0x0200", "red Ethernet"}, {"0x0280", "red inalambrica"}, {"0x0106", "SATA AHCI"}, {"0x0108", "NVMe"},
    {"0x0101", "IDE"}, {"0x0100", "SCSI"}, {"0x0c03", "USB"}, {"0x0c05", "SMBus"}, {"0x0600", "puente de host"},
    {"0x0601", "puente ISA"}, {"0x0604", "puente PCI"}, {"0x1300", "instrumentacion"}, {"0x1080", "cifrado"},
    {"0x0800", "sistema"}, {"0x0880", "sistema"}, {"0x00ff", "sin clase"},
};

static void describe_pci(const char *dir, struct device *d) {
    char vendor[16] = "", product[16] = "", class[16] = "";
    read_attr(dir, "vendor", vendor, sizeof(vendor));
    read_attr(dir, "device", product, sizeof(product));
    read_attr(dir, "class", class, sizeof(class));
    const char *who = "", *what = "otro";
    for (size_t i = 0; i < ULTRA_LEN(vendors); i++)
        if (strcmp(vendor, vendors[i].code) == 0) who = vendors[i].what;
    for (size_t i = 0; i < ULTRA_LEN(classes); i++)
        if (strncmp(class, classes[i].code, 6) == 0) what = classes[i].what;
    snprintf(d->what, sizeof(d->what), "%s%s%s:%s %s", who, *who ? " " : "",
             vendor[0] ? vendor + 2 : "?", product[0] ? product + 2 : "?", what);
}

/*
 * Every device of a bus that publishes a modalias, in name order so the
 * fingerprint does not depend on directory order. details: name and driver too.
 */
static int visible(const struct dirent *e) { return e->d_name[0] != '.'; }

static void scan_bus(const char *bus, bool details) {
    char root[64], dir[384];
    snprintf(root, sizeof(root), "/sys/bus/%s/devices", bus);
    struct dirent **list = NULL;
    int n = scandir(root, &list, visible, alphasort);
    if (n < 0) return;
    for (int i = 0; i < n; i++) {
        int len = snprintf(dir, sizeof(dir), "%s/%s", root, list[i]->d_name);
        struct device *d = ndevices < MAX_DEVICES ? &devices[ndevices] : NULL;
        if (d && len > 0 && (size_t)len < sizeof(dir) &&
            read_attr(dir, "modalias", d->alias, sizeof(d->alias)) && ultra_copy(d->bus, sizeof(d->bus), bus) &&
            ultra_copy(d->id, sizeof(d->id), list[i]->d_name)) {
            d->what[0] = d->driver[0] = d->modules[0] = 0;
            if (details && strcmp(bus, "pci") == 0) describe_pci(dir, d);
            /* An interface has no product string; its parent device does. */
            else if (details && !read_attr(dir, "product", d->what, sizeof(d->what)))
                read_attr(dir, "../product", d->what, sizeof(d->what));
            ndevices++;
        }
        free(list[i]);
    }
    free(list);
}

static void read_drivers(void) {
    char path[384], link[256];
    for (size_t i = 0; i < ndevices; i++) {
        struct device *d = &devices[i];
        d->driver[0] = 0;
        int len = snprintf(path, sizeof(path), "/sys/bus/%s/devices/%s/driver", d->bus, d->id);
        if (len < 0 || (size_t)len >= sizeof(path)) continue;
        ssize_t n = readlink(path, link, sizeof(link) - 1);
        if (n <= 0) continue;
        link[n] = 0;
        const char *base = strrchr(link, '/');
        ultra_copy(d->driver, sizeof(d->driver), base ? base + 1 : link);
    }
}

/* FNV-1a over the PCI devices: a different board, card or slot gives another value. */
static uint64_t pci_fingerprint(void) {
    uint64_t h = 0xcbf29ce484222325ULL;
    for (size_t i = 0; i < ndevices; i++) {
        if (strcmp(devices[i].bus, "pci") != 0) continue;
        for (const char *s = devices[i].id; *s; s++) h = (h ^ (unsigned char)*s) * 0x100000001b3ULL;
        for (const char *s = devices[i].alias; *s; s++) h = (h ^ (unsigned char)*s) * 0x100000001b3ULL;
    }
    return h;
}

/* Lines of a file buffer, split in place. */
static char **split_lines(char *buf, size_t *count) {
    size_t n = 1;
    for (const char *c = buf; *c; c++) n += *c == '\n';
    char **lines = calloc(n, sizeof(*lines));
    if (!lines) return NULL;
    size_t k = 0;
    char *save = NULL;
    for (char *line = strtok_r(buf, "\n", &save); line && k < n; line = strtok_r(NULL, "\n", &save)) lines[k++] = line;
    *count = k;
    return lines;
}

/* Exact membership in a comma-separated list: snd must not match snd_hda_intel. */
static bool in_list(const char *list, const char *name) {
    size_t n = strlen(name);
    for (const char *p = list; (p = strstr(p, name)) != NULL; p += n)
        if ((p == list || p[-1] == ',') && (p[n] == ',' || p[n] == 0)) return true;
    return false;
}

static void match_aliases(void) {
    char path[256];
    snprintf(path, sizeof(path), "%s/modules.alias", moddir);
    char *buf = slurp(path);
    size_t n = 0;
    char **lines = buf ? split_lines(buf, &n) : NULL;
    if (!lines) {
        say("ROJO", "no se pudo leer %s", path);
        free(buf);
        return;
    }
    for (size_t i = 0; i < n; i++) {
        /* alias <pattern> <module> */
        if (strncmp(lines[i], "alias ", 6) != 0) continue;
        char *pattern = lines[i] + 6, *space = strchr(pattern, ' ');
        if (!space) continue;
        *space = 0;
        const char *module = space + 1;
        for (size_t j = 0; j < ndevices; j++) {
            struct device *d = &devices[j];
            if (fnmatch(pattern, d->alias, 0) != 0) continue;
            want(module);
            if (!in_list(d->modules, module) &&
                !((d->modules[0] == 0 || ultra_append(d->modules, sizeof(d->modules), ",")) &&
                  ultra_append(d->modules, sizeof(d->modules), module)))
                say("AMARILLO", "%s: lista de modulos recortada", d->id);
        }
    }
    free(lines);
    free(buf);
}

static const struct dep *find_dep(const char *name) {
    for (size_t i = 0; i < ndeps; i++)
        if (strcmp(deps[i].name, name) == 0) return &deps[i];
    return NULL;
}

/* Dependencies first, then the module, straight from its file with finit_module. */
static bool load_module(const char *name, unsigned depth) {
    if (depth > DEP_DEPTH) {
        say("ROJO", "%s: dependencias demasiado profundas", name);
        return false;
    }
    for (size_t i = 0; i < ntried; i++)
        if (strcmp(tried[i], name) == 0) return true;
    if (ntried < MAX_TRIED && ultra_copy(tried[ntried], NAME_LEN, name)) ntried++;
    char path[512];
    /* Already loaded, or built into the kernel. */
    snprintf(path, sizeof(path), "/sys/module/%s", name);
    if (access(path, F_OK) == 0) return true;
    const struct dep *d = find_dep(name);
    if (!d) {
        say("AMARILLO", "%s no esta en modules.dep de este kernel", name);
        nfailed++;
        return false;
    }
    char list[1024], dep_name[NAME_LEN];
    if (!ULTRA_COPY(list, d->deps)) {
        say("ROJO", "%s: lista de dependencias demasiado larga", name);
        nfailed++;
        return false;
    }
    char *save = NULL;
    for (char *tok = strtok_r(list, " ", &save); tok; tok = strtok_r(NULL, " ", &save))
        if (!module_name(tok, dep_name, sizeof(dep_name)) || !load_module(dep_name, depth + 1)) {
            say("ROJO", "%s: fallo su dependencia %s", name, tok);
            nfailed++;
            return false;
        }
    int len = snprintf(path, sizeof(path), "%s/%s", moddir, d->path);
    int fd = len > 0 && (size_t)len < sizeof(path) ? open(path, O_RDONLY | O_CLOEXEC) : -1;
    if (fd < 0) {
        say("ROJO", "%s: %s", path, strerror(errno));
        nfailed++;
        return false;
    }
    long r = syscall(SYS_finit_module, fd, "", 0);
    int err = errno;
    close(fd);
    if (r != 0 && err != EEXIST) {
        say("ROJO", "cargar %s: %s", name, strerror(err));
        nfailed++;
        return false;
    }
    nloaded++;
    return true;
}

static void load_wanted(void) {
    char path[256];
    snprintf(path, sizeof(path), "%s/modules.dep", moddir);
    char *buf = slurp(path);
    size_t n = 0;
    char **lines = buf ? split_lines(buf, &n) : NULL;
    deps = lines ? calloc(n ? n : 1, sizeof(*deps)) : NULL;
    if (!deps) {
        say("ROJO", "no se pudo leer %s", path);
        free(lines);
        free(buf);
        return;
    }
    ndeps = 0;
    for (size_t i = 0; i < n; i++) {
        /* kernel/sound/hda/controllers/snd-hda-intel.ko: kernel/sound/... */
        char *colon = strchr(lines[i], ':');
        if (!colon) continue;
        *colon = 0;
        struct dep *d = &deps[ndeps];
        d->path = lines[i];
        d->deps = colon + 1;
        if (module_name(d->path, d->name, sizeof(d->name))) ndeps++;
    }
    ntried = nloaded = nfailed = 0;
    for (size_t i = 0; i < nwanted; i++) load_module(wanted[i], 0);
    free(deps);
    deps = NULL;
    ndeps = 0;
    free(lines);
    free(buf);
}

struct saved {
    char kernel[96];
    uint64_t fingerprint;
    int64_t detect_us, load_us;
};

/* Reads PC.configuración; the module list goes straight into wanted[]. */
static bool read_config(struct saved *s) {
    char *buf = slurp(CONFIG_PATH);
    if (!buf) return false;
    bool modules = false, ok = true;
    s->kernel[0] = 0;
    s->fingerprint = 0;
    s->detect_us = s->load_us = -1;
    nwanted = 0;
    char *save = NULL;
    for (char *line = strtok_r(buf, "\n", &save); line; line = strtok_r(NULL, "\n", &save)) {
        if (line[0] == '#') continue;
        if (line[0] == '[') { modules = strcmp(line, "[modulos]") == 0; continue; }
        if (modules) {
            if (valid_name(line)) want(line);
            else ok = false;
        } else if (strncmp(line, "kernel=", 7) == 0) ok = ok && ULTRA_COPY(s->kernel, line + 7);
        else if (strncmp(line, "huella_pci=", 11) == 0) s->fingerprint = strtoull(line + 11, NULL, 16);
        else if (strncmp(line, "deteccion_us=", 13) == 0) s->detect_us = strtoll(line + 13, NULL, 10);
        else if (strncmp(line, "carga_us=", 9) == 0) s->load_us = strtoll(line + 9, NULL, 10);
    }
    free(buf);
    if (!ok) say("AMARILLO", "PC.configuracion tiene lineas invalidas; se detecta de nuevo");
    return ok && s->kernel[0] && s->detect_us >= 0 && s->load_us >= 0;
}

static char out[32768];
static size_t outlen;
static bool out_full;
static void put(const char *fmt, ...) __attribute__((format(printf, 1, 2)));
static void put(const char *fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    int n = vsnprintf(out + outlen, sizeof(out) - outlen, fmt, ap);
    va_end(ap);
    if (n < 0 || (size_t)n >= sizeof(out) - outlen) out_full = true;
    else outlen += (size_t)n;
}

static bool write_all(int fd, const char *buf, size_t len) {
    while (len > 0) {
        ssize_t n = write(fd, buf, len);
        if (n < 0 && errno == EINTR) continue;
        if (n <= 0) return false;
        buf += n;
        len -= (size_t)n;
    }
    return true;
}

/* Written beside the old file and renamed over it: a power cut leaves the old or the new, never half. */
static bool write_config(const char *kernel, uint64_t fingerprint, int64_t detect_us, int64_t load_us) {
    outlen = 0;
    out_full = false;
    put("# ULTRA LinuX - PC.configuración\n"
        "# Escrito por el init en el primer arranque en este PC. Borrarlo fuerza otra deteccion.\n"
        "# Solo elige modulos: el codigo sale de ULTRA-SISTEMA-A, que es de solo lectura.\n"
        "kernel=%s\nhuella_pci=%016" PRIx64 "\ndeteccion_us=%" PRId64 "\ncarga_us=%" PRId64 "\n\n[componentes]\n",
        kernel, fingerprint, detect_us, load_us);
    for (size_t i = 0; i < ndevices; i++) {
        const struct device *d = &devices[i];
        put("%s %s | %s | driver=%s | modulos=%s | %s\n", d->bus, d->id, d->what[0] ? d->what : "-",
            d->driver[0] ? d->driver : "-", d->modules[0] ? d->modules : "-", d->alias);
    }
    put("\n[modulos]\n");
    for (size_t i = 0; i < nwanted; i++) put("%s\n", wanted[i]);
    if (out_full) {
        say("ROJO", "PC.configuracion no cabe en %zu bytes; no se guarda", sizeof(out));
        return false;
    }
    int fd = open(CONFIG_TMP, O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0644);
    if (fd < 0) {
        say("ROJO", "%s: %s", CONFIG_TMP, strerror(errno));
        return false;
    }
    bool ok = write_all(fd, out, outlen) && fsync(fd) == 0;
    int err = errno;
    close(fd);
    if (!ok || rename(CONFIG_TMP, CONFIG_PATH) < 0) {
        say("ROJO", "guardar PC.configuracion: %s", strerror(ok ? errno : err));
        unlink(CONFIG_TMP);
        return false;
    }
    int dir = open("/datos", O_RDONLY | O_DIRECTORY | O_CLOEXEC);
    if (dir >= 0) {
        fsync(dir);
        close(dir);
    }
    return true;
}

/* One line per boot, so the gain can be compared across many boots. */
static void log_boot(const char *line) {
    int fd = open(BOOT_LOG, O_WRONLY | O_CREAT | O_APPEND | O_CLOEXEC, 0644);
    if (fd < 0) return;
    if (!write_all(fd, line, strlen(line)) || fsync(fd) < 0) say("AMARILLO", "no se pudo anotar en %s", BOOT_LOG);
    close(fd);
}

/*
 * Windows cannot read ext4: when ULTRA runs from BMO (A:\), a copy of the
 * results goes beside its files, with plain ASCII names.
 */
static void copy_for_windows(void) {
    const char *dir = windows_dir();
    if (!dir) return;
    const char *const pairs[][2] = {{CONFIG_PATH, "PC-configuracion.txt"}, {BOOT_LOG, "arranques.txt"}};
    for (size_t i = 0; i < ULTRA_LEN(pairs); i++) {
        char dst[256];
        char *buf = slurp(pairs[i][0]);
        int len = snprintf(dst, sizeof(dst), "%s/%s", dir, pairs[i][1]);
        int fd = buf && len > 0 && (size_t)len < sizeof(dst) ? open(dst, O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0644) : -1;
        if (fd >= 0) {
            if (!write_all(fd, buf, strlen(buf)) || fsync(fd) < 0) say("AMARILLO", "no se pudo copiar a %s", dst);
            close(fd);
        } else if (buf) say("AMARILLO", "no se pudo crear %s/%s", dir, pairs[i][1]);
        free(buf);
    }
}

void prepare_hardware(bool data) {
    struct utsname u;
    char line[384];
    if (uname(&u) < 0) {
        say("ROJO", "uname: %s", strerror(errno));
        return;
    }
    int len = snprintf(moddir, sizeof(moddir), "/sistema/lib/modules/%s", u.release);
    if (len < 0 || (size_t)len >= sizeof(moddir) || access(moddir, R_OK) < 0) {
        say("AMARILLO", "sin modulos para el kernel %s en ULTRA-SISTEMA-A; no se carga ninguno", u.release);
        return;
    }
    /* Guided boot: the saved list, if this is the same kernel on the same PC. */
    struct saved s;
    int64_t t0 = now_us();
    const char *why = "no hay /datos: la deteccion se repetira en cada arranque";
    if (data && read_config(&s)) {
        ndevices = 0;
        scan_bus("pci", false);
        if (strcmp(s.kernel, u.release) != 0) why = "PC.configuracion es de otro kernel";
        else if (s.fingerprint != pci_fingerprint()) why = "la huella PCI cambio: otro PC u otra tarjeta";
        else {
            int64_t t1 = now_us();
            load_wanted();
            int64_t t2 = now_us(), first = s.detect_us + s.load_us, now = t2 - t0;
            say(nfailed ? "AMARILLO" : "VERDE",
                "arranque guiado por PC.configuracion: %zu modulos (%zu cargados, %zu fallos); lectura %" PRId64
                " us + carga %" PRId64 " us = %" PRId64 " us; primer arranque %" PRId64 " us; ganado %" PRId64 " us",
                nwanted, nloaded, nfailed, t1 - t0, t2 - t1, now, first, first - now);
            snprintf(line, sizeof(line), "arranque=guiado modulos=%zu lectura_us=%" PRId64 " carga_us=%" PRId64
                     " total_us=%" PRId64 " ganado_us=%" PRId64 "\n", nwanted, t1 - t0, t2 - t1, now, first - now);
            log_boot(line);
            copy_for_windows();
            return;
        }
    } else if (data) why = "no hay PC.configuracion valida";
    say("VERDE", "primer arranque en este PC (%s): deteccion completa", why);
    /* Full detection: every PCI and USB modalias against every alias. */
    nwanted = ndevices = 0;
    int64_t d0 = now_us();
    scan_bus("pci", true);
    scan_bus("usb", true);
    match_aliases();
    int64_t d1 = now_us();
    load_wanted();
    int64_t d2 = now_us();
    read_drivers();
    say(nfailed ? "AMARILLO" : "VERDE",
        "%zu componentes, %zu modulos necesarios (%zu cargados, %zu fallos); deteccion %" PRId64 " us + carga %" PRId64 " us",
        ndevices, nwanted, nloaded, nfailed, d1 - d0, d2 - d1);
    if (!data) return;
    if (write_config(u.release, pci_fingerprint(), d1 - d0, d2 - d1)) say("VERDE", "guardado en " CONFIG_PATH);
    snprintf(line, sizeof(line), "arranque=deteccion componentes=%zu modulos=%zu deteccion_us=%" PRId64
             " carga_us=%" PRId64 " total_us=%" PRId64 "\n", ndevices, nwanted, d1 - d0, d2 - d1, d2 - d0);
    log_boot(line);
    copy_for_windows();
}
