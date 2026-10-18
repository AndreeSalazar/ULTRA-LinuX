/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: un case sin break cae al siguiente sin que se note */
int f(int x) { int r = 0; switch (x) { case 1: r = 1; case 2: r += 2; break; } return r; }
