/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: comparar int con size_t convierte -1 en un numero enorme */
#include <stddef.h>
int f(int i, size_t n) { return i < n; }
