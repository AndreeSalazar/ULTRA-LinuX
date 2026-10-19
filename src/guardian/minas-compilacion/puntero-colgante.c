/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: guardar en un global la direccion de una local */
int *g;
void f(void) { int x = 1; g = &x; }
