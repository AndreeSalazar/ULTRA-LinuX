/* SPDX-License-Identifier: GPL-2.0-only */
/* Read-only, bounded telemetry. No data-plane sockets, capture or privileges.
 * sysfs counters are separate reads, not an atomic snapshot or wire-byte count. */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <ifaddrs.h>
#include <inttypes.h>
#include <net/if.h>
#include <stdio.h>
#include <string.h>
#include <time.h>
#include <unistd.h>
#include "guardian/ultra_guard.h"

/* The same parser is exercised with malformed/truncated fixtures in the lab. */
static bool counter(const char *s, size_t n, uint64_t *out) {
    if (n == 0) return false;
    if (s[n - 1] == '\n') n--;
    if (n == 0 || n > 20) return false;
    uint64_t value = 0;
    for (size_t i = 0; i < n; i++) {
        unsigned char c = (unsigned char)s[i];
        if (c < '0' || c > '9') return false;
        uint64_t digit = (uint64_t)(c - '0');
        if (value > (UINT64_MAX - digit) / 10) return false;
        value = value * 10 + digit;
    }
    *out = value;
    return true;
}

static void attribute(const char *iface, const char *name, const char *key) {
    char path[256], buf[32];
    uint64_t value;
    int len = snprintf(path, sizeof(path), "/sys/class/net/%s/%s", iface, name);
    if (len < 0 || (size_t)len >= sizeof(path)) { printf(" %s=NA", key); return; }
    int fd = open(path, O_RDONLY | O_CLOEXEC | O_NOFOLLOW | O_NONBLOCK);
    if (fd < 0) { printf(" %s=NA", key); return; }
    size_t used = 0;
    bool ok = true;
    while (used < sizeof(buf)) {
        ssize_t n = read(fd, buf + used, sizeof(buf) - used);
        if (n < 0 && errno == EINTR) continue;
        if (n < 0) { ok = false; break; }
        if (n == 0) break;
        used += (size_t)n;
    }
    close(fd);
    if (!ok || used == sizeof(buf) || !counter(buf, used, &value))
        printf(" %s=NA", key);
    else printf(" %s=%" PRIu64, key, value);
}

static bool valid_iface(const char *name) {
    size_t n = strnlen(name, IFNAMSIZ);
    if (n == 0 || n >= IFNAMSIZ) return false;
    for (size_t i = 0; i < n; i++) {
        unsigned char c = (unsigned char)name[i];
        if (!((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
              (c >= '0' && c <= '9') || c == '_' || c == '-' || c == '.' || c == ':')) return false;
    }
    return strcmp(name, ".") != 0 && strcmp(name, "..") != 0;
}

#ifndef ULTRA_RED_TEST
int main(void) {
    ultra_guard_install("ultra-red-contadores");
    struct timespec ts;
    if (clock_gettime(CLOCK_MONOTONIC, &ts) != 0) return 1;
    struct if_nameindex *interfaces = if_nameindex();
    if (!interfaces) { puts("RED NA inventario_no_disponible"); return 1; }
    size_t count = 0;
    for (struct if_nameindex *p = interfaces; p->if_index != 0 && p->if_name; p++) {
        if (strcmp(p->if_name, "lo") == 0) continue;
        if (count >= 64) { puts("RED NA inventario_truncado_limite_64"); break; }
        if (!valid_iface(p->if_name)) { puts("RED NA nombre_no_admitido"); continue; }
        count++;
        printf("RED iface=%s indice=%u monotonic_s=%jd.%09ld", p->if_name, p->if_index,
               (intmax_t)ts.tv_sec, ts.tv_nsec);
        static const char *const stats[] = {"rx_bytes", "rx_packets", "rx_errors", "rx_dropped",
                                           "tx_bytes", "tx_packets", "tx_errors", "tx_dropped"};
        for (size_t i = 0; i < ULTRA_LEN(stats); i++) {
            char attr[64];
            int n = snprintf(attr, sizeof(attr), "statistics/%s", stats[i]);
            if (n < 0 || (size_t)n >= sizeof(attr)) { if_freenameindex(interfaces); return 1; }
            attribute(p->if_name, attr, stats[i]);
        }
        attribute(p->if_name, "carrier", "carrier");
        attribute(p->if_name, "speed", "speed_Mbps");
        attribute(p->if_name, "mtu", "mtu");
        putchar('\n');
    }
    if_freenameindex(interfaces);
    if (count == 0) puts("RED NA sin_interfaces_no_loopback");
    return ferror(stdout) ? 1 : 0;
}
#endif
