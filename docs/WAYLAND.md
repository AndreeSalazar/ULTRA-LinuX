# Camino a Wayland

Wayland en sí es un protocolo pequeño. Lo grande es lo que tiene debajo: un espacio de usuario dinámico con glibc, dispositivos, sesión, bibliotecas gráficas y un compositor. ULTRA lo construye por etapas, cada una probada en QEMU antes de llegar al PC, con el método del [PLAN](PLAN.md#enfoque-compatible-no-copia-18-10-2026): abrir, medir, vigilar con guardianes y documentar.

**Medida de partida:** un Wayland mínimo (`cage` + `foot` + `seatd`) son **122 paquetes de Arch y 128 MiB de descarga**. Mesa con LLVM, para dibujar por software, es lo más pesado.

| Etapa | Qué | Estado |
|---|---|---|
| **W1** | Sistema base de Arch en EROFS, montado por el init como raíz en `/arch` | **Hecha** |
| W1b | La raíz por defecto pasa a ser Arch | Pendiente: antes, `caja` necesita reglas medidas para programas dinámicos |
| W2 | `udevd` (de Arch, sin systemd) + `seatd`, y `cage` + `foot` sobre `simpledrm` | Pendiente: la primera ventana |
| W3 | RTX 3060: `nvidia-open`, Vulkan, gamescope | Pendiente |

## W1: el sistema base de Arch dentro de ULTRA

**Construcción** (`scripts/build-baseline.sh`): el `pacman` de Arch instala en una raíz aparte `filesystem glibc bash coreutils grep sed less`. Son 47 paquetes con sus dependencias (`evidencias/sistema-arch-paquetes.txt`), sin cambiar nada: compatible con Arch, no una copia. Quedan fuera la documentación, las páginas de manual, las traducciones y las cabeceras de desarrollo, que un arranque no necesita. Se añaden los módulos del kernel y se comprime todo en EROFS con lz4hc: **70 MiB**, uno por kernel (`sistema-<versión>.erofs`).

**Guardián de la construcción:** `libc.so.6`, `bash` y `ls` deben traer la marca CET (`IBT, SHSTK`); si falta, la construcción se para. Los binarios de Arch la traen, así que **la pila sombra de ULTRA también protege a los programas de Arch**.

**Arranque** (`src/sistema.c`): el init monta en `/arch` una raíz completa con overlayfs:
- debajo, la imagen EROFS, de solo lectura, que nunca cambia;
- encima, una capa en memoria (tmpfs) que recibe toda escritura, así que cada arranque empieza limpio;
- dentro, `/dev`, `/proc`, `/sys`, `/tmp` y `/datos` enlazados, para ver la máquina real y los datos que sobreviven.

Al apagar suelta `/arch` antes de desmontar los discos, para que `A:` quede limpio.

**Uso:** `ultra-entrar` entra en el `bash` de Arch. `ultra-entrar programa args` ejecuta un solo programa de Arch y vuelve a la shell de ULTRA.

**Por qué todavía no es la raíz por defecto:** `caja` ejecuta todo desde `/bin` con reglas de Landlock y seccomp medidas para BusyBox. Si `/bin` pasara a ser Arch, sus programas dinámicos necesitarían leer `/usr/lib` y llamadas al sistema que la lista no declara, y `caja` los bloquearía. W1b llega cuando `caja` tenga sus reglas medidas para el ecosistema de Arch.

**Probado en QEMU** (`evidencias/w1-sistema-arch-qemu.log`): `generico --bmo` 57/57, `pc --bmo` 58/58, `--disco` 51/51, y `--auto` y `--auto-falla` en VERDE. Entre las comprobaciones:
- `VERDE [SISTEMA]: base Arch en /arch (47 paquetes, glibc 2.44; …)`;
- el `bash` de Arch (`ARCH-5.3…`) y `ls` de GNU coreutils;
- `grep` de Arch leyendo su propio `/proc/self/status` ve `shstk`: CET activo en un programa dinámico de Arch;
- `/datos` visible desde dentro;
- una escritura en `/etc` de Arch va a la capa en memoria, y la imagen `/sistema` no cambia.

**Tamaño:** cada imagen de sistema pasa de 2 MiB a 70 MiB. La partición `ULTRA-SISTEMA-A` del USB pasa a 160 MiB, y `BMO` en el disco de prueba a 320 MiB. En `A:` sobran 31 GB.
