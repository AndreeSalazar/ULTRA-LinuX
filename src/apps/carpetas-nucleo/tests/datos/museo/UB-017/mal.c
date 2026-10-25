#include <string.h>
size_t f(const char *origen) {
    char dst[8];
    strncpy(dst, origen, sizeof dst);
    return strlen(dst);
}
