# Reconstrucción del 21 de octubre de 2026

**254 comprobaciones aprobadas, sin fallos finales.** Ambos perfiles se
reconstruyeron y descargaron al proyecto principal de Windows.

| Perfil | Compilación del kernel | Arranque, escritorio, Brave y guardianes | Audio/juegos |
|---|---:|---:|---:|
| Genérico | #45 | 107 | 18 |
| PC | #31 | 111 | 18 |

Los 11 artefactos descargados coinciden con sus SHA256. Las imágenes EROFS
ocupan 1146601472 bytes (genérico) y 1733001216 bytes (PC). El disco doble
contiene ambos perfiles y la recuperación. `artefactos-verificados.json`
registra tamaños y hashes; `resumen.json`, los resultados por perfil.

El README se reorganiza alrededor del escritorio actual, las funciones,
controles, persistencia, procedencia y clasificación como distribución personal
experimental basada en Linux y paquetes Arch. Sus imágenes son capturas nuevas
de esta ejecución QEMU, sin retoques. No miden la RTX física.

## Mejoras verificadas

- El informe consulta cada punto de montaje por separado. Dos argumentos de
  findmnt se interpretaban como origen y destino y omitían los montajes reales.
- El filtro del kernel incluye audio USB, además de HDA y GPU.
- Estado de juegos continúa aunque /juegos no esté montado y explica el home
  temporal, sin interrumpir el diagnóstico de Vulkan.

La sintaxis Bash y el caso sin montajes se comprobaron además en el taller:
ambos puntos figuran como NO MONTADO y el estado continúa. El taller no tiene
vulkaninfo instalado; las imágenes de escritorio sí incluyen esa herramienta.
`fuentes-diagnostico.json` identifica los dos ayudantes y sus hashes coinciden
con las fuentes enviadas al taller.

## Alcance y archivos

`generico/` y `pc/` contienen resultados, manifiestos, procedencia de Brave y
Heroic y capturas nuevas del escritorio, Brave y Steam. `registros.zip`
conserva compilaciones y consolas completas de esta ronda.

Ambos ensayos de audio/juegos reutilizan únicamente la caché del banco
sintético, sin cuentas personales. Comprueban PipeWire con USB emulado,
prioridad, boost, silencio, muestras PCM, Steam con formulario visible,
Heroic Wayland, permisos y persistencia tras reiniciar. No prueban el juego
real ni sustituyen la audición del dispositivo físico.

NVIDIA PC conserva un aviso amarillo: 11674 líneas de objtool sobre retornos
sin protección MITIGATION_RETHUNK. No son funciones únicas y no se afirma que
esa superficie esté completamente cubierta; requiere comprobación de SRSO en
el PC real. Se conserva `pc/nvidia-guardian.txt`.

La biblioteca de imágenes de las pruebas emite una advertencia de API futura
sobre getdata; no impidió verificar las capturas ni cambió el resultado.

Tras las pruebas, el usuario solicitó reinstalar en A:, que había vaciado para
otros experimentos. Se verificó BMO FAT32 de datos, disco 0, partición 2, y se
restauró datos.ext4 desde el respaldo principal cuyo hash coincide con el recibo.
Se copiaron ambos kernels y sistemas ya probados y se comprobaron sus hashes
en destino. Las entradas F11 existentes se reutilizan sin alterar su orden.
`instalacion-A.json` conserva la verificación. No se modifica el almacenamiento
personal de D: ni se utiliza en el banco de pruebas.
El taller se apagó limpiamente al acabar la construcción; las máquinas de
prueba también terminaron con apagado. No se deja un servicio nuevo ejecutándose.
