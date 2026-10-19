/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: el analizador ve dos free sobre el mismo puntero */
#include <stdlib.h>
void f(void) { char *p = malloc(8); free(p); free(p); }
