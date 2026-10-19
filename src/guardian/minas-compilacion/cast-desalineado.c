/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: convertir char* en int* puede crear un puntero desalineado */
int f(char *p) { int *q = (int *)(p + 1); return *q; }
