/* SPDX-License-Identifier: GPL-2.0-only */
#define _GNU_SOURCE
#include <sys/wait.h>
#include "identidad-politica.h"
#include "../guardian/ultra_guard.h"

int main(void) {
    ULTRA_CHECK(geteuid() == 0, "TEST-IDENTIDAD", "esta prueba exige root solo en el taller");
    ULTRA_CHECK(!caja_identity_valid(65534, 65534), "TEST-IDENTIDAD", "root fue aceptado");
    pid_t child = fork();
    ULTRA_CHECK(child >= 0, "TEST-IDENTIDAD", "fork fallo");
    if (child == 0) {
        /* Deliberately contaminate only this disposable child, never the host. */
        struct __user_cap_header_struct h = {_LINUX_CAPABILITY_VERSION_3, 0};
        struct __user_cap_data_struct d[2] = {{0}};
        ULTRA_CHECK(syscall(SYS_capget, &h, d) == 0, "TEST-IDENTIDAD", "capget fallo");
        d[0].inheritable |= 1u << CAP_CHOWN;
        ULTRA_CHECK(syscall(SYS_capset, &h, d) == 0 &&
                    prctl(PR_CAP_AMBIENT, PR_CAP_AMBIENT_RAISE, CAP_CHOWN, 0, 0) == 0 &&
                    prctl(PR_SET_KEEPCAPS, 1, 0, 0, 0) == 0,
                    "TEST-IDENTIDAD", "no se preparo la contaminacion de capacidades");
        ULTRA_CHECK(caja_drop_identity(65534, 65534), "TEST-IDENTIDAD", "retirada incompleta");
        errno = 0;
        ULTRA_CHECK(setuid(0) == -1 && errno == EPERM, "TEST-IDENTIDAD", "UID 0 recuperable");
        errno = 0;
        ULTRA_CHECK(setgid(0) == -1 && errno == EPERM, "TEST-IDENTIDAD", "GID 0 recuperable");
        ULTRA_CHECK(caja_identity_valid(65534, 65534), "TEST-IDENTIDAD", "estado cambio tras rechazo");
        _exit(0);
    }
    int st = 0;
    ULTRA_CHECK(waitpid(child, &st, 0) == child && WIFEXITED(st) && WEXITSTATUS(st) == 0,
                "TEST-IDENTIDAD", "fallo el control real de credenciales");
    puts("IDENTIDAD-OK: root rechazado; heredables/ambientales/keepcaps retirados; cinco conjuntos vacios; UID/GID 0 no recuperables por setuid/setgid");
    return 0;
}
