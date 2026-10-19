/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * Runtime minefield. Each mine triggers one classic C failure on purpose.
 * A guardian must stop it; reaching "MINA NO DETECTADA" means no guardian woke.
 * Built three times: production (traps, FORTIFY, CET), diagnosis (ASan + UBSan)
 * and raw (every guardian off) to show what the mine does when nobody watches.
 * volatile keeps GCC from folding the mine away at compile time.
 *
 * Kinds: 'D' mines must be STOPPED. 'N' mines cannot be seen at run time, so a
 * guardian must NEUTRALIZE them: turn the UB into one defined, checked result.
 */
#define _GNU_SOURCE
#include <limits.h>
#include <stdbool.h>
#include <stdint.h>
#include <unistd.h>
#include "ultra_guard.h"

static volatile int zero = 0, big = INT_MAX, shift = 40, far_index = 6, minus_one = -1, int_min = INT_MIN;
static volatile size_t len16 = 16, huge_count = SIZE_MAX / 16 + 2, big_alloca = (size_t)64 << 20;

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

static void mine_vla_negative(void) { int n = minus_one; char buf[n]; buf[0] = 1; volatile char r = buf[0]; (void)r; }
static void mine_unreachable(void) { if (zero == 0) __builtin_unreachable(); }
static __attribute__((noinline, nonnull(1))) size_t count_chars(const char *s) { return strlen(s); }
static void mine_nonnull(void) { const char *volatile p = NULL; volatile size_t r = count_chars(p); (void)r; }
static void mine_size_wrap(void) {
    size_t n = huge_count * 16; /* unsigned: wraps to 16 without any UB */
    char *p = malloc(n);
    if (!p) return;
    memset(p, 0, 64);
    volatile char r = p[0]; (void)r;
    free(p);
}
static __attribute__((noinline)) int deep(int d) {
    volatile char pad[256];
    pad[0] = (char)d;
    return deep(d + 1) + pad[0];
}
static void mine_recursion(void) { volatile int r = deep(0); (void)r; }
static void mine_alloca(void) { char *p = __builtin_alloca(big_alloca); p[0] = 1; volatile char r = p[0]; (void)r; }
static void mine_free_stack(void) { char b[16]; char *volatile p = b; free(p); }
static void mine_free_middle(void) { char *volatile p = malloc(64); free(p + 8); }
static void mine_heap_underflow(void) {
    char *volatile p = malloc(64);
    if (!p) return;
    for (int i = 1; i <= 16; i++) p[-i] = 0x41;
    free(p);
}
static void mine_format_n(void) {
    char fmt[16];
    ULTRA_COPY(fmt, "abc%n\n");
    int n = 0;
    printf(fmt, &n);
}
static void mine_overlap(void) {
    char b[32] = "0123456789abcdefghijklmnopqrstu";
    char *volatile p = b;
    memcpy(p + 4, p, len16); /* a runtime size: a constant 16 is inlined and no library check runs */
    volatile char r = b[5]; (void)r;
}
static void mine_read_past(void) { int a[4] = {1, 2, 3, 4}; int *volatile p = a; volatile int r = p[6]; (void)r; }
static __attribute__((noinline)) int *local_address(void) { int x = 42; int *volatile p = &x; return p; }
static void mine_use_after_return(void) { int *p = local_address(); volatile int r = *p; (void)r; }
static void mine_int_min_div(void) { volatile int r = int_min / minus_one; (void)r; }
static void mine_negative_shift_base(void) { volatile int r = minus_one << 1; (void)r; }
static void mine_abs_int_min(void) { volatile int r = abs(int_min); (void)r; }
static void mine_exec_stack(void) {
    unsigned char code[16] = {0xc3}; /* ret */
    void (*volatile f)(void) = (void (*)(void))(void *)code;
    f();
}
static void mine_write_literal(void) { char *volatile s = (char *)"literal"; s[0] = 'X'; }
extern void (*__init_array_start[])(void);
static void mine_relro(void) {
    void (*volatile *slot)(void) = __init_array_start;
    *slot = escaped; /* overwriting a startup pointer: the classic GOT/init_array hijack */
    fputs("MINA NO DETECTADA: init_array escribible, el arranque quedo secuestrable\n", stderr);
    exit(99);
}
static void mine_sign_size(void) {
    char dst[16];
    int n = minus_one;
    memcpy(dst, "x", (size_t)n); /* the explicit cast silences -Wsign-conversion on purpose */
    volatile char r = dst[0]; (void)r;
}

