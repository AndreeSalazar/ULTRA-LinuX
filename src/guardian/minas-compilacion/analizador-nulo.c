/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: malloc puede devolver NULL y se usa sin comprobar */
#include <stdlib.h>
int f(void) { int *p = malloc(sizeof(int)); *p = 3; int r = *p; free(p); return r; }
