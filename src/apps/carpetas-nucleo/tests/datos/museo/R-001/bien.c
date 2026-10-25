#include <string.h>
int cuenta_a(const char *s) {
    int n = 0;
    size_t largo = strlen(s);
    for (size_t i = 0; i < largo; i++) {
        if (s[i] == 'a') n++;
    }
    return n;
}
