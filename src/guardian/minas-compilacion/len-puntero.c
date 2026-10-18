/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: ULTRA_LEN sobre un puntero daria 8/1 en vez del tamano real */
#include "../ultra_guard.h"
size_t f(char *p) { return ULTRA_LEN(p); }
