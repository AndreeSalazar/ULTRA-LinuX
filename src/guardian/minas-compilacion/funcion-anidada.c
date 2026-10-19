/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: una funcion anidada con trampolin exige pila ejecutable */
int run(int (*cb)(int));
int f(int k) { int add(int x) { return x + k; } return run(add); }
