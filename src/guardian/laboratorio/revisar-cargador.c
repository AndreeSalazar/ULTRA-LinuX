/* SPDX-License-Identifier: GPL-2.0-only */
/* Review snapshot: intercept ALL syscall() calls; never load a real module. */
#define _GNU_SOURCE
#define syscall ultra_audit_syscall
#include "../../hardware.c"
#undef syscall

static size_t attempted;
long ultra_audit_syscall(long nr, ...) {
    (void)nr;
    attempted++;
    errno = EPERM;
    return -1;
}
void report(const char *level, const char *code, const char *detail) {
    fprintf(stderr, "%s [%s]: %s\n", level, code, detail);
}
const char *windows_dir(void) { return NULL; }

int main(void) {
    size_t findings = 0;
    deps = NULL; ndeps = 0;
    ntried = nloaded = nfailed = nskipped = 0;
    bool optional = load_module("ultra_audit_absent_optional", 0);
    ULTRA_CHECK(optional && nskipped == 1 && nfailed == 0,
                "AUDIT-CONTROL", "cambio el control de modulo opcional de otro kernel");
    puts("CONTROL: modulo opcional ajeno se omite como contempla W3a");
    ntried = nloaded = nfailed = nskipped = 0;
    bool required = load_module("ultra_audit_absent_dependency", 1);
    if (required && nfailed == 0) {
        puts("HALLAZGO: dependencia requerida ausente aceptada; retorno=true, fallos=0");
        findings++;
    }
    ULTRA_CHECK(ULTRA_COPY(moddir, "/minas/fixture"), "AUDIT-CONTROL", "ruta demasiado larga");
    nwanted = 2;
    ULTRA_CHECK(ULTRA_COPY(wanted[0], "ultra_audit_absent_one") &&
                ULTRA_COPY(wanted[1], "ultra_audit_absent_two"), "AUDIT-CONTROL", "nombre demasiado largo");
    load_wanted();
    if (nskipped == 0 && nfailed == 0) {
        puts("HALLAZGO: dos omisiones en hijos no llegan al padre; omisiones=0, fallos=0");
        findings++;
    }
    ULTRA_CHECK(attempted == 0, "AUDIT-CONTROL", "la prueba alcanzo una syscall interceptada");
    printf("REVISION-CARGADOR: hallazgos=%zu; syscalls interceptadas=%zu; sin cargar modulos\n", findings, attempted);
    return 0; /* An audit can finish successfully while reporting defects. */
}
