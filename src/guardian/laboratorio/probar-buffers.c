/* SPDX-License-Identifier: GPL-2.0-only */
#include <limits.h>
#include "../produccion/ultra_buffer.h"
#include "ultra_guard.h"

/* Independent wide-integer oracle for this GCC/x86-64 laboratory. No allocations. */
_Static_assert(SIZE_MAX == UINT64_MAX, "oracle requires 64-bit size_t");
static size_t checked, valid, rejected;

static void layout(size_t offset, size_t stride, size_t rows, size_t bytes, size_t cap) {
    struct ultra_buffer_span out = {17, 19, 23};
    enum ultra_buffer_status want;
    __uint128_t length = (__uint128_t)stride * rows;
    __uint128_t end = (__uint128_t)offset + length;
    if (!rows || !bytes || bytes > stride) want = ULTRA_BUFFER_ARGUMENT;
    else if (length > SIZE_MAX || end > SIZE_MAX) want = ULTRA_BUFFER_OVERFLOW;
    else if (end > cap) want = ULTRA_BUFFER_CAPACITY;
    else want = ULTRA_BUFFER_OK;
    enum ultra_buffer_status got = ultra_buffer_layout(offset, stride, rows, bytes, cap, &out);
    ULTRA_CHECK(got == want, "TEST-BUFFER", "estado distinto del oraculo de 128 bits");
    if (got == ULTRA_BUFFER_OK) {
        ULTRA_CHECK(out.begin == offset && out.length == (size_t)length && out.end == (size_t)end,
                    "TEST-BUFFER", "rango aceptado incorrecto");
        valid++;
    } else {
        ULTRA_CHECK(out.begin == 17 && out.length == 19 && out.end == 23,
                    "TEST-BUFFER", "un rechazo modifico el resultado");
        rejected++;
    }
    checked++;
}

int main(void) {
    const size_t edges[] = {0, 1, 4, 16, SIZE_MAX / 2, SIZE_MAX - 1, SIZE_MAX};
    for (size_t a = 0; a < ULTRA_LEN(edges); a++)
        for (size_t b = 0; b < ULTRA_LEN(edges); b++)
            for (size_t c = 0; c < ULTRA_LEN(edges); c++)
                for (size_t d = 0; d < ULTRA_LEN(edges); d++)
                    for (size_t e = 0; e < ULTRA_LEN(edges); e++)
                        layout(edges[a], edges[b], edges[c], edges[d], edges[e]);
    struct ultra_buffer_span out = {17, 19, 23};
    ULTRA_CHECK(ultra_buffer32_layout(1920, 1080, 7680, 0, 8294400, &out) == ULTRA_BUFFER_OK &&
                out.length == 8294400 && out.end == 8294400,
                "TEST-BUFFER", "control 1080p rechazado");
    ULTRA_CHECK(ultra_buffer32_layout(3, 2, 16, 4, 36, &out) == ULTRA_BUFFER_OK && out.end == 36,
                "TEST-BUFFER", "padding y offset validos rechazados");
    const int32_t wrong[][4] = {{-1, 1, 4, 0}, {1, -1, 4, 0}, {1, 1, -1, 0},
                              {1, 1, 4, -1}, {0, 1, 4, 0}, {2, 1, 4, 0}, {INT32_MAX, 1, 4, 0}};
    for (size_t i = 0; i < ULTRA_LEN(wrong); i++)
        ULTRA_CHECK(ultra_buffer32_layout(wrong[i][0], wrong[i][1], wrong[i][2], wrong[i][3], SIZE_MAX, &out)
                    == ULTRA_BUFFER_ARGUMENT, "TEST-BUFFER", "dimension invalida aceptada");
    ULTRA_CHECK(ultra_buffer32_layout(1920, 1080, 7680, 0, 8294399, &out) == ULTRA_BUFFER_CAPACITY,
                "TEST-BUFFER", "pool un byte menor aceptado");
    ULTRA_CHECK(ultra_buffer_layout(0, 4, 1, 4, 4, NULL) == ULTRA_BUFFER_ARGUMENT,
                "TEST-BUFFER", "salida nula aceptada");
    printf("BUFFERS-OK: matriz=%zu validos=%zu rechazos=%zu; controles=%zu; sin reservar ni mapear buffers\n",
           checked, valid, rejected, ULTRA_LEN(wrong) + 4);
    return 0;
}
