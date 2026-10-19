/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * Firmware boot entries from Linux, through efivarfs. Only what ULTRA needs:
 * find an entry that starts a given file on the BMO partition, create one,
 * append it to BootOrder, ask for it once with BootNext, and delete it.
 *
 * Guardians: an entry is only ever created for a path under \EFI\ULTRA\ on
 * the partition ULTRA itself mounted; BootOrder only gains or loses ULTRA's
 * own number, never reorders or drops anyone else's.
 */
#define _GNU_SOURCE
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <linux/fs.h>
#include <sys/ioctl.h>
#include <sys/mount.h>
#include <sys/stat.h>
#include <unistd.h>
#include "init.h"
/* Last: it poisons unsafe functions after the system headers are parsed. */
#include "guardian/ultra_guard.h"

#define EFIVARS "/sys/firmware/efi/efivars"
#define GLOBAL "8be4df61-93ca-11d2-aa0d-00e098032b8c"
/* Non-volatile, boot service and runtime access: what boot variables carry. */
#define EFI_ATTR 0x7u

enum { VAR_MAX = 4096, PATH_MAX_CHARS = 128 };

bool efi_ready(void) {
    if (access("/sys/firmware/efi", F_OK) < 0) return false;
    if (access(EFIVARS "/BootOrder-" GLOBAL, F_OK) == 0) return true;
    if (mount("efivarfs", EFIVARS, "efivarfs", MS_NOSUID | MS_NODEV | MS_NOEXEC, NULL) < 0 && errno != EBUSY) return false;
    return access(EFIVARS, R_OK) == 0;
}

static bool var_path(char *out, size_t cap, const char *name) {
    int n = snprintf(out, cap, EFIVARS "/%s-" GLOBAL, name);
    return n > 0 && (size_t)n < cap;
}

/* The variable's data, without the 4 attribute bytes efivarfs puts first. -1 if absent. */
static ssize_t var_read(const char *name, uint8_t *buf, size_t cap) {
    char path[160];
    uint8_t raw[VAR_MAX + 4];
    if (!var_path(path, sizeof(path), name)) return -1;
    int fd = open(path, O_RDONLY | O_CLOEXEC);
    if (fd < 0) return -1;
    ssize_t n = read(fd, raw, sizeof(raw));
    close(fd);
    if (n < 4 || (size_t)(n - 4) > cap) return -1;
    memcpy(buf, raw + 4, (size_t)(n - 4));
    return n - 4;
}

/* efivarfs marks existing variables immutable so a stray rm cannot brick a board. */
static void make_mutable(const char *path) {
    int fd = open(path, O_RDONLY | O_CLOEXEC);
    if (fd < 0) return;
    int flags = 0;
    if (ioctl(fd, FS_IOC_GETFLAGS, &flags) == 0 && (flags & FS_IMMUTABLE_FL)) {
        flags &= ~FS_IMMUTABLE_FL;
        (void)ioctl(fd, FS_IOC_SETFLAGS, &flags);
    }
    close(fd);
}

static bool var_write(const char *name, const uint8_t *data, size_t len) {
    char path[160];
    uint8_t raw[VAR_MAX + 4];
    if (!var_path(path, sizeof(path), name) || len > VAR_MAX) return false;
    uint32_t attr = EFI_ATTR;
    memcpy(raw, &attr, 4);
    memcpy(raw + 4, data, len);
    if (access(path, F_OK) == 0) make_mutable(path);
    int fd = open(path, O_WRONLY | O_CREAT | O_CLOEXEC, 0644);
    if (fd < 0) return false;
    /* One write: efivarfs sets the whole variable in a single call. */
    bool ok = write(fd, raw, len + 4) == (ssize_t)(len + 4);
    close(fd);
    return ok;
}

static bool var_delete(const char *name) {
    char path[160];
    if (!var_path(path, sizeof(path), name)) return false;
    if (access(path, F_OK) < 0) return true;
    make_mutable(path);
    return unlink(path) == 0;
}

