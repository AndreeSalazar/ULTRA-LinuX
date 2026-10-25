#include <stdint.h>
uint32_t leer(const unsigned char *buf) {
    return *(uint32_t *)(buf + 1);
}
