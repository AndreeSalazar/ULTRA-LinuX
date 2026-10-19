/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * ULTRA LinuX finds its disks by name, never by device order: the same disk
 * is vda in QEMU and sdb on the PC. Two layouts:
 *
 *   USB or whole disk: GPT partitions named ULTRA-SISTEMA-A and ULTRA-DATOS
 *   (PARTNAME in /sys/class/block/<x>/uevent).
 *
 *   Files on a FAT32 partition labelled BMO (A:\ on Windows), for a PC with
 *   no spare disk: BMO\ULTRA\sistema-<uname -r>.erofs (or sistema-a.erofs)
 *   and BMO\ULTRA\datos.ext4 are attached as loop devices. The label is read from the raw root directory,
 *   because Windows leaves "NO NAME" in the boot sector.
 *
 * Nothing else is ever mounted, so the Windows partitions are never touched.
 */
#define _GNU_SOURCE
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <linux/loop.h>
#include <stdarg.h>
#include <sys/ioctl.h>
#include <sys/mount.h>
#include <sys/stat.h>
#include <sys/utsname.h>
#include <time.h>
#include <unistd.h>
#include "init.h"
/* Last: it poisons unsafe functions after the system headers are parsed. */
#include "guardian/ultra_guard.h"

/* SATA and USB disks appear a few seconds after the kernel starts init. */
enum { WAIT_MS = 10000, STEP_MS = 100, MAX_MOUNTS = 4 };

#define FAT_LABEL "BMO"
#define FAT_TARGET "/bmo"
#define FAT_DIR FAT_TARGET "/ULTRA"

struct partition {
    const char *name, *target, *type, *file;
    unsigned long flags;
    char dev[300];
    int found;
};
static struct partition parts[] = {
    /* The system image cannot change: EROFS has no write support at all. */
    {"ULTRA-SISTEMA-A", "/sistema", "erofs", FAT_DIR "/sistema-a.erofs", MS_RDONLY | MS_NODEV | MS_NOSUID, "", 0},
    {"ULTRA-DATOS", "/datos", "ext4", FAT_DIR "/datos.ext4", MS_NODEV | MS_NOSUID, "", 0},
};
/* The FAT32 partition labelled BMO, if any. */
static char fat_dev[300];
static int fat_found;
/* Block devices of any kind seen in the last scan: none means a kernel without disks. */
static size_t block_devices;
/* What was mounted, so shut-down unmounts it in reverse order. */
static const char *mounted[MAX_MOUNTS];
static size_t nmounted;
static bool data_mounted;

static void say(const char *level, const char *fmt, ...) __attribute__((format(printf, 2, 3)));
static void say(const char *level, const char *fmt, ...) {
    char detail[384];
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(detail, sizeof(detail), fmt, ap);
    va_end(ap);
    report(level, "PARTICION", detail);
}

/* One KEY=value from a uevent file. */
static bool uevent_value(const char *dev, const char *key, char *out, size_t cap) {
    char path[320], buf[1024];
    int len = snprintf(path, sizeof(path), "/sys/class/block/%s/uevent", dev);
    if (len < 0 || (size_t)len >= sizeof(path)) return false;
    int fd = open(path, O_RDONLY | O_CLOEXEC);
    if (fd < 0) return false;
    ssize_t n = read(fd, buf, sizeof(buf) - 1);
    close(fd);
    if (n <= 0) return false;
    buf[n] = 0;
    size_t klen = strlen(key);
    char *save = NULL;
    for (char *line = strtok_r(buf, "\n", &save); line; line = strtok_r(NULL, "\n", &save))
        if (strncmp(line, key, klen) == 0 && line[klen] == '=') return ultra_copy(out, cap, line + klen + 1);
    return false;
}

static uint32_t le16(const unsigned char *p) { return (uint32_t)p[0] | (uint32_t)p[1] << 8; }
static uint32_t le32(const unsigned char *p) { return le16(p) | le16(p + 2) << 16; }

/*
 * True if the partition is FAT32 and its root directory holds the volume label.
 * Read-only and without mounting: a raw read of the boot sector and of the
 * first 4 KiB of the root directory.
 */
