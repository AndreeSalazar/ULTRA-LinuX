#include <stdlib.h>
int *f(void) {
    static int vive = 3;
    int *p = malloc(sizeof *p);
    if (p) *p = vive;
    return p;
}
