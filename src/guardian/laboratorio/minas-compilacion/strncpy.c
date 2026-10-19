/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: strncpy no termina la cadena si no cabe; envenenado */
#include "../ultra_guard.h"
void f(char *d, const char *s) { strncpy(d, s, 8); }
