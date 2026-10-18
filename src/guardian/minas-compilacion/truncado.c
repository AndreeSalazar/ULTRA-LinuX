/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: un long en int pierde bits sin avisar */
int f(long v) { return v; }
