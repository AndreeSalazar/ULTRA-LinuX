/* SPDX-License-Identifier: GPL-2.0-only */
#define _GNU_SOURCE
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/resource.h>
#include <unistd.h>

static int reject(const char *why) { fprintf(stderr, "AISLAMIENTO-ERROR: %s\n", why); return 1; }
int main(void) {
    if (access("/home/builder", F_OK) == 0 || access("/vm", F_OK) == 0)
        return reject("directorio del taller visible");
    int fd = open("/minas/escritura-prohibida", O_CREAT | O_WRONLY, 0600);
    if (fd >= 0) { close(fd); return reject("artefactos escribibles"); }
    if (errno != EROFS) return reject("rechazo de escritura distinto de EROFS");
    DIR *d = opendir("/proc");
    if (!d) return reject("sin proc privado");
    size_t pids = 0;
    struct dirent *e;
    while ((e = readdir(d))) if (e->d_name[0] >= '0' && e->d_name[0] <= '9') pids++;
    closedir(d);
    /* Bubblewrap keeps a PID 1 reaper alongside this process. */
    if (pids != 2 || getpid() != 2 || getppid() != 1)
        return reject("procesos externos visibles o PID namespace inesperado");
    FILE *f = fopen("/proc/net/route", "r");
    if (!f) return reject("sin tabla de rutas privada");
    char line[256];
    if (!fgets(line, sizeof(line), f)) { fclose(f); return reject("sin cabecera de rutas"); }
    if (fgets(line, sizeof(line), f)) { fclose(f); return reject("ruta de red disponible"); }
    fclose(f);
    if (getenv("SSH_AUTH_SOCK") || getenv("ULTRA_SECRET_TEST"))
        return reject("entorno heredado");
    f = fopen("/proc/self/status", "r");
    if (!f) return reject("sin estado de privilegios");
    bool caps = false, nnp = false;
    while (fgets(line, sizeof(line), f)) {
        if (!strcmp(line, "CapEff:\t0000000000000000\n")) caps = true;
        if (!strcmp(line, "NoNewPrivs:\t1\n")) nnp = true;
    }
    fclose(f);
    if (!caps || !nnp) return reject("capacidades activas o no_new_privs ausente");
    struct rlimit limit;
    if (getrlimit(RLIMIT_CORE, &limit) != 0 || limit.rlim_max != 0)
        return reject("core dumps permitidos");
    if (getrlimit(RLIMIT_CPU, &limit) != 0 || limit.rlim_max != 10)
        return reject("limite CPU ausente");
    puts("AISLAMIENTO-OK: home ausente, artefactos RO, PID privado, sin rutas, entorno limpio, CapEff=0, NNP=1, limites CPU/core");
    return 0;
}
