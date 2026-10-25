#include <stdint.h>
#include <string.h>
uint32_t leer(const unsigned char *buf) {
    uint32_t v;
    memcpy(&v, buf + 1, sizeof v);
    return v;
}
