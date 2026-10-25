#include <string.h>
int cuenta_a(const char *s) {
    int n = 0;
    for (size_t i = 0; i < strlen(s); i++) {
        if (s[i] == 'a') n++;
    }
    return n;
}
