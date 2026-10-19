/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * The ULTRA logo, drawn by the init on the firmware screen through /dev/fb0:
 * top-right corner, blended by its alpha over whatever is already there.
 * The kernel stays unmodified; the picture travels in the initramfs as
 * /etc/ultra-logo.bin ("ULGO", u16 width, u16 height, RGBA pixels), made by
 * scripts/convertir_logo.py. Based on Tux by Larry Ewing (lewing@isc.tamu.edu),
 * drawn with The GIMP.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <linux/fb.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>
#include "init.h"
/* Last: it poisons unsafe functions after the system headers are parsed. */
#include "guardian/ultra_guard.h"

#define LOGO "/etc/ultra-logo.bin"
enum { MARGIN = 24, MAX_SIDE = 1024 };

static bool load_logo(uint8_t **pixels, uint32_t *w, uint32_t *h) {
    int fd = open(LOGO, O_RDONLY | O_CLOEXEC);
    if (fd < 0) return false;
    struct stat st;
    uint8_t head[8];
    bool ok = fstat(fd, &st) == 0 && read(fd, head, sizeof(head)) == (ssize_t)sizeof(head) && memcmp(head, "ULGO", 4) == 0;
    *w = ok ? (uint32_t)(head[4] | head[5] << 8) : 0;
    *h = ok ? (uint32_t)(head[6] | head[7] << 8) : 0;
    /* The size on disk must be exactly the header plus width x height RGBA pixels. */
    size_t need = (size_t)*w * *h * 4;
    ok = ok && *w > 0 && *h > 0 && *w <= MAX_SIDE && *h <= MAX_SIDE && st.st_size >= 0 &&
         (size_t)st.st_size == need + sizeof(head);
    *pixels = ok ? malloc(need) : NULL;
    size_t got = 0;
    while (*pixels && got < need) {
        ssize_t n = read(fd, *pixels + got, need - got);
        if (n < 0 && errno == EINTR) continue;
        if (n <= 0) break;
        got += (size_t)n;
    }
    close(fd);
    if (*pixels && got == need) return true;
    free(*pixels);
    *pixels = NULL;
    return false;
}

/* One colour channel: logo over screen, by the logo's alpha. */
static uint8_t blend(uint8_t over, uint8_t under, uint8_t alpha) {
    return (uint8_t)((over * alpha + under * (255 - alpha) + 127) / 255);
}

void show_logo(void) {
    if (!screen_available()) return;
    uint8_t *logo;
    uint32_t w, h;
    if (!load_logo(&logo, &w, &h)) {
        report("AMARILLO", "LOGO", "no se pudo leer " LOGO);
        return;
    }
    char detail[160];
    int fd = open("/dev/fb0", O_RDWR | O_CLOEXEC);
    struct fb_var_screeninfo var;
    struct fb_fix_screeninfo fix;
    bool ok = fd >= 0 && ioctl(fd, FBIOGET_VSCREENINFO, &var) == 0 && ioctl(fd, FBIOGET_FSCREENINFO, &fix) == 0;
    /* 32-bit pixels with byte-aligned channels: what simpledrm offers on UEFI. */
    ok = ok && var.bits_per_pixel == 32 && var.red.offset % 8 == 0 && var.green.offset % 8 == 0 &&
         var.blue.offset % 8 == 0 && var.red.offset < 32 && var.green.offset < 32 && var.blue.offset < 32;
    ok = ok && var.xres >= w + 2 * MARGIN && var.yres >= h + 2 * MARGIN;
    uint32_t x0 = ok ? var.xres - w - MARGIN : 0, y0 = MARGIN;
    /* The last byte touched must fall inside the mapped screen. */
    size_t last = (size_t)(y0 + h - 1) * fix.line_length + (size_t)(x0 + w) * 4;
    ok = ok && fix.line_length >= var.xres * 4 && last <= fix.smem_len;
    uint8_t *screen = ok ? mmap(NULL, fix.smem_len, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0) : MAP_FAILED;
    if (screen == MAP_FAILED) {
        snprintf(detail, sizeof(detail), "sin pantalla de 32 bits para el logo (/dev/fb0: %s)", fd < 0 ? strerror(errno) : "formato");
        report("AMARILLO", "LOGO", detail);
        if (fd >= 0) close(fd);
        free(logo);
        return;
    }
    size_t r = var.red.offset / 8, g = var.green.offset / 8, b = var.blue.offset / 8;
    for (uint32_t y = 0; y < h; y++) {
        uint8_t *row = screen + (size_t)(y0 + y) * fix.line_length + (size_t)x0 * 4;
        const uint8_t *src = logo + (size_t)y * w * 4;
        for (uint32_t x = 0; x < w; x++) {
            const uint8_t *s = src + (size_t)x * 4;
            uint8_t *d = row + (size_t)x * 4;
            if (s[3] == 0) continue;
            d[r] = blend(s[0], d[r], s[3]);
            d[g] = blend(s[1], d[g], s[3]);
            d[b] = blend(s[2], d[b], s[3]);
        }
    }
    munmap(screen, fix.smem_len);
    close(fd);
    free(logo);
    snprintf(detail, sizeof(detail), "%ux%u en la pantalla de %ux%u", w, h, var.xres, var.yres);
    report("VERDE", "LOGO", detail);
}
