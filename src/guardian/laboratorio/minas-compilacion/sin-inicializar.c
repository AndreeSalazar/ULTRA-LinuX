/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: leer una variable sin valor es UB */
int f(int c) { int x; if (c) x = 1; return x; }
