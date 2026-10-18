/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: un arreglo de tamano variable en la pila puede saltar la pagina de guarda */
void g(char *);
void f(int n) { char buf[n]; g(buf); }
