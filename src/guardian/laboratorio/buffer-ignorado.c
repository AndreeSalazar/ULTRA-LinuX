/* SPDX-License-Identifier: GPL-2.0-only */
/* Deliberately invalid use: production review must reject ignoring the status. */
#include "../produccion/ultra_buffer.h"
int main(void) {
    struct ultra_buffer_span out;
    ultra_buffer32_layout(1, 1, 4, 0, 4, &out);
    return 0;
}
