/* SPDX-License-Identifier: GPL-2.0-only */
/* Exercise the actual private parser; unused display functions are link-GC'd. */
#include "../../tarjeta.c"

int main(void) {
    char out[32] = "keep";
    ULTRA_CHECK(line_value("model name : Ryzen de prueba\nother: no\n", "model name", out, sizeof(out)) &&
                strcmp(out, "Ryzen de prueba") == 0, "TEST-TARJETA", "control valido rechazado");
    char small[4] = "old";
    ULTRA_CHECK(!line_value("model name : nombre largo\n", "model name", small, sizeof(small)) &&
                strcmp(small, "old") == 0, "TEST-TARJETA", "rechazo cambio el destino");
    ULTRA_CHECK(line_value("model name : abc\n", "model name", small, sizeof(small)) &&
                strcmp(small, "abc") == 0, "TEST-TARJETA", "capacidad exacta rechazada");
    ULTRA_CHECK(!line_value("otra clave: valor\n", "model name", small, sizeof(small)) &&
                strcmp(small, "abc") == 0, "TEST-TARJETA", "clave ausente cambio el destino");
    ULTRA_CHECK(!line_value("model name : valor\n", "model name", small, 0) &&
                strcmp(small, "abc") == 0, "TEST-TARJETA", "capacidad cero cambio el destino");
    puts("TARJETA-OK: 5 controles del parser real; rechazo conserva destino");
    return 0;
}
