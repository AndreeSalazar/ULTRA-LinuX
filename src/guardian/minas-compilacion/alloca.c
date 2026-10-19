/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: alloca reserva pila sin limite ni aviso de fallo */
#include <alloca.h>
void g(char *);
void f(unsigned long n) { char *p = alloca(n); g(p); }
