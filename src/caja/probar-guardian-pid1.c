/* SPDX-License-Identifier: GPL-2.0-only */
#define _GNU_SOURCE
#include <unistd.h>
#include "../guardian/ultra_guard.h"
int main(int argc, char **argv) {
    if (argc != 2 || getpid() != 1) return 2;
    ultra_guard_install("control-pid1");
    if (!strcmp(argv[1], "abort")) ultra_fail("TEST-PID1", __FILE__, __LINE__, "rechazo esperado sin bucle");
    if (!strcmp(argv[1], "trap")) __builtin_trap();
    return 3;
}
