#include <stdlib.h>
void f(void) {
    char *p = malloc(16);
    if (!p) return;
    free(p + 1);
}
