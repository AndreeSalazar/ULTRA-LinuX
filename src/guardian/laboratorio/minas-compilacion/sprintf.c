/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: sprintf escribe sin limite; envenenado, usar snprintf */
#include "../ultra_guard.h"
void f(char *d, int x) { sprintf(d, "%d", x); }
