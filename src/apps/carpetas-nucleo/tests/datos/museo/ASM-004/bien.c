#include <pthread.h>
#include <stdatomic.h>
int dato;
atomic_int listo;
void *productor(void *x) {
    dato = 42;
    atomic_store_explicit(&listo, 1, memory_order_release);
    return x;
}
