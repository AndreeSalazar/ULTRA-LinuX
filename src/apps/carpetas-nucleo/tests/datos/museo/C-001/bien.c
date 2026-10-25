#include <stdlib.h>
void f(void) {
    char *p = malloc(16);
    if (!p) return;
    char *vista = p + 1;
    (void)vista;
    free(p);
}
