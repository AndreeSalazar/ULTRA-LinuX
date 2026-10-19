/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: modificar i dos veces sin punto de secuencia es UB */
int f(int i) { i = i++ + 1; return i; }
