/* SPDX-License-Identifier: GPL-2.0-only */
/* Include the real production implementation; no duplicate model parser. */
#define ULTRA_RED_TEST
#include "../../red.c"

int main(void) {
    ultra_guard_install("probar-red");
    static const struct { const char *input; bool valid; uint64_t value; } cases[] = {
        {"", false, 0}, {"\n", false, 0}, {"0", true, 0}, {"123\n", true, 123},
        {"18446744073709551615\n", true, UINT64_MAX},
        {"18446744073709551616\n", false, 0}, {"-1\n", false, 0}, {"+1", false, 0},
        {" 1", false, 0}, {"1\n2", false, 0}, {"1x", false, 0}, {"1\r\n", false, 0},
        {"9999999999999999999999999999999", false, 0}
    };
    for (size_t i = 0; i < ULTRA_LEN(cases); i++) {
        uint64_t n = 777;
        bool ok = counter(cases[i].input, strlen(cases[i].input), &n);
        ULTRA_CHECK(ok == cases[i].valid, "RED-PARSER", "aceptacion incorrecta");
        ULTRA_CHECK(ok ? n == cases[i].value : n == 777, "RED-PARSER", "valor incorrecto");
    }
    /* Embedded NUL and high bytes may never be interpreted as decimal digits. */
    const char embedded[] = {'1', '\0', '2'}, high[] = {(char)0xff};
    uint64_t n = 0;
    ULTRA_CHECK(!counter(embedded, sizeof(embedded), &n) && !counter(high, sizeof(high), &n),
                "RED-PARSER", "entrada binaria aceptada");
    const char exact[] = {'4', '2'}; /* No terminating NUL: length is authoritative. */
    ULTRA_CHECK(counter(exact, sizeof(exact), &n) && n == 42, "RED-PARSER", "limite exacto");
    ULTRA_CHECK(valid_iface("enp37s0") && valid_iface("red-0.1") && !valid_iface("../x") &&
                !valid_iface("..") && !valid_iface("") && !valid_iface("1234567890123456"),
                "RED-NOMBRE", "nombre no validado");
    /* Exercise missing sysfs data: must report NA, without fabricating zero. */
    attribute("ultra-no-existe", "statistics/rx_bytes", "ausente");
    puts("\nVERDE [RED-C]: 18 grupos de controles, limites y desbordamiento");
    return 0;
}