/* Little-endian reads from disk and variable bytes: values are data, built byte by byte. */
static uint16_t rd16(const uint8_t *p) { return (uint16_t)(p[0] | p[1] << 8); }
static uint32_t rd32(const uint8_t *p) { return (uint32_t)rd16(p) | (uint32_t)rd16(p + 2) << 16; }
static uint64_t rd64(const uint8_t *p) { return (uint64_t)rd32(p) | (uint64_t)rd32(p + 4) << 32; }

/* The partition's GPT number, start, size and unique GUID, as the firmware sees them. */
struct gpt_part {
    uint32_t number;
    uint64_t start, size;
    uint8_t guid[16];
};

static bool sys_number(const char *dev, const char *attr, uint64_t *out) {
    char path[160], buf[32];
    int n = snprintf(path, sizeof(path), "/sys/class/block/%s/%s", dev, attr);
    if (n < 0 || (size_t)n >= sizeof(path)) return false;
    int fd = open(path, O_RDONLY | O_CLOEXEC);
    if (fd < 0) return false;
    ssize_t r = read(fd, buf, sizeof(buf) - 1);
    close(fd);
    if (r <= 0) return false;
    buf[r] = 0;
    char *end = NULL;
    unsigned long long v = strtoull(buf, &end, 10);
    if (end == buf) return false;
    *out = v;
    return true;
}

static bool gpt_lookup(const char *devpath, struct gpt_part *p) {
    const char *dev = strrchr(devpath, '/');
    dev = dev ? dev + 1 : devpath;
    uint64_t number, start, size;
    if (!sys_number(dev, "partition", &number) || !sys_number(dev, "start", &start) || !sys_number(dev, "size", &size))
        return false;
    /* The parent disk: /sys/class/block/sda2 lives under .../sda. */
    char link[512], disk[300];
    int n = snprintf(link, sizeof(link), "/sys/class/block/%s/..", dev);
    char *real = n > 0 ? realpath(link, NULL) : NULL;
    if (!real) return false;
    const char *parent = strrchr(real, '/');
    n = snprintf(disk, sizeof(disk), "/dev/%s", parent ? parent + 1 : real);
    free(real);
    if (n < 0 || (size_t)n >= sizeof(disk)) return false;
    /* /sys counts 512-byte sectors; the GPT counts logical blocks. */
    uint64_t lbs = 512;
    if (!sys_number(disk + 5, "queue/logical_block_size", &lbs) || (lbs != 512 && lbs != 4096)) lbs = 512;
    int fd = open(disk, O_RDONLY | O_CLOEXEC);
    if (fd < 0) return false;
    uint8_t hdr[512], entry[128];
    bool ok = pread(fd, hdr, sizeof(hdr), (off_t)lbs) == (ssize_t)sizeof(hdr) && memcmp(hdr, "EFI PART", 8) == 0;
    uint64_t entries_lba = 0;
    uint32_t count = 0, esize = 0;
    if (ok) {
        entries_lba = rd64(hdr + 72);
        count = rd32(hdr + 80);
        esize = rd32(hdr + 84);
        ok = number >= 1 && number <= count && esize >= 128 && esize <= 4096 && entries_lba >= 2 && entries_lba < (1ULL << 40);
    }
    if (ok) {
        uint64_t table = ULTRA_MUL(entries_lba, lbs);
        uint64_t within = ULTRA_MUL(number - 1, (uint64_t)esize);
        uint64_t off = ULTRA_ADD(table, within);
        ok = pread(fd, entry, sizeof(entry), (off_t)off) == (ssize_t)sizeof(entry);
    }
    close(fd);
    if (!ok) return false;
    p->number = (uint32_t)number;
    p->start = start * 512 / lbs;
    p->size = size * 512 / lbs;
    memcpy(p->guid, entry + 16, 16);
    return true;
}

