#include <ctype.h>
int cuenta(const char *s) {
    int n = 0;
    while (*s) { if (isdigit(*s)) n++; s++; }
    return n;
}
