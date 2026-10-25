#include <string.h>
unsigned bits(float f) {
    unsigned u;
    memcpy(&u, &f, sizeof u);
    return u;
}
