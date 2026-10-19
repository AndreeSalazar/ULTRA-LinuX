# Area C++

Separada del guardian C17 existente. No mueve codigo de arranque ni cambia
la imagen instalada. produccion/ contiene el perfil de compilacion para
fuentes propias; laboratorio/ contiene sondas y el catalogo de casos.
El perfil no es un guardian completo ni convierte un paquete en una caja.

## Toolchain verificado

2026-10-19: el taller dispone de G++ 16.2.1 20260810 y CMake 4.4.4.
Hyprland main consultado declara CMAKE_CXX_STANDARD 26; su CI tiene un
trabajo gcc sobre Arch. La wiki indica GCC >= 16 o Clang >= 19 con biblioteca
estandar compatible. No hay un unico compilador usado por todas las builds.
G++ es el frontend C++ de GCC; GLSL tiene su propio compilador.

Fuentes: [CMake upstream](https://github.com/hyprwm/Hyprland/blob/main/CMakeLists.txt),
[CI upstream](https://github.com/hyprwm/Hyprland/blob/main/.github/workflows/ci.yaml),
[instalacion](https://wiki.hypr.land/Getting-Started/Installation/).
HEAD remoto observado: f6e3a723abdbe64e2d0b36fda9f8985e5b6e040e.
Las paginas main consultadas son mutables y no constituyen un snapshot de
ese commit. Antes de construir Hyprland, descargar y verificar su revision
exacta y dependencias. Este banco aun no construye Hyprland.

La sonda pide modo C++26 y comprueba expected/format y limites aritmeticos;
no certifica que GCC implemente todo el estandar. No aplicar sin revision
el perfil C static-pie, -fno-exceptions, -fno-rtti o banderas de ABI a Hyprland.
Upstream desactiva LTO por posibles problemas con plugins; conservar esa
decision hasta medir una alternativa compatible.

## Construccion del banco en Linux

Desde la raiz del repositorio:

```sh
cmake -S 'C++' -B build/cxx-produccion -DCMAKE_CXX_COMPILER=g++
cmake --build build/cxx-produccion
cmake -S 'C++' -B build/cxx-diagnostico -DCMAKE_CXX_COMPILER=g++ -DULTRA_CXX_DIAGNOSTICO=ON
cmake --build build/cxx-diagnostico
```

Ejecutar cada sonda mediante scripts/aislar-mina.sh. Los binarios propios
son pruebas CPU sin dispositivo GPU. Sanitizadores solo en diagnostico;
rendimiento se compara entre builds de produccion equivalentes.

La [sonda inicial](../evidencias/cxx/sonda-inicial.txt) compilo y termino
con salida 0 en ambos perfiles, dentro de bwrap sin /dev/dri. GCC declara
__cplusplus=202400 en este modo experimental. No hubo diagnosticos de
ASan/UBSan en esa ejecucion. Es un control de infraestructura, no una suite
de UB ni una demostracion de ausencia de fallos.

## GL/EGL es el siguiente banco

OpenGL/OpenGL ES son APIs: se compila una implementacion y el programa que
la utiliza. Mesa y el proveedor NVIDIA son implementaciones distintas.
nvidia-open es el modulo del kernel; no significa que podamos reconstruir
las bibliotecas GL de NVIDIA como si fueran Mesa. Primero identificar
proveedor, versiones y licencias de cada componente instalado.

El taller no encontro egl.pc ni glesv2.pc: faltan dependencias de desarrollo
en su entorno. Antes de crear el contexto de prueba, preparar esas dependencias
en un entorno de laboratorio y guardar su manifiesto. Empezar por Mesa software
sin /dev/dri; registrar EGL_VENDOR/GL_VENDOR/GL_RENDERER/GL_VERSION y errores.
No presentar ese resultado como prueba de RTX. No se ha compilado ni ejecutado
GL/EGL en esta fase. [Categorias](laboratorio/categorias.md) y
[plan grafico](../docs/HYPRLAND-LABORATORIO-GRAFICO.md).
