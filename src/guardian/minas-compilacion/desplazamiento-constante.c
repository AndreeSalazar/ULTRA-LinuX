/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: desplazar 40 bits un int de 32, visible al compilar */
int f(void) { return 1 << 40; }