/* Little-endian append helpers over a bounded buffer. */
struct out {
    uint8_t *b;
    size_t len, cap;
    bool full;
};
static void put(struct out *o, const void *data, size_t n) {
    if (o->full || n > o->cap - o->len) {
        o->full = true;
        return;
    }
    memcpy(o->b + o->len, data, n);
    o->len += n;
}
static void put_u8(struct out *o, uint8_t v) { put(o, &v, 1); }
static void put_u16(struct out *o, uint16_t v) { put(o, &v, 2); }
static void put_u32(struct out *o, uint32_t v) { put(o, &v, 4); }
static void put_u64(struct out *o, uint64_t v) { put(o, &v, 8); }
/* ASCII to UCS-2, with its terminating zero. */
static void put_ucs2(struct out *o, const char *s) {
    for (; *s; s++) put_u16(o, (uint16_t)(unsigned char)*s);
    put_u16(o, 0);
}

/* The device path of a file on the partition: hard drive node, file node, end. */
static void put_path(struct out *o, const struct gpt_part *p, const char *file) {
    /* Hard drive node: type 4, subtype 1, 42 bytes. */
    put_u8(o, 4);
    put_u8(o, 1);
    put_u16(o, 42);
    put_u32(o, p->number);
    put_u64(o, p->start);
    put_u64(o, p->size);
    put(o, p->guid, 16);
    put_u8(o, 2); /* GPT partition */
    put_u8(o, 2); /* GUID signature */
    /* File path node: type 4, subtype 4, UCS-2 path. */
    size_t chars = strlen(file) + 1;
    put_u8(o, 4);
    put_u8(o, 4);
    put_u16(o, (uint16_t)(4 + 2 * chars));
    put_ucs2(o, file);
    /* End of the device path. */
    put_u8(o, 0x7f);
    put_u8(o, 0xff);
    put_u16(o, 4);
}

static bool ascii_ieq(uint16_t a, char b) {
    if (a >= 'a' && a <= 'z') a = (uint16_t)(a - 32);
    char c = (b >= 'a' && b <= 'z') ? (char)(b - 32) : b;
    return a == (uint16_t)(unsigned char)c;
}

/* True if this load option starts `file` on the partition with this GUID. */
static bool option_matches(const uint8_t *v, size_t len, const struct gpt_part *p, const char *file) {
    if (len < 6) return false;
    uint16_t fplen = rd16(v + 4);
    size_t i = 6;
    /* Skip the UCS-2 description. */
    while (i + 1 < len && (v[i] | v[i + 1])) i += 2;
    i += 2;
    size_t end = i + fplen;
    if (end > len) return false;
    bool same_partition = false, same_file = false;
    while (i + 4 <= end) {
        uint8_t type = v[i], sub = v[i + 1];
        uint16_t nlen = rd16(v + i + 2);
        if (nlen < 4 || i + nlen > end) return false;
        if (type == 4 && sub == 1 && nlen >= 42) same_partition = memcmp(v + i + 24, p->guid, 16) == 0;
        if (type == 4 && sub == 4) {
            size_t chars = ((size_t)nlen - 4) / 2, k = 0;
            const uint8_t *s = v + i + 4;
            for (; k < chars && file[k]; k++) {
                uint16_t ch = rd16(s + 2 * k);
                if (!ascii_ieq(ch, file[k])) break;
            }
            uint16_t after = 0;
            if (k < chars) after = rd16(s + 2 * k);
            same_file = file[k] == 0 && after == 0;
        }
        if (type == 0x7f && sub == 0xff) break;
        i += nlen;
    }
    return same_partition && same_file;
}

/* Boot#### numbers that exist, from the efivarfs directory. */
static size_t boot_numbers(uint16_t *nums, size_t cap) {
    DIR *d = opendir(EFIVARS);
    if (!d) return 0;
    size_t n = 0;
    struct dirent *e;
    while ((e = readdir(d)) != NULL && n < cap) {
        unsigned int x;
        char rest[64];
        if (strlen(e->d_name) == 45 && sscanf(e->d_name, "Boot%4x-%63s", &x, rest) == 2 && strcmp(rest, GLOBAL) == 0)
            nums[n++] = (uint16_t)x;
    }
    closedir(d);
    return n;
}

