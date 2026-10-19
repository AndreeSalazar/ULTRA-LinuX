# ULTRA LinuX

Sistema propio construido sobre el kernel oficial de Linux, compilado con GCC. Toma ideas de Arch Linux (control y pacman) y de SteamOS (sesión de juego e inmutabilidad), pero no es ninguna de las dos ni usa sus marcas. Hardware objetivo: Ryzen 5 5600X, MSI A320M-A PRO MAX, NVIDIA RTX 3060 12 GB.

## Blindaje de C: un campo de minas con banderas

C no se vuelve seguro solo por usar opciones del compilador. Lo que sí se consigue es que **cada mina conocida tenga su bandera**: el fallo se detiene en el acto, dice qué guardián despertó y por qué, y queda encerrado. Las opciones viven en un solo sitio, `scripts/guardianes.sh`, compartido por la construcción y por las pruebas.

### Guardián 1: frontera de memoria

| Capa | Cómo | Bandera cuando despierta |
|---|---|---|
| Funciones envenenadas | `#pragma GCC poison strcpy strcat sprintf vsprintf gets strtok` en `ultra_guard.h` | `attempt to use poisoned 'strcpy'`: no compila |
| Copias con tamaño | `ULTRA_COPY`, `ULTRA_APPEND`, `ULTRA_AT`, `ULTRA_LEN` (rechaza punteros) | `ROJO [FRONTERA-INDICE] archivo:línea` |
| FORTIFY 3 | `-fhardened` (incluye `_FORTIFY_SOURCE=3`) | `*** buffer overflow detected ***`, también en el heap |
| Protector de pila | `-fhardened` (`-fstack-protector-strong`, `-fstack-clash-protection`) | `*** stack smashing detected ***` |
| Pila sombra CET | `-fcf-protection=full` + `GLIBC_TUNABLES` + `X86_USER_SHADOW_STACK` | `SEGV_CPERR: la pila sombra CET bloqueo un retorno falso` |
| Kernel | `HARDENED_USERCOPY`, `INIT_ON_ALLOC_DEFAULT_ON`, `INIT_STACK_ALL_ZERO`, KASLR, `SLAB_FREELIST_HARDENED/RANDOM`, `LIST_HARDENED`, `FORTIFY_SOURCE` | La construcción se niega (ROJO) si falta alguna |

### Guardián 2: UB

- **Revisión** (`ultra_revisar`): avisos como errores en `-O2` y analizador en `-O0`. Hallazgo medido: el `-fanalyzer` de GCC 16 **no ve** fugas, NULL ni lecturas sin inicializar en `-O1`/`-O2`, porque el optimizador las pliega antes. Por eso es un pase aparte.
- **Producción**: trampas UBSan sin biblioteca (`-fsanitize=undefined,float-cast-overflow -fsanitize-trap=all`). `ultra_guard.c` instala un manejador en pila alternativa que traduce la señal a una bandera con programa, código y `pc=inicio+0x…` (se lleva a línea con `addr2line`).
- **Diagnóstico**: ASan + UBSan completos, que nombran el motivo exacto (`signed integer overflow: 2147483647 + 1…`). Más lento; nunca se distribuye.
- **PID 1**: el init traduce la muerte de cualquier proceso, también de los huérfanos, a `ROJO [GUARDIAN]` con su motivo.

### Guardián 3: aislamiento (`caja`)

El programa **declara**, `caja` **concede** y el kernel **comprueba**, como en BMO-X. La declaración es código C compilado, no un archivo que haya que interpretar: no existe un parser que atacar. Capas, aplicadas antes de `execve`:

1. Namespaces de montaje, PID, UTS, IPC y red: dentro, el programa es PID 1 y la máquina se llama `caja`.
2. Límites de memoria, procesos y archivos; sin volcados de memoria.
3. Usuario 65534, conjunto límite de capacidades vacío y `no_new_privs`.
4. Landlock: solo las rutas declaradas.
5. seccomp con lista blanca **medida con strace**. Una llamada no declarada se detiene en un evento ptrace, así que la bandera la **nombra** (`no declarada: socket (41)`) y la caja entera se destruye.

Si el kernel no ofrece Landlock o seccomp, `caja` se niega a ejecutar: nunca hay una caja más débil en silencio.

### Resultados

El campo de minas crece por rondas; el detalle actualizado, los hallazgos y cómo añadir minas están en [CAMPO-MINAS.md](CAMPO-MINAS.md). Ronda 2: 36/36 de compilación y 34 VERDE + 1 NEUTRA + 4 AMARILLO + 0 ROJO de ejecución. Lo que sigue es la ronda 1.