static bool fat32_labelled(const char *dev, const char *label) {
    unsigned char boot[512], root[4096];
    int fd = open(dev, O_RDONLY | O_CLOEXEC);
    if (fd < 0) return false;
    bool ok = pread(fd, boot, sizeof(boot), 0) == (ssize_t)sizeof(boot) && memcmp(boot + 82, "FAT32   ", 8) == 0 &&
              boot[510] == 0x55 && boot[511] == 0xAA;
    uint32_t bps = ok ? le16(boot + 11) : 0, spc = boot[13], reserved = le16(boot + 14), fats = boot[16];
    uint32_t fat_size = le32(boot + 36), root_cluster = le32(boot + 44);
    /* Values from disk are data: anything a real FAT32 cannot have is refused. */
    ok = ok && (bps == 512 || bps == 1024 || bps == 2048 || bps == 4096) && spc >= 1 && spc <= 128 &&
         (spc & (spc - 1)) == 0 && reserved >= 1 && fats >= 1 && fats <= 2 && fat_size >= 1 && root_cluster >= 2;
    if (ok) {
        /* First sector of the root directory: reserved + FAT copies + clusters before it. */
        uint64_t fat_area = ULTRA_MUL((uint64_t)fats, (uint64_t)fat_size);
        uint64_t before = ULTRA_MUL((uint64_t)root_cluster - 2, (uint64_t)spc);
        uint64_t data_start = ULTRA_ADD((uint64_t)reserved, fat_area);
        uint64_t sector = ULTRA_ADD(data_start, before);
        uint64_t offset = ULTRA_MUL(sector, (uint64_t)bps);
        ok = offset <= (uint64_t)INT64_MAX && pread(fd, root, sizeof(root), (off_t)offset) == (ssize_t)sizeof(root);
    }
    close(fd);
    if (!ok) return false;
    char want[11];
    memset(want, ' ', sizeof(want));
    size_t n = strlen(label);
    if (n > sizeof(want)) return false;
    memcpy(want, label, n);
    /* Only the first cluster belongs to the root directory for sure. */
    size_t limit = (size_t)bps * spc < sizeof(root) ? (size_t)bps * spc : sizeof(root);
    for (size_t e = 0; e + 32 <= limit; e += 32) {
        const unsigned char *entry = root + e;
        if (entry[0] == 0) break;
        /* Deleted entries and long-name pieces are not the label. */
        if (entry[0] == 0xE5 || entry[11] == 0x0F) continue;
        if ((entry[11] & 0x08) && memcmp(entry, want, sizeof(want)) == 0) return true;
    }
    return false;
}

static void record(char *dev, size_t cap, int *found, const char *name) {
    if ((*found)++ != 0) return;
    int len = snprintf(dev, cap, "/dev/%s", name);
    if (len < 0 || (size_t)len >= cap) *found = 0;
}

/* True when one of the two layouts is complete. */
static bool scan(void) {
    for (size_t i = 0; i < ULTRA_LEN(parts); i++) parts[i].found = 0;
    fat_found = 0;
    block_devices = 0;
    DIR *dir = opendir("/sys/class/block");
    if (!dir) return false;
    struct dirent *e;
    while ((e = readdir(dir)) != NULL) {
        char name[80], type[32], dev[300];
        if (e->d_name[0] == '.') continue;
        block_devices++;
        if (!uevent_value(e->d_name, "DEVTYPE", type, sizeof(type)) || strcmp(type, "partition") != 0) continue;
        if (uevent_value(e->d_name, "PARTNAME", name, sizeof(name)))
            for (size_t i = 0; i < ULTRA_LEN(parts); i++)
                if (strcmp(name, parts[i].name) == 0) record(parts[i].dev, sizeof(parts[i].dev), &parts[i].found, e->d_name);
        int len = snprintf(dev, sizeof(dev), "/dev/%s", e->d_name);
        if (len > 0 && (size_t)len < sizeof(dev) && fat32_labelled(dev, FAT_LABEL))
            record(fat_dev, sizeof(fat_dev), &fat_found, e->d_name);
    }
    closedir(dir);
    bool named = true;
    for (size_t i = 0; i < ULTRA_LEN(parts); i++) named = named && parts[i].found > 0;
    return named || fat_found > 0;
}

static bool mount_one(const char *dev, const char *target, const char *type, unsigned long flags, const char *what) {
    if (mkdir(target, 0755) < 0 && errno != EEXIST) {
        say("ROJO", "mkdir %s: %s", target, strerror(errno));
        return false;
    }
    if (mount(dev, target, type, flags, NULL) < 0) {
        say("ROJO", "%s (%s, %s) en %s: %s", what, dev, type, target, strerror(errno));
        return false;
    }
    if (nmounted < MAX_MOUNTS) mounted[nmounted++] = target;
    if (strcmp(target, "/datos") == 0) data_mounted = true;
    say("VERDE", "%s (%s, %s%s) en %s", what, dev, type, (flags & MS_RDONLY) ? ", solo lectura" : "", target);
    return true;
}

/*
 * A file as a block device. The returned descriptor must stay open until the
 * mount: with AUTOCLEAR the loop device detaches at its last close or unmount.
 */
static int attach_loop(const char *file, bool read_only, char *dev, size_t cap) {
    int ctl = open("/dev/loop-control", O_RDWR | O_CLOEXEC);
    if (ctl < 0) return -1;
    int n = ioctl(ctl, LOOP_CTL_GET_FREE);
    close(ctl);
    int len = n >= 0 ? snprintf(dev, cap, "/dev/loop%d", n) : -1;
    if (len < 0 || (size_t)len >= cap) return -1;
    int mode = (read_only ? O_RDONLY : O_RDWR) | O_CLOEXEC;
    int back = open(file, mode);
    if (back < 0) return -1;
    int lo = open(dev, mode);
    if (lo < 0) {
        close(back);
        return -1;
    }
    struct loop_config cfg;
    memset(&cfg, 0, sizeof(cfg));
    cfg.fd = (uint32_t)back;
    cfg.info.lo_flags = LO_FLAGS_AUTOCLEAR | (read_only ? LO_FLAGS_READ_ONLY : 0);
    int r = ioctl(lo, LOOP_CONFIGURE, &cfg);
    int err = errno;
    close(back);
    if (r < 0) {
        close(lo);
        errno = err;
        return -1;
    }
    return lo;
}

