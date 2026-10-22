# Reutilizacion de recursos OpenGL ES y Vulkan

Fecha: 2026-10-22, America/Lima. Continuacion del
[banco de vida util](GRAFICA-VIDA-UTIL.md). Objetivo: reducir creaciones repetidas
manteniendo el resultado y las dependencias necesarias. Alcance: fuentes propias
del laboratorio; no modifica Hyprland, NVIDIA, Brave, Proton ni las imagenes de A:.

## GLES: actualizar en lugar de recrear

`Texture::update_rgba8` conserva nombre, dimensiones y almacenamiento existentes
y carga los nuevos pixels mediante `glTexSubImage2D`. Conserva los controles de
contexto, capacidad y presupuesto y rechaza objetos cerrados o movidos y errores
GL anteriores. Guarda/restaura binding, PBO y parametros de desempaquetado, igual
que la creacion. No mantiene una cache de estado que pueda quedar desactualizada
por otras llamadas GL. La dimensiones viajan con el objeto al moverlo.

G-12 verifica 64 actualizaciones con contenido distinto, leyendo el pixel de cada
una desde un framebuffer. Todas conservan el nombre original, incluso con PBO y
stride externos adversos. Rechazar capacidad/presupuesto insuficiente o un error
previo conserva el contenido. Tambien se prueban contexto ausente/ajeno, objeto
cerrado y objeto movido. La clase sigue exigiendo que su contexto sobreviva al
objeto y que el llamador no destruya o redefina el recurso por un alias externo.

## Vulkan: reutilizar despues de completar

V-04 compara dos rutas con la misma operacion de 4096 bytes:

| Ruta | Buffers/asignaciones/pools/command buffers/fences | Grabaciones | Envios |
|---|---|---|---|
| Recrear en cada ciclo | 64 de cada uno | 64 | 64 |
| Reutilizar tras fence | 1 de cada uno | 1 | 64 |

Cada envio ejecuta `vkCmdFillBuffer`, una barrera TRANSFER_WRITE a HOST_READ y
una espera de fence con plazo de un segundo. Solo despues se comprueban las
1024 palabras del resultado. Antes de cada envio se rellena el mapeo con ceros:
un resultado viejo no basta para pasar. Memoria coherente; no se afirma soporte
para memoria no coherente, varias colas ni trabajo solapado.

Se conserva la barrera necesaria y se evita introducir `vkDeviceWaitIdle` o
`vkQueueWaitIdle` en cada ciclo. No se reutiliza un command buffer pendiente.
Si submit falla o no se confirma la finalizacion, el proceso desechable termina
con error sin ejecutar destructores sobre recursos potencialmente pendientes.
El aislamiento exterior tambien tiene limites de tiempo y memoria. Esta ruta
de fallo no es recuperacion de un dispositivo real y no se indujo un cuelgue.

La sonda solicita explicitamente validacion de sincronizacion mediante
`VkValidationFeaturesEXT`. Se mantienen la validacion ordinaria, el rechazo
exacto del buffer de tamano cero y el control de error tardio de la entrega previa.

## Verificacion y limites

[Informe final](../evidencias/cxx/grafica-92e0ed646a36417f9194aee8dc401360/informe.txt):
snapshot independiente, opciones, versiones y hashes. O2 y ASan/UBSan O1,
llvmpipe dentro del taller QEMU, sin exponer la RTX ni el escritorio personal.
El ROJO del control negativo tardio es deliberado y su salida 1 es obligatoria.
El ejecutor completo termino con salida 0; ambos perfiles completaron G-12 y
V-04, sin errores inesperados de validacion ni diagnosticos ASan/UBSan/LSan.

Se comprueba una reduccion estructural de creaciones y grabaciones. No se midio
una aceleracion, FPS, frametime ni ahorro de VRAM fisica. El diagnostico con capas
y sanitizadores no es una medicion de rendimiento. Siguen pendientes benchmarks
sin instrumentacion, cache de pipelines (todavia no hay pipelines en esta sonda),
dependencias entre etapas de dibujo y mediciones nativas antes de integrar un
consumidor real. No se eliminan barreras o comprobaciones globalmente.

Referencias: [actualizacion de textura GLES](https://registry.khronos.org/OpenGL-Refpages/es3.0/html/glTexSubImage2D.xhtml),
[dependencias y lectura desde CPU en Vulkan](https://docs.vulkan.org/guide/latest/synchronization_examples.html),
[cache de pipelines](https://docs.vulkan.org/guide/latest/pipeline_cache.html).
