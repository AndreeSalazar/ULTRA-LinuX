/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: el analizador ve un uso tras free en compilacion, aunque produccion no lo detecte al ejecutar */
#include <stdlib.h>
int f(void) { int *p = malloc(sizeof(int)); if (!p) return 0; *p = 1; free(p); return *p; }
