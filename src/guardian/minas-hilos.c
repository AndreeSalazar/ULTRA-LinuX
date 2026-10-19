/* SPDX-License-Identifier: GPL-2.0-only */
#define _GNU_SOURCE
#include <pthread.h>
#include <stdatomic.h>
#include <stdbool.h>
#include <stdio.h>
#include <string.h>

static pthread_barrier_t gate;
static atomic_int synchronized_count;
static volatile int raced_count;
static bool safe;

static void *worker(void *unused) {
    (void)unused;
    int r = pthread_barrier_wait(&gate);
    if (r != 0 && r != PTHREAD_BARRIER_SERIAL_THREAD) return (void *)1;
    for (int i = 0; i < 10000; i++) {
        if (safe) atomic_fetch_add_explicit(&synchronized_count, 1, memory_order_relaxed);
        else raced_count++; /* Volatile does not synchronize threads. Intentional UB. */
    }
    return NULL;
}

int main(int argc, char **argv) {
    if (argc != 2 || (strcmp(argv[1], "race") && strcmp(argv[1], "atomic"))) return 2;
    safe = !strcmp(argv[1], "atomic");
    if (pthread_barrier_init(&gate, NULL, 2) != 0) return 3;
    pthread_t threads[2];
    for (size_t i = 0; i < 2; i++) if (pthread_create(&threads[i], NULL, worker, NULL) != 0) return 4;
    for (size_t i = 0; i < 2; i++) {
        void *result = NULL;
        if (pthread_join(threads[i], &result) != 0 || result != NULL) return 5;
    }
    if (pthread_barrier_destroy(&gate) != 0) return 6;
    if (!safe) { puts("MINA NO DETECTADA: carrera sin bandera"); return 99; }
    if (atomic_load_explicit(&synchronized_count, memory_order_relaxed) != 20000) return 7;
    puts("HILOS-OK: contador atomico=20000");
    return 0;
}
