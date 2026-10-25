#include <stdarg.h>
char primero(int n, ...) {
    va_list ap;
    va_start(ap, n);
    char c = va_arg(ap, char);
    va_end(ap);
    return c;
}
