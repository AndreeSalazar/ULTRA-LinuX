# Primer cambio en el comando grafico de ULTRA

Fecha: 2026-10-22, America/Lima. Esta entrega modifica un consumidor existente:
`src/rootfs/bin/ultra-graficos`, utilizado por el menu del escritorio. Integra una
politica de cierre y error para la prueba Vulkan. Las clases C++ de texturas
siguen en el banco: no se insertaron en Hyprland ni en las bibliotecas NVIDIA.

## Cambio observable

Antes, el comando mataba normalmente `vkcube` a los 12 segundos y aceptaba el
codigo 124 de timeout como exito. El proceso podia terminar sin recorrer su
destruccion normal de objetos Vulkan.

Ahora solicita `--c 120`: el cubo termina por su contador de fotogramas, usando
la ruta normal de limpieza del programa. Un plazo exterior de 30 segundos,
seguido de dos segundos de gracia, queda como proteccion ante bloqueo. Un timeout
ya es fallo, igual que cualquier otro codigo no cero. La notificacion describe
esa diferencia y el log conserva codigo de salida y fotogramas solicitados.
Cerrar la ventana normalmente tambien puede devolver cero: no se presenta el
contador solicitado como una medicion de fotogramas realmente mostrados.

La validacion permanece limitada al proceso de prueba, sin modificar el entorno
del compositor ni los juegos. Esta mejora afecta vida util y diagnostico; no
acredita mas FPS ni integra las optimizaciones C++ en un renderer externo.

## Pruebas reales y controles de error

[Ronda final](../evidencias/cxx/grafica-dda73a32cc2c47e58334738e13d38c1b/informe.txt):
el ejecutor completo termino con salida 0. La integracion corre como UID 1000,
con Cage/Wayland headless, compositor pixman y Vulkan llvmpipe. Se comprueba en
el registro del cargador que se inserto VK_LAYER_KHRONOS_validation.

- INTEGRACION-01: `vkcube` real, cierre con codigo 0 y sin errores de validacion.
- INTEGRACION-02: timeout simulado; se conserva codigo 124 y se registra el fallo.
- INTEGRACION-03: error simulado de `vkcube`; se conserva codigo 7.

Los dos ultimos controles usan ejecutables ficticios solo en el PATH del
laboratorio. La fuente de produccion no incluye modos especiales para simularlos.
No se indujo un bloqueo real. Las notificaciones D-Bus no se comprueban en el
sandbox sin bus; su fallo previsto no impide el dibujo y queda en el informe.
El compositor informa tambien que no hay DRM lease manager en este entorno sin
GPU; no es un error de validacion Vulkan.

El [primer intento](../evidencias/cxx/grafica-0d1596f6c6b94a07b502babaa99d9052/informe.txt)
termino por RLIMIT_FSIZE: el limite de 1 MiB impedía crear un buffer compartido
Wayland; el log de diagnostico solo ocupaba 641 bytes. Se permite 8 MiB por
archivo exclusivamente a `probar-ultra-graficos.sh`. Las otras sondas mantienen
1 MiB. Siguen vigentes 512 MiB de memoria total, tmpfs de 64 MiB, swap cero,
64 tareas, plazo externo y ausencia de /dev/dri. No se eliminan limites para que
la prueba pase.

## Cargas GLES ampliadas

G-14 se ejecuta en O2 y ASan/UBSan: dimensiones 1x1, 31x17, 256x128 y 1920x1080.
La mayor textura usa 8.294.400 bytes RGBA8. Se actualizan tres patrones que
dependen de fila, columna y fotograma; se compara el readback completo, no un
unico pixel. Despues se rechaza una capacidad de entrada un byte insuficiente
y se vuelve a comparar la imagen completa para comprobar que no cambio.
Presupuesto por textura: 16 MiB. No mide rendimiento de esas dimensiones.

Los benchmarks existentes siguen midiendo las cargas anteriores de 4x4 y
buffers de 4096 bytes. Sus nuevas muestras se agregan al historial CSV y no se
mezclan con G-14 ni se atribuyen al cambio del comando.

## Alcance de la integracion

INT-02 esta integrado en la fuente del comando y probado con Wayland real en
software. INT-01 sigue pendiente: elegir un consumidor de las clases C++ y
medir/regresar ese cambio. Un paquete Hyprland instalado no equivale a un
renderer propio modificado. Tampoco se declara terminada MED-02: todavia falta
medir cargas grandes y una escena representativa.

No se recompilaron ni desplegaron las imagenes de A: en esta entrega. El siguiente
despliegue debe empaquetar el helper, verificar el arranque completo y preservar
los datos personales. La prueba fisica RTX continua fuera del alcance actual.

Referencia: [contador y limpieza de vkcube upstream](https://github.com/KhronosGroup/Vulkan-Tools/blob/main/cube/cube.c).
