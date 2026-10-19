/* SPDX-License-Identifier: GPL-2.0-only */
/* BANDERA: un int negativo convertido a size_t se vuelve un tamano gigante */
#include <stddef.h>
size_t f(int n) { return n; }