/* A neutralized mine reports its result: defined means a guardian made the UB harmless. */
static void neutral(bool defined, long value, const char *expected) {
    if (defined) {
        printf("NEUTRALIZADA: resultado definido %ld (%s)\n", value, expected);
        exit(0);
    }
    fprintf(stderr, "UB VISIBLE: resultado %ld, se esperaba %s\n", value, expected);
    exit(99);
}
static __attribute__((noinline)) int alias_read(int *i, float *f) {
    *i = 1;
    *f = 0.0f;
    return *i; /* with strict aliasing GCC assumes f cannot touch i and returns 1 */
}
static void mine_strict_alias(void) {
    int x = 0;
    int r = alias_read(&x, (float *)(void *)&x);
    neutral(r == 0, r, "0: la memoria real");
}
static __attribute__((noinline)) void dirty_stack(void) {
    volatile int junk[64];
    for (int i = 0; i < 64; i++) junk[i] = 0x41414141;
}
static __attribute__((noinline)) int read_uninit(int set) {
    int x;
    if (set) x = 5;
    return x;
}
static void mine_uninit_runtime(void) {
    dirty_stack();
    int r = read_uninit(zero);
    neutral(r == 0, r, "0: variable puesta a cero por el guardian");
}

static void mine_checked_add(void) { volatile int r = ULTRA_ADD(big, 1); (void)r; }
static void mine_checked_sub(void) { volatile int r = ULTRA_SUB(int_min, 1); (void)r; }
static void mine_checked_mul(void) { volatile int r = ULTRA_MUL(big, 2); (void)r; }
static void mine_checked_size(void) { volatile size_t r = ULTRA_MUL(huge_count, (size_t)16); (void)r; }
static void mine_sub_overflow(void) { volatile int r = int_min - 1; (void)r; }
static void mine_negative_shift(void) { volatile int r = 1 << minus_one; (void)r; }
static void mine_checked_index(void) { int a[4] = {0}; ULTRA_AT(a, far_index) = 1; }
static void mine_mul_overflow(void) { volatile int r = big * 2; (void)r; }
static void mine_negate_min(void) { volatile int r = -int_min; (void)r; }
static void mine_remainder_min(void) { volatile int r = int_min % minus_one; (void)r; }
static void mine_vla_zero(void) { int n = zero; char a[n]; a[0] = 1; volatile char r = a[0]; (void)r; }
static void mine_negative_index(void) { int a[4] = {0}; volatile int r = a[minus_one]; (void)r; }
static void mine_subobject(void) {
    struct { int a[2]; int b[2]; } s = {{1, 2}, {3, 4}};
    volatile int idx = 2;
    volatile int r = s.a[idx]; (void)r;
}
static void mine_unterminated(void) {
    char *volatile p = malloc(8);
    if (!p) exit(98);
    memset(p, 'x', 8);
    volatile size_t r = strlen(p); (void)r;
    free(p);
}
static void mine_after_scope(void) {
    int *volatile p;
    { int a = 42; p = &a; }
    volatile int r = *p; (void)r;
}
static void mine_pointer_compare(void) {
    char *volatile a = malloc(8), *volatile b = malloc(8);
    if (!a || !b) exit(98);
    volatile int r = a < b; (void)r;
    free(a); free(b);
}
static void mine_pointer_subtract(void) {
    char *volatile a = malloc(8), *volatile b = malloc(8);
    if (!a || !b) exit(98);
    volatile ptrdiff_t r = a - b; (void)r;
    free(a); free(b);
}