- **Campo de minas** (`scripts/campo-minas.sh`, en el taller): **31 VERDE, 1 AMARILLO, 0 ROJO** (`evidencias/fase1.5-campo-minas.txt`). Son 15 minas de compilación que no compilan y 17 minas de ejecución detenidas en producción.
- **El AMARILLO honesto**: un uso tras `free` en producción no se detecta al ejecutar. Lo atrapan el analizador al compilar (cuando el camino es visible) y ASan en diagnóstico.
- **Arranque blindado** (`evidencias/fase1.5-arranque-blindado.log`): en el kernel ULTRA, CET activo desde PID 1 (`x86_Thread_features: shstk`) hasta los programas, retorno secuestrado bloqueado, trampa de UB y FORTIFY. `probar-caja` da 11/11: fugas de escritura y lectura rechazadas por Landlock; `socket`, `mount` y `kill` nombrados y la caja destruida.
- **Errores reales que pararon los guardianes en esta fase**: `SA_RESETHAND` (bit 31) cambiando de signo en un `int`; un `sethostname` sin comprobar (visto solo con FORTIFY); y una falsa fuga de descriptor que se resolvió haciendo explícita la intención, sin silenciar el guardián. La caja, por su parte, bloqueó `getgroups` de `id` porque no estaba medido; se declaró tras comprobarlo.
- **Ejecutables**: `init` y `caja` son `static-pie` (estáticos, con ASLR, BIND_NOW y CET). Kernel de 3,2 MB.

### Límites que no se esconden

- La pila sombra protege los retornos, no los saltos indirectos: IBT queda marcado pero no se activa en espacio de usuario.
- En WHPX, glibc no detecta SHSTK y se fuerza con `glibc.cpu.hwcaps=SHSTK`. En el metal habrá que comprobarlo de nuevo.
- `caja` todavía no usa cgroups v2 (los límites son rlimits) y su lista blanca cubre BusyBox; cada programa nuevo necesita su propia declaración medida.

## Fase 1: shell propia (VERDE)

- `src/init.c` es el PID 1. Monta `/dev`, `/proc`, `/sys`, `/tmp` y `/run`, informa cada paso como VERDE/AMARILLO/ROJO y supervisa una shell BusyBox: la relanza si muere y frena si muere en bucle.
- Las señales se atienden de forma síncrona con `sigwaitinfo`. `poweroff`, `reboot` y `halt` de BusyBox apagan mediante el init. Ctrl-Alt-Del llega como señal en lugar de reiniciar de golpe.
- BusyBox 1.36.1 viene del repositorio oficial de Arch. La compilación lo rechaza si no es estático.
- El kernel añade ACPI y PCI. Sin ACPI, `poweroff` solo detenía la CPU y QEMU no terminaba.
- `src/rootfs/etc/` aporta `os-release` y `profile`.
- `scripts/probar_arranque.py` arranca el kernel sin pantalla, escribe comandos por la consola serie y comprueba las respuestas. Resultado: 5/5 comandos VERDE y apagado VERDE (`evidencias/fase1-prueba-shell.log`). Kernel `bzImage-tiny64` de 2,3 MB, SHA-256 `d284a8cf…7462`.

Para verlo en una ventana:

```powershell
.\construir-kernel.ps1 -Accion Ver
```

## Fase 2: kernel genérico x86-64 (primer paso VERDE)

Hay tres perfiles de kernel, que se eligen con `-Perfil`:

| Perfil | Base | Para qué |
|---|---|---|
| `produccion` | `tinyconfig` + guardianes + KFENCE | Laboratorio mínimo, rápido de compilar |
| `vigia` | `produccion` + KASAN, UBSAN, KFENCE, lockdep y detectores de bloqueos | El kernel informa sus propios fallos; el registro debe quedar limpio |
| `generico` | `x86_64_defconfig` + guardianes + KFENCE + módulos | Arrancar en cualquier PC x86-64 moderno |
| `pc` | `generico` + `X86_NATIVE_CPU` (Zen 3) + init, caja y minas con `-O3 -march=znver3 -flto` | Solo el Ryzen 5 5600X de este proyecto; `uname -r` termina en `-pc-g…` |

**Perfil `pc`: optimización máxima sin quitar guardianes.** El kernel se compila con `-march=native` (el taller corre con `-cpu host`, así que es el 5600X real), y el espacio de usuario con `-O3 -march=znver3 -flto=auto` **después** de `ULTRA_PROD`: `-fhardened`, las trampas UBSan, FORTIFY y CET siguen activos. La prueba completa sale VERDE (44/44): las minas siguen detonando su bandera con `-O3` y LTO.

Lo que se descartó a propósito: `-O3` en el kernel (sin opción oficial y desaconsejado por los desarrolladores del kernel), LTO del kernel (solo con Clang) y `-Ofast`/`-ffast-math` (rompe la coma flotante IEEE: es comportamiento indefinido a cambio de velocidad).

