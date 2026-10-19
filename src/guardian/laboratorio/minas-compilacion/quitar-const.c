/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: quitar const con un cast permite escribir en memoria de solo lectura */
void f(const char *s) { char *w = (char *)s; w[0] = 'x'; }