static void mount_named(void) {
    for (size_t i = 0; i < ULTRA_LEN(parts); i++) {
        struct partition *p = &parts[i];
        if (p->found > 1) {
            /* Two disks with the same name: guessing could mount the wrong one. */
            say("ROJO", "%d particiones se llaman %s: no se monta ninguna", p->found, p->name);
            continue;
        }
        mount_one(p->dev, p->target, p->type, p->flags, p->name);
    }
}

static void mount_files(void) {
    if (fat_found > 1) {
        say("ROJO", "%d particiones FAT32 se llaman %s: no se monta ninguna", fat_found, FAT_LABEL);
        return;
    }
    /* noexec: nothing runs from the FAT32 side; the system comes from the EROFS image. */
    if (!mount_one(fat_dev, FAT_TARGET, "vfat", MS_NODEV | MS_NOSUID | MS_NOEXEC, FAT_LABEL)) return;
    /* Each kernel brings its own system image, so several kernels share one BMO and one datos.ext4. */
    struct utsname u;
    char own[300] = "";
    if (uname(&u) == 0) {
        int len = snprintf(own, sizeof(own), "%s/sistema-%s.erofs", FAT_DIR, u.release);
        if (len < 0 || (size_t)len >= sizeof(own) || access(own, R_OK) < 0) own[0] = 0;
    }
    for (size_t i = 0; i < ULTRA_LEN(parts); i++) {
        struct partition *p = &parts[i];
        bool read_only = (p->flags & MS_RDONLY) != 0;
        const char *file = read_only && own[0] ? own : p->file;
        int lo = attach_loop(file, read_only, p->dev, sizeof(p->dev));
        if (lo < 0) {
            say("AMARILLO", "%s: %s; %s queda sin montar", file, strerror(errno), p->target);
            continue;
        }
        char what[384];
        snprintf(what, sizeof(what), "%s <- %s", p->name, file);
        mount_one(p->dev, p->target, p->type, p->flags, what);
        close(lo);
    }
}

bool mount_partitions(void) {
    int waited = 0;
    bool done = scan();
    if (block_devices == 0) {
        /* The lab kernels boot from memory alone: there is nothing to wait for. */
        report("AMARILLO", "PARTICION", "el kernel no ve ningun disco: /sistema y /datos quedan sin montar");
        return false;
    }
    while (!done && waited < WAIT_MS) {
        struct timespec step = {0, STEP_MS * 1000000L};
        nanosleep(&step, NULL);
        waited += STEP_MS;
        done = scan();
    }
    if (waited > 0) say("AMARILLO", "discos buscados durante %d ms", waited);
    bool named = true;
    for (size_t i = 0; i < ULTRA_LEN(parts); i++) named = named && parts[i].found > 0;
    if (named) mount_named();
    else if (fat_found > 0) mount_files();
    else
        say("AMARILLO", "ni particiones %s/%s ni FAT32 %s: /sistema y /datos quedan sin montar", parts[0].name,
            parts[1].name, FAT_LABEL);
    return data_mounted;
}

const char *bmo_device(void) { return windows_dir() ? fat_dev : NULL; }

const char *windows_dir(void) {
    for (size_t i = 0; i < nmounted; i++)
        if (strcmp(mounted[i], FAT_TARGET) == 0) return FAT_DIR;
    return NULL;
}

void unmount_data(void) {
    /* Reverse order: /datos and /sistema release their loop files before BMO goes. */
    while (nmounted > 0) {
        const char *target = mounted[--nmounted];
        if (strcmp(target, "/sistema") == 0) {
            /* Read-only: nothing to flush, but its loop file lives on BMO. */
            if (umount2(target, 0) < 0) say("AMARILLO", "umount %s: %s", target, strerror(errno));
            continue;
        }
        if (umount2(target, 0) == 0) {
            if (strcmp(target, "/datos") == 0) data_mounted = false;
            report("VERDE", "DESMONTAJE", strcmp(target, "/datos") == 0 ? "/datos desmontado limpio" : FAT_TARGET " desmontado limpio");
            continue;
        }
        int err = errno;
        char detail[160];
        /* Something still holds a file open: read-only still leaves the disk clean. */
        if (mount(NULL, target, NULL, MS_REMOUNT | MS_RDONLY, NULL) == 0) {
            snprintf(detail, sizeof(detail), "umount %s: %s; queda en solo lectura", target, strerror(err));
            report("AMARILLO", "DESMONTAJE", detail);
        } else {
            snprintf(detail, sizeof(detail), "umount %s: %s; remontar: %s", target, strerror(err), strerror(errno));
            report("ROJO", "DESMONTAJE", detail);
        }
    }
}
