/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * Prueba de la intro de arranque sin hardware: incluye src/logo.c, dibuja los
 * cuadros reales en una pantalla falsa de 1920x1080 y comprueba que al final
 * la pantalla queda identica. Uso: scripts/probar-intro.sh [carpeta-de-cuadros]
 */
#define _GNU_SOURCE
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <stdbool.h>
#define screen_available screen_available_falso
#define report report_falso
#include "logo.c"
#undef screen_available
#undef report
bool screen_available_falso(void) { return true; }
void report_falso(const char *l, const char *c, const char *d) { fprintf(stderr, "%s [%s] %s\n", l, c, d); }
static void guardar(const uint8_t *mem, uint32_t W, uint32_t H, const char *ruta) {
    FILE *f = fopen(ruta, "wb"); fprintf(f, "P6\n%u %u\n255\n", W, H);
    for (size_t i = 0; i < (size_t)W * H; i++) { fputc(mem[i*4+2], f); fputc(mem[i*4+1], f); fputc(mem[i*4], f); }
    fclose(f);
}
int main(int argc, char **argv) {
    const uint32_t W = 1920, H = 1080; const size_t line = W * 4;
    uint8_t *mem = malloc(line * H), *orig = malloc(line * H);
    /* Fondo de consola: lineas de texto grises para comprobar la restauracion. */
    for (uint32_t y = 0; y < H; y++) for (uint32_t x = 0; x < W; x++) {
        uint8_t v = ((y / 18) % 2 == 0 && (x / 9) % 3 != 0 && y % 18 < 12) ? 150 : 8;
        uint8_t *d = mem + (size_t)y * line + (size_t)x * 4; d[0] = v; d[1] = v; d[2] = v; d[3] = 0;
    }
    memcpy(orig, mem, line * H);
    uint8_t *pic; uint32_t w, h;
    if (!load_picture(argv[1], &pic, &w, &h)) { fprintf(stderr, "sin imagen\n"); return 1; }
    uint32_t rw = w + 2 * PAD, rh = h + 2 * PAD, rx = (W - rw) / 2, ry = (H - rh) / 2;
    uint8_t *saved = malloc((size_t)rw * rh * 4);
    for (uint32_t y = 0; y < rh; y++) memcpy(saved + (size_t)y * rw * 4, mem + (size_t)(ry + y) * line + (size_t)rx * 4, (size_t)rw * 4);
    static const uint32_t blocks[FRAMES] = {32, 32, 24, 24, 16, 16, 12, 12, 8, 8, 6, 6, 4, 4, 3, 2, 1, 1};
    uint32_t state = 0x2077B305u; struct band bands[BANDS]; char ruta[512]; int n_out = 0;
    for (uint32_t k = 0; k < FRAMES; k++) {
        size_t n = intro_bands(k, h, &state, bands);
        uint32_t fade = k < 6 ? 255 * (k + 1) / 6 : 255, scan = k < 9 ? h * k / 8 : UINT32_MAX;
        intro_frame(mem, line, 2, 1, 0, rx, ry, rw, rh, pic, w, h, blocks[k], fade, bands, n, scan);
        snprintf(ruta, sizeof ruta, "%s/c%03d.ppm", argv[2], n_out++); guardar(mem, W, H, ruta);
    }
    for (int i = 0; i < 8; i++) { snprintf(ruta, sizeof ruta, "%s/c%03d.ppm", argv[2], n_out++); guardar(mem, W, H, ruta); }
    for (uint32_t k = FADE; k-- > 0;) {
        intro_frame(mem, line, 2, 1, 0, rx, ry, rw, rh, pic, w, h, 1, 255 * k / FADE, bands, 0, UINT32_MAX);
        snprintf(ruta, sizeof ruta, "%s/c%03d.ppm", argv[2], n_out++); guardar(mem, W, H, ruta);
    }
    for (uint32_t y = 0; y < rh; y++) memcpy(mem + (size_t)(ry + y) * line + (size_t)rx * 4, saved + (size_t)y * rw * 4, (size_t)rw * 4);
    snprintf(ruta, sizeof ruta, "%s/c%03d.ppm", argv[2], n_out++); guardar(mem, W, H, ruta);
    printf("cuadros %d, pantalla restaurada identica: %s\n", n_out, memcmp(mem, orig, line * H) == 0 ? "SI" : "NO");
    free(saved); free(pic); free(mem); free(orig); return 0;
}
