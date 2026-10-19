/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: strcpy no conoce el tamano del destino; envenenado por ultra_guard.h */
#include "../ultra_guard.h"
void f(char *d, const char *s) { strcpy(d, s); }
