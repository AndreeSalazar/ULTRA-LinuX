/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: llamar una funcion con otra firma corrompe argumentos y retorno */
typedef void (*fn)(int);
long g(long a, long b);
fn f(void) { return (fn)g; }
