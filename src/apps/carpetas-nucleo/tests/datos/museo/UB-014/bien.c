#include <setjmp.h>
jmp_buf salto;
int f(void) {
    volatile int n = 0;
    if (setjmp(salto)) return n;
    n = 5;
    longjmp(salto, 1);
}
