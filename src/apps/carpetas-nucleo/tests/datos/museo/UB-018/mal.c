#include <string.h>
void f(char *buf, size_t n) {
    memcpy(buf, buf + 1, n);
}