struct mine {
    const char *name;
    char kind;
    const char *why;
    void (*run)(void);
    const char *twin; /* compile-time mine that stops the same failure before it can run */
};

static const struct mine mines[] = {
    {"producto-desborde", 'D', "multiplicacion con signo fuera de rango", mine_mul_overflow},
    {"negar-int-min", 'D', "-INT_MIN no cabe en int", mine_negate_min},
    {"resto-int-min", 'D', "INT_MIN % -1 tambien es UB", mine_remainder_min},
    {"vla-cero", 'D', "VLA con longitud cero viola C17", mine_vla_zero},
    {"indice-negativo", 'D', "indice anterior al inicio del arreglo", mine_negative_index},
    {"subobjeto-fuera", 'D', "cruzar del arreglo a al vecino b no valida el indice de a", mine_subobject},
    {"cadena-sin-nul", 'D', "strlen necesita un terminador dentro del objeto", mine_unterminated},
    {"uso-tras-scope", 'D', "la vida de una local termina al salir de su bloque", mine_after_scope},
    {"comparar-punteros-ajenos", 'D', "comparacion relacional de dos reservas distintas", mine_pointer_compare},
    {"restar-punteros-ajenos", 'D', "resta de punteros que no pertenecen al mismo arreglo", mine_pointer_subtract},
    {"entero-desborde", 'D', "int con signo pasa de INT_MAX: en C es UB y el optimizador puede borrar tus comprobaciones", mine_overflow},
    {"division-cero", 'D', "dividir por cero es UB; en x86 suele matar, pero el compilador puede asumir que nunca pasa", mine_divide},
    {"desplazamiento", 'D', "desplazar 40 bits un int de 32 es UB; el resultado depende de la CPU", mine_shift},
    {"indice-fuera", 'D', "escribir a[6] en un arreglo de 4 pisa memoria vecina sin avisar", mine_index},
    {"copia-pila", 'D', "memcpy de 16 bytes en un buffer de 8: clasico desborde de pila", mine_stack_copy},
    {"desborde-heap", 'D', "memset de 16 bytes en un malloc de 8 corrompe el heap", mine_heap_overflow},
    {"uso-tras-free", 'D', "leer memoria ya liberada: puede contener datos de otro objeto", mine_use_after_free, "analizador-uso-tras-free"},
    {"doble-free", 'D', "liberar dos veces corrompe el allocator; base de muchos exploits", mine_double_free},
    {"puntero-nulo", 'D', "escribir a traves de NULL; el compilador puede asumir que nunca es NULL", mine_null},
    {"pila-destruida", 'D', "un bucle escribe 64 bytes en un buffer de 8 hasta pisar el retorno", mine_smash},
    {"bool-invalido", 'D', "un bool con valor 7 rompe la logica: ni true ni false", mine_bool},
    {"desalineado", 'D', "leer un int desde una direccion impar es UB en C", mine_misaligned},
    {"puntero-vuelta", 'D', "aritmetica de punteros que da la vuelta al espacio de direcciones", mine_pointer_wrap},
    {"float-a-int", 'D', "convertir 1e20 a int no cabe: UB", mine_float_cast},
    {"retorno-secuestrado", 'D', "sobrescribir la direccion de retorno: base del ROP; solo CET lo bloquea en el retorno", mine_return_hijack},
    {"ultra-suma", 'D', "ULTRA_ADD: el guardian explicito comprueba la suma antes de usarla", mine_checked_add},
    {"ultra-resta", 'D', "ULTRA_SUB debe rechazar INT_MIN menos uno", mine_checked_sub},
    {"ultra-producto", 'D', "ULTRA_MUL debe rechazar producto con signo fuera de rango", mine_checked_mul},
    {"tamano-producto", 'D', "size_t puede envolver legalmente: comprobar antes de reservar memoria", mine_checked_size},
    {"resta-desborde", 'D', "restar uno a INT_MIN es UB", mine_sub_overflow},
    {"desplazamiento-negativo", 'D', "un contador de desplazamiento negativo es UB", mine_negative_shift},
    {"ultra-indice", 'D', "ULTRA_AT: el guardian explicito comprueba el indice antes de acceder", mine_checked_index},
    {"vla-negativa", 'D', "un arreglo de tamano -1 en la pila: UB, la pila se rompe", mine_vla_negative},
    {"inalcanzable", 'D', "llegar a __builtin_unreachable: el compilador borro el codigo que seguia", mine_unreachable},
    {"nonnull", 'D', "pasar NULL a una funcion que prometio no recibirlo", mine_nonnull},
    {"tamano-vuelta", 'D', "count*16 da la vuelta a 16 (sin UB); malloc pequeno y escritura grande: el clasico de los exploits", mine_size_wrap},
    {"recursion-infinita", 'D', "la pila crece hasta la pagina de guarda", mine_recursion},
    {"alloca-gigante", 'D', "alloca de 64 MiB salta la pagina de guarda hacia otra memoria", mine_alloca},
    {"free-pila", 'D', "liberar memoria que no vino de malloc", mine_free_stack},
    {"free-medio", 'D', "liberar un puntero desplazado dentro del bloque", mine_free_middle},
    {"heap-subdesborde", 'D', "escribir antes del bloque pisa la cabecera del allocator", mine_heap_underflow},
    {"formato-n", 'D', "%n en un formato escribible: printf escribe en memoria", mine_format_n},
    {"memcpy-solapado", 'D', "memcpy con origen y destino solapados es UB; usar memmove", mine_overlap, "memcpy-solapado-constante"},
    {"lectura-fuera", 'D', "leer p[6] de un arreglo de 4 a traves de un puntero: filtra memoria vecina", mine_read_past},
    {"uso-tras-retorno", 'D', "usar la direccion de una local de una funcion que ya volvio", mine_use_after_return, "retorna-local"},
    {"int-min-entre-menos-uno", 'D', "INT_MIN / -1 desborda: UB y excepcion en x86", mine_int_min_div},
    {"desplazar-negativo", 'D', "desplazar a la izquierda un numero negativo es UB en C17", mine_negative_shift_base},
    {"abs-int-min", 'D', "abs(INT_MIN) no cabe en int", mine_abs_int_min},
    {"ejecutar-pila", 'D', "saltar a bytes en la pila: inyeccion de codigo", mine_exec_stack},
    {"escribir-literal", 'D', "escribir en un literal de cadena, que vive en memoria de solo lectura", mine_write_literal},
    {"secuestrar-arranque", 'D', "sobrescribir un puntero de init_array, como un ataque a la GOT", mine_relro},
    {"signo-tamano", 'D', "un cast explicito convierte -1 en tamano gigante y calla al guardian de compilacion", mine_sign_size},
    {"alias-estricto", 'N', "leer un int por un float*: con alias estricto el optimizador devuelve un valor viejo", mine_strict_alias},
    {"sin-inicializar-ejecucion", 'N', "leer una local sin valor; medido: GCC 16 -O2 la convierte en 5 aunque el guardian la ponga a cero", mine_uninit_runtime, "sin-inicializar"},
};

int main(int argc, char **argv) {
#ifndef ULTRA_CRUDO
    ultra_guard_install("minas");
#endif
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
        for (size_t i = 0; i < ULTRA_LEN(mines); i++) printf("%s|%c|%s|%s\n", mines[i].name, mines[i].kind, mines[i].why, mines[i].twin ? mines[i].twin : "");
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
