/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: el analizador encuentra un camino donde malloc nunca se libera */
#include <stdlib.h>
int f(int c) { char *p = malloc(16); if (!p) return -1; if (c) return 1; free(p); return 0; }
