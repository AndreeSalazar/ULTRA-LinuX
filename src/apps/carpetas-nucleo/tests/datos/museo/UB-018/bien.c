#include <string.h>
void f(char *buf, size_t n) {
    memmove(buf, buf + 1, n);
}
