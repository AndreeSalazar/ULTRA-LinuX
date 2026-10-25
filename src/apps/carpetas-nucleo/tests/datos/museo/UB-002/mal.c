#include <limits.h>
int f(int a) {
    return a / 0 + INT_MIN / -1;
}
