/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: memset con sizeof(puntero) borra 8 bytes en vez del objeto */
#include <string.h>
struct s { char data[64]; };
void f(struct s *p) { memset(p, 0, sizeof(p)); }