**Medida en QEMU** (`scripts/medir_perfiles.py`, 7 arranques por perfil sobre el disco de `A:\`, `evidencias/medida-perfiles-qemu.txt`):

| Perfil | Hasta el fin de la etapa de hardware (`uptime`) | Etapa de hardware guiada | Carga de módulos |
|---|---|---|---|
| `generico` | 410 ms | 17,2 ms | 14,8 ms |
| `pc` | 405 ms | 16,8 ms | 14,4 ms |

**Sin diferencia real** (≈1 %, dentro del ruido; `uptime` avanza de 10 en 10 ms). El arranque no está limitado por la CPU, sino por esperar a dispositivos y leer de disco. `pc` gana donde hay cálculo, y eso llegará con programas de verdad (fase 3 y siguientes). Falta medirlo en el metal.

El perfil `generico` añade:
- almacenamiento NVMe, AHCI y virtio;
- USB xHCI, almacenamiento USB y teclado o ratón HID;
- ext4 y VFAT;
- arranque EFI (`EFI_STUB`) y pantalla de firmware (`simpledrm`);
- las piezas AMD: microcódigo, `amd-pstate` e IOMMU;
- como módulos: la red Realtek `r8169`, el sonido `snd-hda-intel` y el sensor de temperatura `k10temp` del Ryzen.

Si falta cualquiera de estas opciones obligatorias, la construcción se niega.

Resultado (`evidencias/arranque-generico.log`):
- Linux `7.3.0-rc7`: bzImage de 17,5 MB con el initramfs incorporado, 1661 opciones integradas y 24 opciones como módulo (21 archivos `.ko` en `modulos.tar.gz`).
- La prueba completa sale VERDE: shell, minas, CET, la caja 11/11 y KFENCE.
- El kernel ve el bus PCI (controladores `ahci`, `xhci_hcd`, `virtio-pci`…) y soporta ext4.

Pendiente en esta fase:
1. ~~**Arrancar desde un disco**~~ **Hecho, sin GRUB:** `ultra-esp.img` es una GPT con la ESP FAT32 (el kernel como `EFI/BOOT/BOOTX64.EFI`), `ULTRA-SISTEMA-A` en EROFS de solo lectura y `ULTRA-DATOS` en ext4. El init las monta por nombre de partición. La prueba en dos arranques sale VERDE: un archivo escrito en `/datos` sigue ahí tras reiniciar (`evidencias/arranque-disco-persistente.log`). Ver [ARRANQUE-SIN-GRUB.md](ARRANQUE-SIN-GRUB.md).
2. ~~**Cargar módulos de verdad**~~ **Hecho:** los módulos viven en `ULTRA-SISTEMA-A`. En el primer arranque, el init compara los `modalias` PCI y USB con `modules.alias`, carga lo necesario y escribe `/datos/PC.configuración`. Después carga solo esa lista. En QEMU no gana tiempo (−86 µs: comparar 239 alias cuesta 0,7 ms y la carga domina), y así queda medido. **Siguiente:** probarlo en el PC con [PRUEBA-PC-REAL.md](PRUEBA-PC-REAL.md).
3. **Perfil de tu PC**: afinar para el Ryzen 5600X y la A320M, y preparar `nvidia-open` (un módulo externo compilado contra este árbol, con DRM/KMS activos) para la RTX 3060.
4. Pasar los mismos detectores del perfil vigía sobre la configuración genérica.

El laboratorio de velocidad está diseñado pero sin implementar: ver [LABORATORIO-VELOCIDAD.md](LABORATORIO-VELOCIDAD.md).

## Hoja de ruta

1. **Fase 1: shell propia.** Hecha.
   **Fase 1.5: blindaje de C.** Hecha: tres guardianes, campo de minas y caja.
2. **Fase 2: kernel genérico x86-64 + perfil de tu PC.** Primer paso hecho (perfil `generico`); faltan el arranque desde disco y el perfil de tu PC. Arranque en cualquier PC moderno y perfil afinado para el Ryzen y la A320M: NVMe/SATA, USB, red, sonido, EFI y framebuffer. La RTX 3060 (Ampere) usará el módulo `nvidia-open`, que exige módulos de kernel activados.
3. **Fase 3: sistema base.** Disco con glibc y pacman sobre tu kernel; supervisión al estilo Void hecha con el init C blindado y servicios en cajas.
4. **Fase 4: gráficos y juegos.** Mesa, Vulkan con el driver NVIDIA, Wayland, KDE Plasma, gamescope y Steam.
5. **Fase 5: identidad ULTRA.** Raíz inmutable A/B al estilo SteamOS y ajustes del kernel medidos frente a la referencia.
6. **Fase 6: el PC real.** Arranque dual sin tocar Windows, con recuperación.
