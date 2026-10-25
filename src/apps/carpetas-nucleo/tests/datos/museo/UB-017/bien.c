#include <string.h>
size_t f(const char *origen) {
    char dst[8];
    strncpy(dst, origen, sizeof dst);
    dst[sizeof dst - 1] = 0;
    return strlen(dst);
}
