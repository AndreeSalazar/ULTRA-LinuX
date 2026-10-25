#include <pthread.h>
#include <stdatomic.h>
atomic_int listo;
void *trabajo(void *x) { atomic_store(&listo, 1); return x; }
void f(void) {
    pthread_t t;
    pthread_create(&t, 0, trabajo, 0);
}
