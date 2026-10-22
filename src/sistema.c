/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * Wayland stage W1: Arch's base system as a full root at /arch.
 *
 * /sistema (the read-only EROFS image) is the lower layer; a tmpfs on top takes
 * every write, so the image never changes and a reboot starts clean. /dev,
 * /proc, /sys, /tmp and /datos are bound inside, so programs there see the real
 * machine and the data that survives. `ultra-entrar` walks into it.
 *
 * The init stays where it is: PID 1, its guardians, BusyBox for the shells and
 * caja, whose rules are measured for BusyBox. Switching the default root to Arch
 * comes once caja has rules measured for Arch's dynamic programs.
 */
#define _GNU_SOURCE
#include <dirent.h>
#include <errno.h>
#include <sys/mount.h>
#include <sys/stat.h>
#include <unistd.h>
#include "init.h"
/* Last: it poisons unsafe functions after the system headers are parsed. */
#include "guardian/ultra_guard.h"

#define ARCH "/arch"
#define LAYER "/run/arch-capa"

static bool ready;

static bool make_dir(const char *path) { return mkdir(path, 0755) == 0 || errno == EEXIST; }

static bool fail(const char *what, const char *path) {
    char detail[256];
    snprintf(detail, sizeof(detail), "%s %s: %s; el sistema Arch queda sin montar", what, path, strerror(errno));
    report("ROJO", "SISTEMA", detail);
    return false;
}

/* Packages and the glibc version, from the pacman database inside the image. */
static void describe(char *out, size_t cap) {
    DIR *d = opendir("/sistema/var/lib/pacman/local");
    size_t packages = 0;
    char glibc[64] = "glibc ?";
    struct dirent *e;
    while (d && (e = readdir(d)) != NULL) {
        if (e->d_name[0] == '.' || e->d_type != DT_DIR) continue;
        packages++;
        /* glibc-2.44+r50+g1848099f063e-1 -> glibc 2.44 */
        if (strncmp(e->d_name, "glibc-", 6) == 0 && e->d_name[6] >= '0' && e->d_name[6] <= '9') {
            size_t n = strcspn(e->d_name + 6, "+-");
            snprintf(glibc, sizeof(glibc), "glibc %.*s", (int)(n < 20 ? n : 20), e->d_name + 6);
        }
    }
    if (d) closedir(d);
    snprintf(out, cap, "%zu paquetes, %s", packages, glibc);
}

static bool bind(const char *path) {
    char target[64];
    int n = snprintf(target, sizeof(target), ARCH "%s", path);
    if (n < 0 || (size_t)n >= sizeof(target)) return false;
    if (!make_dir(target)) return fail("mkdir", target);
    if (mount(path, target, NULL, MS_BIND | MS_REC, NULL) < 0) return fail("bind", target);
    return true;
}

void prepare_system(bool data) {
    /* An older image carries only the modules: nothing to enter. */
    if (access("/sistema/usr/bin/bash", X_OK) < 0) return;
    if (!make_dir(LAYER) || mount("tmpfs", LAYER, "tmpfs", MS_NOSUID | MS_NODEV, "mode=0755") < 0) {
        fail("capa en memoria", LAYER);
        return;
    }
    if (!make_dir(LAYER "/arriba") || !make_dir(LAYER "/trabajo") || !make_dir(ARCH)) {
        fail("mkdir", LAYER);
        return;
    }
    if (mount("overlay", ARCH, "overlay", 0, "lowerdir=/sistema,upperdir=" LAYER "/arriba,workdir=" LAYER "/trabajo") < 0) {
        fail("overlay", ARCH);
        umount2(LAYER, MNT_DETACH);
        return;
    }
    ready = true;
    const char *const shared[] = {"/dev", "/proc", "/sys", "/tmp"};
    for (size_t i = 0; i < ULTRA_LEN(shared); i++)
        if (!bind(shared[i])) return;
    if (data && !bind("/datos")) return;
    char detail[192], what[96];
    describe(what, sizeof(what));
    snprintf(detail, sizeof(detail), "base Arch en " ARCH " (%s; EROFS + capa en memoria); entra con: ultra-entrar", what);
    report("VERDE", "SISTEMA", detail);
}

void release_system(void) {
    if (!ready) return;
    if (umount(ARCH "/windows") == 0) report("VERDE", "WINDOWS", "NVMe desmontado");
    else if (errno != EINVAL && errno != ENOENT) report("AMARILLO", "WINDOWS-DESMONTAJE", "/windows");
    /* Games are nested: overlay first, ext4 loop (autoclear), then NTFS D:.
       PID 1 has already stopped all processes. Never detach a live backing store. */
    const char *const games[] = {ARCH "/juegos/Cyberpunk2077", ARCH "/juegos", ARCH "/run/ultra-juegos/volumen"};
    for (size_t i = 0; i < ULTRA_LEN(games); i++) {
        if (umount(games[i]) == 0) report("VERDE", "JUEGOS", games[i]);
        else if (errno != EINVAL && errno != ENOENT) report("AMARILLO", "JUEGOS-DESMONTAJE", games[i]);
    }
    /* Detached as a whole tree: the binds go with it, and /sistema and BMO can be unmounted cleanly. */
    if (umount2(ARCH, MNT_DETACH) < 0) report("AMARILLO", "SISTEMA", "no se pudo soltar /arch");
    umount2(LAYER, MNT_DETACH);
    ready = false;
}
