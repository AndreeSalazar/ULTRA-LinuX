# Anti3M aplicado a Linux: el kernel por categorías

"Linux tiene más de 30 millones de líneas" es cierto, y a la vez engaña. Leído por **categorías** (la técnica Anti3M), la mayoría no toca a ningún PC concreto, y lo que sí importa cabe en una lista corta. Este documento mide el árbol del kernel que usa ULTRA y separa lo que de verdad se compila.

## Cómo se mide

`scripts/medir-categorias.sh [perfil]` se ejecuta en el taller (`bash medir-categorias.sh pc`):
- **Árbol:** las líneas de todos los `.c`, `.h`, `.S` y `.rs` versionados, por directorio de primer nivel.
- **Compilado:** un `.c` o `.S` por cada objeto del perfil, más cada cabecera que leyó alguno de ellos. La lista sale de los archivos de dependencias `.cmd` de Kbuild, con cada ruta normalizada y contada una sola vez.

Son líneas completas, con comentarios y líneas en blanco. Las cabeceras generadas durante la compilación no se cuentan, porque no están en el árbol.

## Resultado: perfil `pc`, kernel `32c52ec87d62` (`evidencias/categorias-kernel-pc.txt`)

| Categoría | Líneas en el árbol | Compiladas | Usado |
|---|---|---|---|
| `drivers` | 26 420 192 | 1 271 032 | 4,8 % |
| `arch` | 2 525 833 | 242 314 | 9,6 % |
| `sound` | 1 682 975 | 50 485 | 3,0 % |
| `fs` | 1 653 587 | 335 677 | 20,3 % |
| `include` | 1 476 844 | 714 622 | 48,4 % |
| `tools` | 1 459 127 | 23 464 | 1,6 % |
| `net` | 1 304 423 | 513 348 | 39,4 % |
| `kernel` | 573 978 | 307 435 | 53,6 % |
| `lib` | 327 873 | 99 262 | 30,3 % |
| `mm` | 228 917 | 143 581 | 62,7 % |
| `rust` | 160 644 | 0 | 0 % |
| `security` | 129 001 | 56 521 | 43,8 % |
| `crypto` | 104 593 | 30 765 | 29,4 % |
| `block` | 73 393 | 42 418 | 57,8 % |
| `io_uring` | 27 627 | 27 001 | 97,7 % |
| `ipc` | 9 885 | 9 885 | 100 % |
| `init` | 4 771 | 3 814 | 79,9 % |
| resto (`scripts`, `samples`, `virt`, `certs`, `usr`, `Documentation`) | 104 257 | 17 338 | — |
| **Total** | **38 267 920** | **3 888 962** | **10,2 %** |

Se compilan 3157 archivos `.c`/`.S` y se leen 4276 cabeceras.

## Lo que dicen las categorías

- **El 69 % del árbol son controladores** (`drivers`), y ULTRA usa el 4,8 %. Los "30 millones" son sobre todo hardware que este PC no tiene. `PC.configuración` es la misma idea a escala de un PC: 50 dispositivos detectados, 3 módulos necesarios.
- **El corazón se usa casi entero:** `kernel` 54 %, `mm` 63 %, `block` 58 %, `init` 80 %, `ipc` 100 %. Es poco código y es lo primero que conviene entender.
- **`include` se usa a la mitad**, y contiene los **contratos** con los programas (`include/uapi`).
- **`tools`, `samples` y `Documentation` no forman parte del kernel** que arranca. `rust` está a 0 % porque este perfil no activa Rust.
- **Lo propio de ULTRA:** unas 3500 líneas de C (init, particiones, hardware, tarjeta, caja y guardianes) y unas 1900 de scripts.

## Linux por papeles

| Papel | Directorios | Qué es | Para ULTRA |
|---|---|---|---|
| **Núcleo** | `kernel`, `mm`, `block`, `ipc`, `init` | Procesos, planificador, memoria, disco | Casi todo se usa; el estudio empieza aquí |
| **Contratos** | `include/uapi`: syscalls, `/proc`, `/sys`, ioctl | Lo que Linux promete a los programas | **No se rompe nunca**: es lo que permite usar apps de Arch y de Steam |
| **Controladores** | `drivers`, `sound` | Uno por cada pieza de hardware | Enorme, pero cada PC usa un trozo mínimo; se elige por detección |
| **Arquitectura** | `arch/x86` | Lo específico de la CPU | El perfil `pc` (Zen 3) se afina aquí |
| **Servicios** | `fs`, `net`, `security`, `crypto`, `lib` | Archivos, red, aislamiento, cifrado | Se eligen por función: EROFS, ext4, VFAT, Landlock, seccomp |
| **Herramientas** | `tools`, `scripts`, `samples`, `Documentation` | Fuera del kernel que arranca | Se pueden ignorar al estudiar |

## Qué falta para Wayland, por las mismas categorías

| Categoría | Qué necesita Wayland | Estado en ULTRA |
|---|---|---|
| Controladores: pantalla | Un driver DRM/KMS | ✅ `simpledrm` sirve para empezar sin GPU; la RTX 3060 necesita `nvidia-open` |
| Controladores: entrada | evdev (`/dev/input/event*`) | Por comprobar: viene en `defconfig` |
| Contratos | memfd, memoria compartida, sockets Unix | ✅ En el kernel |
| Espacio de usuario: base | glibc dinámica y el cargador de programas | ❌ Solo hay BusyBox estática: es la **fase 3** |
| Dispositivos y sesión | Algo que haga de udev, y `seatd` | ❌ |
| Bibliotecas | libwayland, libinput, libxkbcommon y sus mapas, pixman, freetype, fuentes | ❌ |
| Compositor y cliente | `cage` o `weston`, y una terminal como `foot` | ❌ |
| Gráficos acelerados | Mesa o el driver NVIDIA de espacio de usuario | ❌ Más adelante |

Al kernel casi no le falta nada: lo que falta es **el espacio de usuario**. Son unos 20–30 paquetes de Arch para un Wayland mínimo dibujado por software. El orden está en [PLAN.md](PLAN.md#enfoque-compatible-no-copia-18-10-2026): arranque con vuelta atrás, fase 3 (glibc y paquetes de Arch dentro del EROFS) y un primer Wayland con `cage` y `foot` sobre `simpledrm`.

## Licencia y autoría

ULTRA (el init, la caja, los guardianes, los scripts y el método) es obra propia y se comparte bajo GPL-2.0. El kernel sigue siendo de sus autores, también bajo GPL-2.0, y se usa sin modificar. Lo que este proyecto demuestra es saber elegirlo, medirlo y vigilarlo por categorías.
