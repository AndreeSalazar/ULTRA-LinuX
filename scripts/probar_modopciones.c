/* SPDX-License-Identifier: GPL-2.0-only */
/* Host test for src/modopciones.h: cc -std=c11 -Wall -Wextra -Werror -o t scripts/probar_modopciones.c */
#include <stdio.h>
#include <string.h>
#include "../src/modopciones.h"

static int fails;
#define EXPECT(cond, msg) do { if (!(cond)) { printf("FALLO: %s\n", msg); fails++; } } while (0)

static void conf_and_cmdline(void) {
    const char *conf =
        "# comentario\n"
        "blacklist nouveau\n"
        "install nvidia /bin/sh -c 'rm -rf /'\n"
        "options nvidia NVreg_PreserveVideoMemoryAllocations=1 NVreg_TemporaryFilePath=/juegos/.ultra-nvidia\n"
        "options nvidia_drm modeset=1   # fin\n"
        "\toptions  nvidia-drm  fbdev=1\n"
        "options nvidia bad=\"comillas\" ok=2\n"
        "options nvidia_uvm uvm_x=1";
    char out[256] = "";
    EXPECT(ultra_mod_from_conf(conf, "nvidia", out, sizeof(out)), "nvidia cabe");
    EXPECT(!strcmp(out, "NVreg_PreserveVideoMemoryAllocations=1 NVreg_TemporaryFilePath=/juegos/.ultra-nvidia ok=2"), out);
    char drm[64] = "";
    ultra_mod_from_conf(conf, "nvidia_drm", drm, sizeof(drm));
    EXPECT(!strcmp(drm, "modeset=1 fbdev=1"), drm);
    const char *cmd = "console=tty0 rdinit=/init nvidia-drm.modeset=1 nvidia-drm.fbdev=1 "
                      "GLIBC_TUNABLES=glibc.cpu.hwcaps=SHSTK libahci.ignore_sss=1 nvidia.x=$(rm) .y=1\n";
    char c[64] = "";
    ultra_mod_from_cmdline(cmd, "nvidia_drm", c, sizeof(c));
    EXPECT(!strcmp(c, "modeset=1 fbdev=1"), c);
    char g[32] = "";
    ultra_mod_from_cmdline(cmd, "GLIBC_TUNABLES=glibc", g, sizeof(g));
    EXPECT(!strcmp(g, ""), "un '=' antes del punto no es modulo");
    char n[32] = "";
    ultra_mod_from_cmdline(cmd, "nvidia", n, sizeof(n));
    EXPECT(!strcmp(n, ""), "caracteres de shell ignorados");
    char a[32] = "";
    ultra_mod_from_cmdline(cmd, "libahci", a, sizeof(a));
    EXPECT(!strcmp(a, "ignore_sss=1"), a);
}

static void overflow_and_empty(void) {
    char small[8] = "";
    EXPECT(!ultra_mod_from_conf("options m abcdefghij=1\n", "m", small, sizeof(small)), "desborde detectado");
    EXPECT(!strcmp(small, ""), "sin escritura parcial");
    char none[16] = "";
    EXPECT(ultra_mod_from_conf("", "m", none, sizeof(none)) && !none[0], "vacio");
    EXPECT(ultra_mod_from_cmdline("", "m", none, sizeof(none)) && !none[0], "cmdline vacia");
    EXPECT(!ultra_mod_same("snd", 3, "snd_hda"), "prefijo no basta");
}

int main(void) {
    conf_and_cmdline();
    overflow_and_empty();
    if (!fails) puts("modopciones: OK");
    return fails != 0;
}
