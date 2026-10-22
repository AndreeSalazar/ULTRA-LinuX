# Medicion de reutilizacion grafica en software

Fecha: 2026-10-22, America/Lima. Objetivo: comprobar si evitar creaciones repetidas
reduce tiempo en las sondas propias. No se modifico el escritorio ni A:.

## Resultado observado

Tiempo por muestra de 2048 operaciones, mediana de las tres medianas por proceso:

| API / carga | Recrear | Reutilizar | Reduccion observada del tiempo total |
|---|---:|---:|---:|
| GLES, texturas RGBA8 de 4x4 | 5,958 ms | 3,414 ms | 42,7 % |
| Vulkan, fill de buffers de 4096 bytes | 162,781 ms | 107,340 ms | 34,1 % |

La reutilizacion fue mas rapida segun la mediana en los tres procesos de cada
API. Son microbenchmarks de llvmpipe en QEMU: los porcentajes no describen FPS,
Hyprland, Brave, Proton, los juegos ni el rendimiento de la RTX.

## Protocolo reproducible

`probar-grafica-cxx.ps1` crea un snapshot nuevo y primero ejecuta las pruebas de
correccion O2 y ASan/UBSan, incluyendo validacion de sincronizacion Vulkan. Luego
comprueba que ambos ejecutables instrumentados rechazan `--medir` con salida 2.
Finalmente lanza tres procesos nuevos por API con el ejecutable O2 y `--medir`.
Cada proceso calienta ambas rutas, sin registrar ese calentamiento, y mide siete
pares alternando cual ruta se ejecuta primero. Se conservan todas las muestras.

El modo Vulkan de medicion solicita cero capas y cero extensiones de validacion;
el aislamiento tambien deshabilita las capas mediante el cargador. Nunca ejecuta
los controles negativos de API en ese modo. El modo ordinario conserva la
validacion y esos controles. No se mide con ASan/UBSan.

Ambas rutas completan la misma carga y comprueban el resultado en cada operacion:

- GLES: 2048 cargas, con rojo variable y lectura del pixel desde framebuffer.
  Recrear usa 2048 texturas; reutilizar usa una creacion y 2047 actualizaciones.
  Ambas rutas realizan las mismas comprobaciones y lecturas.
- Vulkan: 2048 envios, cada uno con fill, barrera TRANSFER_WRITE a HOST_READ,
  fence y comparacion de las 1024 palabras, previamente envenenadas con ceros.
  Recrear crea 2048 conjuntos; reutilizar crea y graba uno. Se conserva la espera
  de finalizacion antes de reutilizar. No hay trabajos solapados.

El tiempo total usa reloj monotono e incluye creacion/destruccion del conjunto,
lecturas, esperas y comprobacion. Excluye inicializacion de EGL/Vulkan y el
calentamiento. El reloj de CPU es CLOCK_PROCESS_CPUTIME_ID, incluyendo los hilos
del proceso; no es tiempo de ejecucion GPU. El RSS es el pico acumulado del
proceso, no memoria instantanea de cada ruta. No se midio VRAM.

La primera ronda de 64 operaciones usaba `std::clock` y produjo ceros de CPU en
muestras GLES demasiado cortas. No se usa para la conclusion: la entrega aumenta
la carga a 2048 y cambia el reloj de CPU. Ese intento queda archivado localmente
en build, separado de la evidencia final.

## Evidencia y verificacion

[Informe crudo](../evidencias/cxx/grafica-b8db178f07844b4c9042f39ce9004d94/informe.txt),
[resumen con tiempos CPU y variacion](../evidencias/cxx/grafica-b8db178f07844b4c9042f39ce9004d94/resumen.json).
El ejecutor termino con salida 0. Los tres ROJO deliberados corresponden al
control de error Vulkan tardio y a los dos rechazos de medicion instrumentada.
No hubo errores inesperados de validacion ni diagnosticos ASan/UBSan/LSan.

Mesa 26.2.5, LLVM 23.1.3, G++ 16.2.1, taller Arch en QEMU/WHPX con 4 vCPU y
4 GiB. Cada proceso tiene 512 MiB, dos hilos de llvmpipe y tiempo limitado; no
dispone de GPU fisica, red ni sockets del escritorio. El host y sus otras cargas
no estan controlados como en un equipo dedicado a benchmarking.

Las medianas por proceso GLES fueron 5,937-6,081 ms al recrear y 3,383-3,426 ms
al reutilizar; en Vulkan, 161,223-168,939 y 105,441-109,113 ms respectivamente.
Se conserva tambien la dispersion de muestras: por ejemplo hubo una muestra
GLES de recreacion de 9,362 ms. No se eliminan valores lentos del informe.

El resumen se puede regenerar con `scripts/resumir-benchmark-grafico.py`, pasando
el informe como argumento. Rechaza rondas incompletas, orden incorrecto, tiempos
no validos o discrepancias con las medianas del ejecutable. Su comprobacion
negativa elimina una muestra y altera una mediana, exigiendo rechazo de ambos.

## Decision y siguiente limite

Conservar la reutilizacion como candidata y el benchmark como referencia local.
No se justifica quitar barreras, comprobaciones de capacidad ni propiedad por
estos tiempos. Hace falta una carga representativa y medicion nativa en la RTX
antes de integrar cambios en un renderer real. Memoria no coherente, pipelines,
colas multiples y recuperacion de dispositivo siguen fuera de esta comparacion.
