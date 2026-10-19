# Plan inicial y criterios para avanzar

1. **Origen fijado.** Descargar el árbol oficial, registrar commit y licencias. Un espejo superficial no equivale al historial completo. El hash identifica contenido; no sustituye la verificación de una firma de publicación.
2. **Entorno Linux en QEMU.** Taller Arch en disco virtual separado, GCC, binutils y dependencias. Registrar versiones. WSL queda fuera de la ruta elegida. Conservar el kernel de trabajo separado del kernel que ejecuta el taller.
3. **Referencia sin cambios.** Compilar fuera de fuentes con `O=`, guardar configuración, commit y registro. Arrancar en QEMU con un initramfs mínimo y demostrar consola. La imagen de usuario y su init son una tarea pendiente.
4. **Guardianes.** Separar controles de construcción, pruebas y mecanismos de ejecución. Definir para cada uno: recurso protegido, condición, acción, motivo, prueba positiva/negativa y coste. Una revisión textual no prueba seguridad de memoria.
5. **Experimentos pequeños.** Ramas propias, un objetivo por cambio y comparación contra la referencia. Medir tiempo de arranque, memoria y rendimiento con condiciones repetibles. Conservar una configuración recuperable.
6. **Sistema de usuario.** Elegir bibliotecas, init, consola, paquetes y actualizaciones. Inspiración Arch no implica reutilizar sus paquetes sin respetar sus dependencias y ABI.
7. **Ecosistema Linux.** Aplicaciones gráficas (Chromium entre otras), compiladores, servicios, juegos compatibles, sonido y bibliotecas. Definir una matriz de pruebas por funciones y aplicaciones.  Integrar almacenamiento, USB, red, gráficos, entorno de pantalla, bibliotecas, fuentes y aislamiento. Mantener las funciones necesarias para el sandbox. POSIX por sí solo no garantiza compatibilidad con binarios Linux ni con Chromium.
8. **Prueba física.** Solo tras superar QEMU y definir recuperación. Conservar Windows y las entradas actuales. La GPU y su controlador se identificarán antes de afirmar aceleración gráfica.

## Primera meta cerrable

Kernel oficial sin cambios compilado con GCC, commit/configuración/herramientas registrados y consola propia visible en QEMU. No incluir Chromium ni una reescritura de POSIX en esta primera meta.

## Registro por experimento

Pregunta; commit base; cambio; configuración; herramienta; prueba; resultado; comparación; clasificación ROJO/AMARILLO/VERDE; motivo; limitaciones; acción siguiente. Ningún resultado verde debe extrapolarse a todo el sistema.
## Enfoque: compatible, no copia (18-10-2026)

ULTRA LinuX no fusiona distribuciones ni usa sus nombres o marcas. Tiene **su propia base** (kernel, init C con guardianes, sistema inmutable en EROFS) y **habla el idioma** de cada ecosistema, para que sus aplicaciones funcionen:

| Ecosistema | Cómo llega | Qué tiene que ofrecer ULTRA |
|---|---|---|
| Arch (pacman, AUR) | Paquetes compatibles con Arch | glibc y rutas `/usr` con la misma ABI (fase 3) |
| SteamOS (Steam, Proton, gamescope) | Programas de Arch sobre una raíz inmutable | NVIDIA, Vulkan, Wayland (fase 4) |
| Flatpak | Apps de cualquier distribución, empaquetadas aparte | Espacios de nombres de usuario, `bubblewrap` |
| Ubuntu, Fedora, Debian… | `distrobox` o `podman` | Contenedores y cgroups v2 |
| Windows | Wine y Proton | Lo mismo que Steam |

La raíz sigue inmutable y las apps viven aparte (`/datos`, Flatpak, contenedores): una app rota no toca el sistema. Las apps ajenas usan su propia caja (`bubblewrap`, `pressure-vessel`); la `caja` de ULTRA protege la base y gana perfiles medidos por ecosistema, uno a uno.

### Método: abrir, entender, vigilar, facilitar

Cada pieza que entra (Wayland, Mesa, Steam, Flatpak…) pasa por lo mismo antes de darla por buena:
1. **Abrir**: leer su código y documentación, y saber de qué está hecha y por qué.
2. **Medir**: syscalls con `strace`, memoria, tiempos y archivos que toca. Con números, no suposiciones.
3. **Vigilar**: aplicarle los guardianes de ULTRA (lista blanca medida, Landlock, compilación con guardianes cuando sea posible, campo de minas propio) y anotar qué despierta.
4. **Exponer y facilitar**: dejar escrito en `docs/` cómo funciona y qué se cambió, para que lo siguiente cueste menos.

### Wayland: la meta divertida

La llegada de Wayland es el primer gran objetivo de este enfoque. Cuando llegue, se estudiará pieza a pieza: el protocolo (sockets Unix y memoria compartida), el compositor (wlroots, KWin o gamescope), DRM/KMS y GBM para la pantalla, libinput para la entrada, Mesa y Vulkan, y XWayland para las apps antiguas. Puntos donde los guardianes pueden aportar: aislar cada cliente en su caja, medir las syscalls reales del compositor, someter el protocolo a pruebas con mensajes malformados y vigilar la memoria del compositor con el perfil vigía.

### Orden

1. **Arranque con vuelta atrás**: el perfil afinado (`pc`) se prueba solo y, si falla tres veces, vuelve al genérico. Es la red de seguridad de todo lo demás.
2. **Fase 3**: base con glibc y pacman compatible con Arch.
3. **Flatpak y distrobox**: abren casi todos los ecosistemas a la vez.
4. **Fase 4**: NVIDIA, Vulkan, **Wayland**, gamescope y Steam.
