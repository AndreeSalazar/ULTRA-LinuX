/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: una variable interna tapa a otra con el mismo nombre */
int f(int x) { int r = x; { int r = 2; x += r; } return r + x; }
