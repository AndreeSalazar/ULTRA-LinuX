/* SPDX-License-Identifier: GPL-2.0-only */
#include "../produccion/ultra_string.h"
int main(void) {
    char dst[4];
    ultra_string_copy(dst, sizeof(dst), "OK", 3);
    return 0;
}