int efi_find_entry(const char *devpath, const char *file) {
    struct gpt_part p;
    if (!gpt_lookup(devpath, &p)) return -1;
    uint16_t nums[512];
    size_t n = boot_numbers(nums, ULTRA_LEN(nums));
    for (size_t i = 0; i < n; i++) {
        char name[16];
        uint8_t v[VAR_MAX];
        snprintf(name, sizeof(name), "Boot%04X", nums[i]);
        ssize_t len = var_read(name, v, sizeof(v));
        if (len > 0 && option_matches(v, (size_t)len, &p, file)) return nums[i];
    }
    return -1;
}

static bool order_read(uint16_t *order, size_t cap, size_t *count) {
    uint8_t v[VAR_MAX];
    ssize_t len = var_read("BootOrder", v, sizeof(v));
    *count = 0;
    if (len < 0) return true;
    if ((size_t)len / 2 > cap) return false;
    *count = (size_t)len / 2;
    memcpy(order, v, *count * 2);
    return true;
}

int efi_create_entry(const char *devpath, const char *file, const char *title) {
    /* Guardian: ULTRA only ever points the firmware at its own files. */
    if (strncmp(file, "\\EFI\\ULTRA\\", 11) != 0 || strlen(file) >= PATH_MAX_CHARS) return -1;
    struct gpt_part p;
    if (!gpt_lookup(devpath, &p)) return -1;
    uint8_t buf[1024];
    struct out o = {buf, 0, sizeof(buf), false};
    put_u32(&o, 1); /* LOAD_OPTION_ACTIVE */
    /* FilePathListLength goes right after the 4 attribute bytes, at offset 4; filled in below. */
    put_u16(&o, 0);
    put_ucs2(&o, title);
    size_t path_start = o.len;
    put_path(&o, &p, file);
    if (o.full) return -1;
    uint16_t fplen = (uint16_t)(o.len - path_start);
    buf[4] = (uint8_t)(fplen & 0xff);
    buf[5] = (uint8_t)(fplen >> 8);
    /* The first free number, from the top: firmware entries tend to use the low ones. */
    uint16_t nums[512];
    size_t n = boot_numbers(nums, ULTRA_LEN(nums));
    int free_num = -1;
    for (int x = 0x00FF; x >= 0x0010 && free_num < 0; x--) {
        bool used = false;
        for (size_t i = 0; i < n; i++) used = used || nums[i] == x;
        if (!used) free_num = x;
    }
    if (free_num < 0) return -1;
    char name[16];
    snprintf(name, sizeof(name), "Boot%04X", (unsigned)free_num);
    if (!var_write(name, buf, o.len)) return -1;
    uint16_t order[256];
    size_t count;
    if (!order_read(order, ULTRA_LEN(order) - 1, &count)) return free_num;
    order[count++] = (uint16_t)free_num;
    /* Last in the order: Windows keeps being the default boot. */
    if (!var_write("BootOrder", (const uint8_t *)order, count * 2)) report("AMARILLO", "PERFIL", "la entrada existe, pero no se pudo añadir a BootOrder");
    return free_num;
}

bool efi_delete_entry(int num) {
    if (num < 0 || num > 0xFFFF) return false;
    uint16_t order[256];
    size_t count, kept = 0;
    if (order_read(order, ULTRA_LEN(order), &count)) {
        for (size_t i = 0; i < count; i++)
            if (order[i] != num) order[kept++] = order[i];
        if (kept != count && !var_write("BootOrder", (const uint8_t *)order, kept * 2)) return false;
    }
    char name[16];
    snprintf(name, sizeof(name), "Boot%04X", (unsigned)num);
    return var_delete(name);
}

bool efi_set_boot_next(int num) {
    uint16_t v = (uint16_t)num;
    return num >= 0 && num <= 0xFFFF && var_write("BootNext", (const uint8_t *)&v, 2);
}

bool efi_boot_next_pending(void) {
    uint8_t v[8];
    return var_read("BootNext", v, sizeof(v)) == 2;
}
