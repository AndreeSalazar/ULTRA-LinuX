/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: sizeof de un arreglo parametro mide el puntero, no el arreglo */
#include <stddef.h>
size_t f(char buf[64]) { return sizeof(buf); }
