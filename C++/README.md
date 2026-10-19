# Area C++

El [plan de aplicacion y estado actual](../docs/PLAN-GUARDIANES-GRAFICOS.md)
registra la ronda de guardianes CPU/GLES, limites de laboratorio y reemplazo
de programas con conservacion del anterior.

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

Para sondas CPU se conserva scripts/aislar-mina.sh; el runner grafico actual
ejecuta contratos y sondas GL mediante scripts/ejecutar-grafica-limitada.sh.
No se expone dispositivo GPU. Sanitizadores solo en diagnostico;
rendimiento se compara entre builds de produccion equivalentes.

La [sonda inicial](../evidencias/cxx/sonda-inicial.txt) compilo y termino
con salida 0 en ambos perfiles, dentro de bwrap sin /dev/dri. GCC declara
__cplusplus=202400 en este modo experimental. No hubo diagnosticos de
ASan/UBSan en esa ejecucion. Es un control de infraestructura, no una suite
de UB ni una demostracion de ausencia de fallos.

## GL/EGL: primer banco instalado y comprobado

El siguiente prototipo ya ofrece cache .gpus con modos estatico y dinamico:
ver [formato, controles y limites](GPUS.md). Solo banco GLES, sin integracion
en Hyprland ni actualizacion asincrona.

OpenGL/OpenGL ES son APIs: se compila una implementacion y el programa que
la utiliza. Mesa y el proveedor NVIDIA son implementaciones distintas.
nvidia-open es el modulo del kernel; no significa que podamos reconstruir
las bibliotecas GL de NVIDIA como si fueran Mesa. Primero identificar
proveedor, versiones y licencias de cada componente instalado.

Se instalaron mesa 26.2.5 y libglvnd 1.7.0 con sus dependencias en la VM
del taller. La [primera ejecucion grafica](../evidencias/cxx/grafica-04676d4802bd4c1599cd23038baacd2a/informe.txt)
registra el manifiesto completo, hashes del snapshot y comandos de compilacion.
Resultado: EGL 1.5, GLES 3.2, GLSL ES 3.20 y llvmpipe/LLVM 23.1.3.
Produccion y ASan/UBSan terminaron con codigo 0: cinco controles por perfil.
Se comprobaron pixel RGBA8, error de enum y recuperacion, shader invalido,
shader valido posterior y liberacion de recursos EGL. El shader valido se
compila. Aquella primera sonda no enlazaba programas; el banco .gpus posterior
si enlaza y dibuja geometria, segun su evidencia independiente.

Desde Windows, probar-grafica-cxx.ps1 crea un snapshot nuevo, compila ambos
perfiles y ejecuta scripts/ejecutar-grafica-limitada.sh en la VM. En Linux,
agregar -DULTRA_CXX_EGL=ON a las configuraciones anteriores y ejecutar
ese runner sobre laboratorio/sonda-egl del directorio de build. Requiere
el usuario del taller y sudo no interactivo para el servicio transitorio;
la carga ejecuta con su UID sin root. El script de aislamiento verifica
las propiedades reales del cgroup antes de lanzar el binario.
El sandbox no expone /dev/dri, red, home ni sockets del escritorio; fuerza
llvmpipe, limita hilos a dos y aplica timeout/limites de recursos. /usr se
expone en solo lectura y el kernel de la VM sigue siendo compartido.
El cgroup limita memoria a 512 MiB, swap a cero y tareas a 64. La evidencia
actual y los detalles de contratos/recuperacion estan en el plan de aplicacion.

Estas banderas instrumentan nuestro ejecutable C++, no las bibliotecas Mesa
precompiladas. No se reconstruyo Mesa ni Hyprland, no se probo la RTX y no
hay optimizacion de rendimiento demostrada. El perfil C++ conserva RTTI y
excepciones y no usa fast-math ni LTO. Para reconstruir Mesa, usar su build
Meson y perfiles por lenguaje/dependencia, sin imponerle este perfil C++.

Mesa distribuye el trabajo entre src/egl, src/mesa, src/compiler y Gallium,
incluido llvmpipe; no es un unico archivo OpenGL.cpp.
[Arbol oficial](https://docs.mesa3d.org/sourcetree.html),
[EGL en Mesa](https://docs.mesa3d.org/egl.html).
En Hyprland, el renderer C++ y los shaders GLSL tienen compiladores y
contratos diferentes. El banco toma GLES3 como base comun, pero no ejecuta
los shaders ni el compositor de Hyprland todavia.
[Categorias](laboratorio/categorias.md) y
[plan grafico](../docs/HYPRLAND-LABORATORIO-GRAFICO.md).
