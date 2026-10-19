/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: memcpy de 8 bytes en un destino de 4, visible al compilar */
#include <string.h>
void g(char *);
void f(void) { char b[4]; memcpy(b, "12345678", 8); g(b); }
