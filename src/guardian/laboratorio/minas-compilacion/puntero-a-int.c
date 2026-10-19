/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: un puntero de 64 bits metido en int pierde la mitad */
int f(void *p) { return (int)p; }
