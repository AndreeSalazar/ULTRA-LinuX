/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: atoi es UB si el numero no cabe en int y no informa errores; envenenado, usar strtol */
#include "../ultra_guard.h"
int f(const char *s) { return atoi(s); }
