/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: memcpy con regiones solapadas visibles al compilar; -Wrestrict */
#include <string.h>
void f(char *b) { memcpy(b + 4, b, 16); }
