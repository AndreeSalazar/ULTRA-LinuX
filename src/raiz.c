/* SPDX-License-Identifier: GPL-2.0-only */
/* A private mount root, so unprivileged browser user namespaces can work.
 * No setuid installation: only the existing privileged console may invoke it. */
#define _GNU_SOURCE
#include <errno.h>
#include <sched.h>
#include <stdio.h>
#include <string.h>
#include <sys/mount.h>
#include <sys/syscall.h>
#include <unistd.h>
#include "guardian/ultra_guard.h"

static int fail(const char *step) {
    fprintf(stderr, "ROJO [RAIZ]: %s: %s\n", step, strerror(errno));
    return 1;
}

int main(int argc, char **argv) {
    if (geteuid() != 0 || argc < 3 || argv[1][0] != '/' || argv[2][0] != '/') {
        fputs("Uso privilegiado: ultra-raiz /raiz /programa [argumentos]\n", stderr);
        return 2;
    }
    if (unshare(CLONE_NEWNS) < 0) return fail("namespace de montajes");
    if (mount(NULL, "/", NULL, MS_REC | MS_PRIVATE, NULL) < 0) return fail("propagacion privada");
    if (mount(argv[1], argv[1], NULL, MS_BIND | MS_REC, NULL) < 0) return fail("raiz montable");
    if (chdir(argv[1]) < 0) return fail("directorio de raiz");
    /* The kernel operation avoids umount(8)'s path canonicalization after pivot. */
    if (syscall(SYS_pivot_root, ".", ".") < 0) return fail("pivot_root");
    if (umount2(".", MNT_DETACH) < 0) return fail("soltar raiz anterior");
    if (chdir("/") < 0) return fail("raiz actual");
    execv(argv[2], argv + 2);
    return fail("ejecutar preparador");
}
