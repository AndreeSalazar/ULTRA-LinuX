/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * The ULTRA logo, drawn by the init on the firmware screen through /dev/fb0:
 * top-right corner, blended by its alpha over whatever is already there.
 * The kernel stays unmodified; the picture travels in the initramfs as
 * /etc/ultra-logo.bin ("ULGO", u16 width, u16 height, RGBA pixels), made by
 * scripts/convertir_logo.py. Based on Tux by Larry Ewing (lewing@isc.tamu.edu),
 * drawn with The GIMP.
 *
 * The boot intro uses the same format: /etc/ultra-intro.bin, Eddi Salazar's own
 * Tux, made by scripts/convertir_intro.py. It is animated in the centre for about
 * 1.2 s (pixel blocks that sharpen, red glitch bands, a scan line, a fade) and the
 * screen underneath is put back afterwards. Without the file it is skipped.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <linux/fb.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>
#include "init.h"
/* Last: it poisons unsafe functions after the system headers are parsed. */
#include "guardian/ultra_guard.h"

#define LOGO "/etc/ultra-logo.bin"
#define INTRO "/etc/ultra-intro.bin"
enum { MARGIN = 24, MAX_SIDE = 1024 };

static bool load_picture(const char *path, uint8_t **pixels, uint32_t *w, uint32_t *h) {
    int fd = open(path, O_RDONLY | O_CLOEXEC);
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

/* The mapped firmware screen: 32-bit pixels with byte-aligned channels. */
struct screen {
    int fd;
    uint8_t *mem;
    size_t len, line, r, g, b;
    uint32_t xres, yres;
};

static bool open_screen(struct screen *s, char *why, size_t why_len) {
    struct fb_var_screeninfo var;
    struct fb_fix_screeninfo fix;
    s->fd = open("/dev/fb0", O_RDWR | O_CLOEXEC);
    s->mem = MAP_FAILED;
    bool ok = s->fd >= 0 && ioctl(s->fd, FBIOGET_VSCREENINFO, &var) == 0 && ioctl(s->fd, FBIOGET_FSCREENINFO, &fix) == 0;
    /* 32-bit pixels with byte-aligned channels: what simpledrm offers on UEFI. */
    ok = ok && var.bits_per_pixel == 32 && var.red.offset % 8 == 0 && var.green.offset % 8 == 0 &&
         var.blue.offset % 8 == 0 && var.red.offset < 32 && var.green.offset < 32 && var.blue.offset < 32;
    ok = ok && var.xres > 0 && var.yres > 0 && fix.line_length >= var.xres * 4 &&
         (size_t)var.yres * fix.line_length <= fix.smem_len;
    if (ok) s->mem = mmap(NULL, fix.smem_len, PROT_READ | PROT_WRITE, MAP_SHARED, s->fd, 0);
    if (s->mem == MAP_FAILED) {
        snprintf(why, why_len, "/dev/fb0: %s", s->fd < 0 ? strerror(errno) : "formato");
        if (s->fd >= 0) close(s->fd);
        s->fd = -1;
        return false;
    }
    s->len = fix.smem_len;
    s->line = fix.line_length;
    s->xres = var.xres;
    s->yres = var.yres;
    s->r = var.red.offset / 8;
    s->g = var.green.offset / 8;
    s->b = var.blue.offset / 8;
    return true;
}

static void close_screen(struct screen *s) {
    if (s->mem != MAP_FAILED) munmap(s->mem, s->len);
    if (s->fd >= 0) close(s->fd);
}

void show_logo(void) {
    if (!screen_available()) return;
    uint8_t *logo;
    uint32_t w, h;
    if (!load_picture(LOGO, &logo, &w, &h)) {
        report("AMARILLO", "LOGO", "no se pudo leer " LOGO);
        return;
    }
    char detail[160], why[96];
    struct screen s;
    bool ok = open_screen(&s, why, sizeof(why));
    if (ok && (s.xres < w + 2 * MARGIN || s.yres < h + 2 * MARGIN)) {
        snprintf(why, sizeof(why), "pantalla de %ux%u", s.xres, s.yres);
        ok = false;
    }
    if (!ok) {
        snprintf(detail, sizeof(detail), "sin pantalla de 32 bits para el logo (%s)", why);
        report("AMARILLO", "LOGO", detail);
        close_screen(&s);
        free(logo);
        return;
    }
    uint32_t x0 = s.xres - w - MARGIN, y0 = MARGIN;
    for (uint32_t y = 0; y < h; y++) {
        uint8_t *row = s.mem + (size_t)(y0 + y) * s.line + (size_t)x0 * 4;
        const uint8_t *src = logo + (size_t)y * w * 4;
        for (uint32_t x = 0; x < w; x++) {
            const uint8_t *p = src + (size_t)x * 4;
            uint8_t *d = row + (size_t)x * 4;
            if (p[3] == 0) continue;
            d[s.r] = blend(p[0], d[s.r], p[3]);
            d[s.g] = blend(p[1], d[s.g], p[3]);
            d[s.b] = blend(p[2], d[s.b], p[3]);
        }
    }
    close_screen(&s);
    free(logo);
    snprintf(detail, sizeof(detail), "%ux%u en la pantalla de %ux%u", w, h, s.xres, s.yres);
    report("VERDE", "LOGO", detail);
}

/* ---------- Boot intro ---------- */
enum { PAD = 48, FRAMES = 18, FRAME_MS = 40, HOLD_MS = 320, FADE = 5, FADE_MS = 30, BANDS = 7 };

/* A small fixed sequence: the same glitch on every boot. */
static uint32_t next_random(uint32_t *state) {
    uint32_t x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    return x;
}

static void pause_ms(long ms) {
    struct timespec t = {.tv_sec = ms / 1000, .tv_nsec = (ms % 1000) * 1000000L};
    while (nanosleep(&t, &t) < 0 && errno == EINTR) {}
}

/* Rows of the region shifted sideways in this frame; tint marks a red glitch band. */
struct band {
    uint32_t top, height;
    int32_t shift;
    bool tint;
};

/*
 * One frame of the intro inside the region (rx, ry, rw, rh) of a 32-bit screen.
 * block: pixel block size (32 down to 1). fade: 0..255 over black.
 * scan: the row of the red scan line inside the picture, or UINT32_MAX for none.
 */
static void intro_frame(uint8_t *mem, size_t line, size_t r, size_t g, size_t b, uint32_t rx, uint32_t ry, uint32_t rw, uint32_t rh,
                 const uint8_t *pic, uint32_t w, uint32_t h, uint32_t block, uint32_t fade, const struct band *bands,
                 size_t nbands, uint32_t scan) {
    for (uint32_t y = 0; y < rh; y++) {
        uint8_t *row = mem + (size_t)(ry + y) * line + (size_t)rx * 4;
        for (uint32_t x = 0; x < rw; x++) {
            uint8_t *d = row + (size_t)x * 4;
            d[r] = 0;
            d[g] = 0;
            d[b] = 0;
        }
    }
    uint32_t px0 = rx + PAD, py0 = ry + PAD;
    for (uint32_t by = 0; by < h; by += block) {
        for (uint32_t bx = 0; bx < w; bx += block) {
            uint32_t sy = by + block / 2 < h ? by + block / 2 : h - 1;
            uint32_t sx = bx + block / 2 < w ? bx + block / 2 : w - 1;
            const uint8_t *p = pic + ((size_t)sy * w + sx) * 4;
            uint32_t alpha = (uint32_t)p[3] * fade / 255;
            if (alpha == 0) continue;
            for (uint32_t y = by; y < by + block && y < h; y++) {
                int32_t shift = 0;
                bool tint = false;
                for (size_t i = 0; i < nbands; i++) {
                    if (y >= bands[i].top && y < bands[i].top + bands[i].height) {
                        shift = bands[i].shift;
                        tint = bands[i].tint;
                    }
                }
                for (uint32_t x = bx; x < bx + block && x < w; x++) {
                    /* PAD >= the largest shift, so the pixel stays inside the region. */
                    uint32_t dx = (uint32_t)((int32_t)(px0 + x) + shift);
                    uint8_t *d = mem + (size_t)(py0 + y) * line + (size_t)dx * 4;
                    uint32_t red = (uint32_t)p[0] * alpha / 255, green = (uint32_t)p[1] * alpha / 255,
                             blue = (uint32_t)p[2] * alpha / 255;
                    if (tint) {
                        red = red + (255 - red) / 2;
                        green /= 4;
                        blue /= 3;
                    }
                    d[r] = (uint8_t)red;
                    d[g] = (uint8_t)green;
                    d[b] = (uint8_t)blue;
                }
            }
        }
    }
    if (scan < h) {
        /* A red line with two softer rows under it, across the whole region. */
        for (uint32_t k = 0; k < 3 && scan + k < h; k++) {
            uint8_t *row = mem + (size_t)(py0 + scan + k) * line + (size_t)rx * 4;
            uint32_t level = 255u >> k;
            for (uint32_t x = 0; x < rw; x++) {
                uint8_t *d = row + (size_t)x * 4;
                d[r] = (uint8_t)(d[r] > level ? d[r] : level);
                d[g] = (uint8_t)(d[g] * 2 / 3);
                d[b] = (uint8_t)(d[b] * 2 / 3);
            }
        }
    }
}

/* Bands for frame k: wide and frequent at the start, gone by frame 12. */
static size_t intro_bands(uint32_t k, uint32_t h, uint32_t *state, struct band *bands) {
    if (k >= 12) return 0;
    uint32_t reach = 36 - 3 * k;
    size_t n = k < 6 ? (size_t)BANDS : (size_t)BANDS / 2;
    for (size_t i = 0; i < n; i++) {
        bands[i].top = next_random(state) % h;
        bands[i].height = 3 + next_random(state) % 28;
        bands[i].shift = (int32_t)(next_random(state) % (2 * reach + 1)) - (int32_t)reach;
        bands[i].tint = next_random(state) % 3 == 0;
    }
    return n;
}

void show_intro(void) {
    if (!screen_available()) return;
    uint8_t *pic;
    uint32_t w, h;
    if (!load_picture(INTRO, &pic, &w, &h)) {
        report("AMARILLO", "INTRO", "no se pudo leer " INTRO);
        return;
    }
    char detail[160], why[96];
    struct screen s;
    bool ok = open_screen(&s, why, sizeof(why));
    uint32_t rw = w + 2 * PAD, rh = h + 2 * PAD;
    if (ok && (s.xres < rw || s.yres < rh)) {
        snprintf(why, sizeof(why), "pantalla de %ux%u", s.xres, s.yres);
        ok = false;
    }
    /* What was on the screen under the intro, to put it back at the end. */
    size_t saved_len = (size_t)rw * rh * 4;
    uint8_t *saved = ok ? malloc(saved_len) : NULL;
    if (!ok || !saved) {
        snprintf(detail, sizeof(detail), "intro omitida (%s)", ok ? "sin memoria" : why);
        report("AMARILLO", "INTRO", detail);
        close_screen(&s);
        free(pic);
        return;
    }
    uint32_t rx = (s.xres - rw) / 2, ry = (s.yres - rh) / 2;
    for (uint32_t y = 0; y < rh; y++)
        memcpy(saved + (size_t)y * rw * 4, s.mem + (size_t)(ry + y) * s.line + (size_t)rx * 4, (size_t)rw * 4);

    static const uint32_t blocks[FRAMES] = {32, 32, 24, 24, 16, 16, 12, 12, 8, 8, 6, 6, 4, 4, 3, 2, 1, 1};
    uint32_t state = 0x2077B305u;
    struct band bands[BANDS];
    for (uint32_t k = 0; k < FRAMES; k++) {
        size_t n = intro_bands(k, h, &state, bands);
        uint32_t fade = k < 6 ? 255 * (k + 1) / 6 : 255;
        uint32_t scan = k < 9 ? h * k / 8 : UINT32_MAX;
        intro_frame(s.mem, s.line, s.r, s.g, s.b, rx, ry, rw, rh, pic, w, h, blocks[k], fade, bands, n, scan);
        pause_ms(FRAME_MS);
    }
    pause_ms(HOLD_MS);
    for (uint32_t k = FADE; k-- > 0;) {
        intro_frame(s.mem, s.line, s.r, s.g, s.b, rx, ry, rw, rh, pic, w, h, 1, 255 * k / FADE, bands, 0, UINT32_MAX);
        pause_ms(FADE_MS);
    }
    for (uint32_t y = 0; y < rh; y++)
        memcpy(s.mem + (size_t)(ry + y) * s.line + (size_t)rx * 4, saved + (size_t)y * rw * 4, (size_t)rw * 4);
    free(saved);
    close_screen(&s);
    free(pic);
    snprintf(detail, sizeof(detail), "%ux%u, %d cuadros en la pantalla de %ux%u", w, h, FRAMES + FADE, s.xres, s.yres);
    report("VERDE", "INTRO", detail);
}
