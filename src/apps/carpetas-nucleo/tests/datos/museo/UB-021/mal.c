#include <pthread.h>
volatile int listo;
void *trabajo(void *x) { listo = 1; return x; }
void f(void) {
    pthread_t t;
    pthread_create(&t, 0, trabajo, 0);
}
