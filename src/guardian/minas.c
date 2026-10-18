/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * Runtime minefield. Each mine triggers one classic C failure on purpose.
 * A guardian must stop it; reaching "MINA NO DETECTADA" means no guardian woke.
 * Built twice: production (traps, FORTIFY, CET) and diagnosis (ASan + UBSan).
 * volatile keeps GCC from folding the mine away at compile time.
 */
#define _GNU_SOURCE
#include <limits.h>
#include <stdbool.h>
#include <stdint.h>
#include <unistd.h>
#include "ultra_guard.h"

static volatile int zero = 0, big = INT_MAX, shift = 40, far_index = 6;
static volatile size_t len16 = 16;

static void escaped(void) {
    fputs("MINA NO DETECTADA: el programa siguio tras el fallo\n", stderr);
    exit(99);
}

static void mine_overflow(void) { volatile int r = big + 1; (void)r; }
static void mine_divide(void) { volatile int r = 1 / zero; (void)r; }
static void mine_shift(void) { volatile int r = 1 << shift; (void)r; }
static void mine_index(void) { int a[4] = {0}; a[far_index] = 1; volatile int r = a[0]; (void)r; }
static void mine_stack_copy(void) {
    char dst[8];
    const char src[32] = "esto no cabe en ocho bytes";
    memcpy(dst, src, len16);
    volatile char r = dst[0]; (void)r;
}
static void mine_heap_overflow(void) {
    char *p = malloc(8);
    if (!p) return;
    memset(p, 'x', len16);
    volatile char r = p[0]; (void)r;
    free(p);
}
static void mine_use_after_free(void) {
    char *volatile p = malloc(64);
    if (!p) return;
    p[0] = 'a';
    free(p);
    volatile char r = p[0]; (void)r;
}
static void mine_double_free(void) {
    char *volatile p = malloc(64);
    free(p);
    free(p);
}
static void mine_null(void) { int *volatile p = NULL; *p = 1; }
static void mine_smash(void) {
    char buf[8];
    char *volatile q = buf;
    for (size_t i = 0; i < 64; i++) q[i] = 'Z';
}
static void mine_bool(void) {
    unsigned char raw = 7;
    bool b;
    memcpy(&b, &raw, 1);
    volatile int r = b ? 1 : 2; (void)r;
}
static void mine_misaligned(void) {
    _Alignas(8) char buf[16] = {0};
    int *volatile p = (int *)(buf + 1);
    volatile int r = *p; (void)r;
}
static void mine_pointer_wrap(void) {
    char *volatile p = (char *)(UINTPTR_MAX - 2);
    char *volatile q = p + len16;
    (void)q;
}
static void mine_float_cast(void) { volatile double d = 1e20; volatile int r = (int)d; (void)r; }

/* Hijacks its own return address without touching the stack canary. */
static __attribute__((noinline)) void hijack_target(void) {
    fputs("MINA NO DETECTADA: retorno secuestrado ejecuto codigo ajeno\n", stderr);
    _exit(99);
}
static __attribute__((noinline)) void mine_return_hijack(void) {
    void **ret = (void **)__builtin_frame_address(0) + 1;
    *(void *volatile *)ret = (void *)hijack_target;
}

static void mine_checked_add(void) { volatile int r = ULTRA_ADD(big, 1); (void)r; }
static void mine_checked_index(void) { int a[4] = {0}; ULTRA_AT(a, far_index) = 1; }

struct mine {
    const char *name;
    const char *why;
    void (*run)(void);
};

static const struct mine mines[] = {
    {"entero-desborde", "int con signo pasa de INT_MAX: en C es UB y el optimizador puede borrar tus comprobaciones", mine_overflow},
    {"division-cero", "dividir por cero es UB; en x86 suele matar, pero el compilador puede asumir que nunca pasa", mine_divide},
    {"desplazamiento", "desplazar 40 bits un int de 32 es UB; el resultado depende de la CPU", mine_shift},
    {"indice-fuera", "escribir a[6] en un arreglo de 4 pisa memoria vecina sin avisar", mine_index},
    {"copia-pila", "memcpy de 16 bytes en un buffer de 8: clasico desborde de pila", mine_stack_copy},
    {"desborde-heap", "memset de 16 bytes en un malloc de 8 corrompe el heap", mine_heap_overflow},
    {"uso-tras-free", "leer memoria ya liberada: puede contener datos de otro objeto", mine_use_after_free},
    {"doble-free", "liberar dos veces corrompe el allocator; base de muchos exploits", mine_double_free},
    {"puntero-nulo", "escribir a traves de NULL; el compilador puede asumir que nunca es NULL", mine_null},
    {"pila-destruida", "un bucle escribe 64 bytes en un buffer de 8 hasta pisar el retorno", mine_smash},
    {"bool-invalido", "un bool con valor 7 rompe la logica: ni true ni false", mine_bool},
    {"desalineado", "leer un int desde una direccion impar es UB en C", mine_misaligned},
    {"puntero-vuelta", "aritmetica de punteros que da la vuelta al espacio de direcciones", mine_pointer_wrap},
    {"float-a-int", "convertir 1e20 a int no cabe: UB", mine_float_cast},
    {"retorno-secuestrado", "sobrescribir la direccion de retorno: base del ROP; solo CET lo bloquea en el retorno", mine_return_hijack},
    {"ultra-suma", "ULTRA_ADD: el guardian explicito comprueba la suma antes de usarla", mine_checked_add},
    {"ultra-indice", "ULTRA_AT: el guardian explicito comprueba el indice antes de acceder", mine_checked_index},
};

int main(int argc, char **argv) {
    ultra_guard_install("minas");
    if (argc == 2 && !strcmp(argv[1], "--cet")) {
        /* The kernel reports the shadow stack per thread; empty means CET only marked the binary. */
        FILE *f = fopen("/proc/self/status", "r");
        char line[256];
        bool on = false;
        while (f && fgets(line, sizeof(line), f))
            if (!strncmp(line, "x86_Thread_features:", 20) && strstr(line, "shstk")) on = true;
        if (f) fclose(f);
        puts(on ? "ACTIVA (shstk)" : "INACTIVA");
        return 0;
    }
    if (argc == 2 && !strcmp(argv[1], "--lista")) {
        for (size_t i = 0; i < ULTRA_LEN(mines); i++) printf("%s|%s\n", mines[i].name, mines[i].why);
        return 0;
    }
    for (size_t i = 0; argc == 2 && i < ULTRA_LEN(mines); i++) {
        if (strcmp(argv[1], mines[i].name)) continue;
        fprintf(stderr, "MINA %s: %s\n", mines[i].name, mines[i].why);
        mines[i].run();
        escaped();
    }
    fputs("uso: minas --lista | minas NOMBRE\n", stderr);
    return 2;
}
