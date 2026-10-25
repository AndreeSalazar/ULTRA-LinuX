#include <stdarg.h>
char primero(int n, ...) {
    va_list ap;
    va_start(ap, n);
    char c = (char)va_arg(ap, int);
    va_end(ap);
    return c;
}
