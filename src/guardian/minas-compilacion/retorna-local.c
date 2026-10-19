/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: devolver la direccion de una variable local que muere al salir */
int *f(void) { int x = 1; return &x; }
