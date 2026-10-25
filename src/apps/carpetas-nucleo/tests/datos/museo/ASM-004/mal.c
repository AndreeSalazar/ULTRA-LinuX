#include <pthread.h>
int dato, listo;
void *productor(void *x) {
    dato = 42;
    __asm__ volatile("" ::: "memory");
    listo = 1;
    return x;
}
