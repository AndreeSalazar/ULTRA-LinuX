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

int main(int argc, char **argv) {
    bool require_fixed = argc == 2 && !strcmp(argv[1], "--exigir-correccion");
    if (argc > 1 && !require_fixed) return 2;
    size_t findings = 0;
    deps = NULL; ndeps = 0;
    ntried = nloaded = nfailed = nskipped = 0;
    bool optional = load_module("ultra_audit_absent_optional", 0);
    ULTRA_CHECK(optional && nskipped == 1 && nfailed == 0,
                "AUDIT-CONTROL", "cambio el control de modulo opcional de otro kernel");
    puts("CONTROL: modulo opcional ajeno se omite como contempla W3a");
    ntried = nloaded = nfailed = nskipped = 0;
    bool required = load_module("ultra_audit_absent_dependency", 1);
    if (required || nfailed == 0 || nskipped != 0) {
        puts("HALLAZGO: dependencia requerida ausente aceptada; retorno=true, fallos=0");
        findings++;
    } else puts("CONTROL: dependencia requerida ausente rechazada y contada");
    ULTRA_CHECK(ULTRA_COPY(moddir, "/minas/fixture"), "AUDIT-CONTROL", "ruta demasiado larga");
    nwanted = 2;
    ULTRA_CHECK(ULTRA_COPY(wanted[0], "ultra_audit_absent_one") &&
                ULTRA_COPY(wanted[1], "ultra_audit_absent_two"), "AUDIT-CONTROL", "nombre demasiado largo");
    load_wanted();
    if (nskipped != 2 || nfailed != 0 || nloaded != 0) {
        puts("HALLAZGO: dos omisiones en hijos no llegan al padre; omisiones=0, fallos=0");
        findings++;
    } else puts("CONTROL: dos omisiones de hijos llegan al padre sin truncamiento");
    for (int variant = 0; variant < 3; variant++) {
        int channel[2];
        const struct load_result sent = {700, 3, 900};
        struct load_result received;
        size_t bytes = variant == 0 ? 0 : variant == 1 ? sizeof(sent) - 1 : sizeof(sent);
        ULTRA_CHECK(pipe2(channel, O_CLOEXEC) == 0 &&
                    write_all(channel[1], (const char *)&sent, bytes), "AUDIT-CONTROL", "pipe de prueba fallo");
        close(channel[1]);
        bool complete = read_load_result(channel[0], &received);
        close(channel[0]);
        ULTRA_CHECK(complete == (variant == 2), "AUDIT-CONTROL", "resultado parcial aceptado");
        if (complete) ULTRA_CHECK(received.loaded == 700 && received.failed == 3 && received.skipped == 900,
                                  "AUDIT-CONTROL", "contadores truncados");
    }
    puts("CONTROL: mensajes vacios/parciales rechazados; contadores mayores de 255 conservados");
    ULTRA_CHECK(ULTRA_COPY(moddir, "/minas/no-existe"), "AUDIT-CONTROL", "ruta demasiado larga");
    load_wanted();
    ULTRA_CHECK(nfailed == 1, "AUDIT-CONTROL", "error al leer modules.dep no contado");
    puts("CONTROL: metadata de dependencias ausente produce fallo contado");
    ULTRA_CHECK(attempted == 0, "AUDIT-CONTROL", "la prueba alcanzo una syscall interceptada");
    printf("REVISION-CARGADOR: hallazgos=%zu; syscalls interceptadas=%zu; sin cargar modulos\n", findings, attempted);
    return require_fixed && findings ? 1 : 0;
}
