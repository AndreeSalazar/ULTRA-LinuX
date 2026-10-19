/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * Partitions are found by their GPT name (PARTNAME in /sys/class/block/<x>/uevent),
 * never by device order: the same USB stick is vda in QEMU and sdb on the PC.
 * Only these exact names are mounted, so the Windows disk is never touched.
 */
#define _GNU_SOURCE
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <sys/mount.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>
#include "init.h"
/* Last: it poisons unsafe functions after the system headers are parsed. */
#include "guardian/ultra_guard.h"

/* USB storage appears a few seconds after the kernel starts init. */
enum { WAIT_MS = 10000, STEP_MS = 100 };

struct partition {
    const char *name, *target, *type;
    unsigned long flags;
    char dev[300];
    int found;
};
static struct partition parts[] = {
    /* The system image cannot change: EROFS has no write support at all. */
    {"ULTRA-SISTEMA-A", "/sistema", "erofs", MS_RDONLY | MS_NODEV | MS_NOSUID, "", 0},
    {"ULTRA-DATOS", "/datos", "ext4", MS_NODEV | MS_NOSUID, "", 0},
};
static bool data_mounted;

static bool partname_of(const char *dev, char *name, size_t cap) {
    char path[320], buf[1024];
    int len = snprintf(path, sizeof(path), "/sys/class/block/%s/uevent", dev);
    if (len < 0 || (size_t)len >= sizeof(path)) return false;
    int fd = open(path, O_RDONLY | O_CLOEXEC);
    if (fd < 0) return false;
    ssize_t n = read(fd, buf, sizeof(buf) - 1);
    close(fd);
    if (n <= 0) return false;
    buf[n] = 0;
    char *save = NULL;
    for (char *line = strtok_r(buf, "\n", &save); line; line = strtok_r(NULL, "\n", &save))
        if (strncmp(line, "PARTNAME=", 9) == 0) return ultra_copy(name, cap, line + 9);
    return false;
}

/* Block devices of any kind seen in the last scan: none means a kernel without disks. */
static size_t block_devices;

static bool scan(void) {
    for (size_t i = 0; i < ULTRA_LEN(parts); i++) parts[i].found = 0;
    block_devices = 0;
    DIR *dir = opendir("/sys/class/block");
    if (!dir) return false;
    struct dirent *e;
    while ((e = readdir(dir)) != NULL) {
        char name[80];
        if (e->d_name[0] == '.') continue;
        block_devices++;
        if (!partname_of(e->d_name, name, sizeof(name))) continue;
        for (size_t i = 0; i < ULTRA_LEN(parts); i++) {
            if (strcmp(name, parts[i].name) != 0) continue;
            if (parts[i].found++ == 0) {
                int len = snprintf(parts[i].dev, sizeof(parts[i].dev), "/dev/%s", e->d_name);
                if (len < 0 || (size_t)len >= sizeof(parts[i].dev)) parts[i].found = 0;
            }
        }
    }
    closedir(dir);
    for (size_t i = 0; i < ULTRA_LEN(parts); i++)
        if (parts[i].found == 0) return false;
    return true;
}

bool mount_partitions(void) {
    char detail[384];
    int waited = 0;
    bool all = scan();
    if (block_devices == 0) {
        /* The lab kernels boot from memory alone: there is nothing to wait for. */
        report("AMARILLO", "PARTICION", "el kernel no ve ningun disco: /sistema y /datos quedan sin montar");
        return false;
    }
    while (!all && waited < WAIT_MS) {
        struct timespec step = {0, STEP_MS * 1000000L};
        nanosleep(&step, NULL);
        waited += STEP_MS;
        all = scan();
    }
    if (waited > 0) {
        snprintf(detail, sizeof(detail), "particiones buscadas durante %d ms", waited);
        report("AMARILLO", "PARTICION", detail);
    }
    for (size_t i = 0; i < ULTRA_LEN(parts); i++) {
        struct partition *p = &parts[i];
        if (p->found == 0) {
            snprintf(detail, sizeof(detail), "%s no encontrada: %s queda sin montar", p->name, p->target);
            report("AMARILLO", "PARTICION", detail);
            continue;
        }
        if (p->found > 1) {
            /* Two disks with the same name: guessing could mount the wrong one. */
            snprintf(detail, sizeof(detail), "%d particiones se llaman %s: no se monta ninguna", p->found, p->name);
            report("ROJO", "PARTICION", detail);
            continue;
        }
        if (mkdir(p->target, 0755) < 0 && errno != EEXIST) {
            snprintf(detail, sizeof(detail), "mkdir %s: %s", p->target, strerror(errno));
            report("ROJO", "PARTICION", detail);
            continue;
        }
        if (mount(p->dev, p->target, p->type, p->flags, NULL) < 0) {
            snprintf(detail, sizeof(detail), "%s (%s, %s) en %s: %s", p->name, p->dev, p->type, p->target, strerror(errno));
            report("ROJO", "PARTICION", detail);
            continue;
        }
        if (strcmp(p->target, "/datos") == 0) data_mounted = true;
        snprintf(detail, sizeof(detail), "%s (%s, %s%s) en %s", p->name, p->dev, p->type,
                 (p->flags & MS_RDONLY) ? ", solo lectura" : "", p->target);
        report("VERDE", "PARTICION", detail);
    }
    return data_mounted;
}

void unmount_data(void) {
    if (!data_mounted) return;
    if (umount2("/datos", 0) == 0) {
        data_mounted = false;
        report("VERDE", "DESMONTAJE", "/datos desmontado limpio");
        return;
    }
    char detail[160];
    int err = errno;
    /* Something still holds a file open: read-only still leaves the journal clean. */
    if (mount(NULL, "/datos", NULL, MS_REMOUNT | MS_RDONLY, NULL) == 0) {
        snprintf(detail, sizeof(detail), "umount /datos: %s; queda en solo lectura", strerror(err));
        report("AMARILLO", "DESMONTAJE", detail);
    } else {
        snprintf(detail, sizeof(detail), "umount /datos: %s; remontar: %s", strerror(err), strerror(errno));
        report("ROJO", "DESMONTAJE", detail);
    }
}
