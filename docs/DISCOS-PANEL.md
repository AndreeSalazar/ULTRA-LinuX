# Discos y bibliotecas: estado visible y revision automatica

2026-10-22, America/Lima.

Super+Espacio > **Discos y bibliotecas > Panel de discos y preparacion** abre un panel que permanece visible y
actualiza el estado cada segundo. Super+J > **Elegir carpetas y discos** abre
el mismo panel. Usa 1 para elegir/quitar carpetas, 2 o R para revisar otro
minuto, 3 para abrir una carpeta activa y Escape para volver. Flechas izquierda
y derecha eligen una accion, Enter la ejecuta; arriba/abajo recorren el informe.

Al entrar al escritorio se inicia una revision de 60 segundos en segundo plano.
El panel permite salir y volver sin perder esa revision. Un bloqueo evita
trabajadores duplicados. El resultado queda en el home persistente, bajo
`~/.cache/ultra/discos/automatico.json`; los errores quedan visibles. Una
operacion de registro en curso puede prolongar el minuto: no se corta a medias
una escritura de metadatos. El estado se actualiza entre operaciones.

La revision lee el catalogo de montajes ya preparado, comprueba las selecciones
y registra las bibliotecas Steam activas cuando Steam esta cerrado. Si esta
abierto, espera durante ese minuto; despues puede repetirse con R. No inicia
Steam ni descarga/verifica juegos. Los registros Linux de bibliotecas se
serializan entre este proceso y el lanzador habitual de Steam.

Las carpetas nuevas se guardan inmediatamente, pero aparecen **pendientes**
hasta salir y volver al escritorio. La fase privilegiada de entrada prepara sus
montajes y la revision comprueba el resultado al regresar. El panel no obtiene
permisos de root, no formatea y no cambia montajes durante una partida.
Para activar lo pendiente: guarda tu trabajo, pulsa **Super+Shift+M** y vuelve
a abrir el escritorio desde el menu **e**. No necesitas volver a elegir la carpeta.
La navegacion recuerda la ultima carpeta incluso al cancelar; el selector ofrece
**Continuar en...** si ese disco y carpeta siguen disponibles.

Sin almacenamiento persistente se muestra una explicacion y no se escribe una
configuracion Steam temporal. El menu no confunde un disco visible con una
biblioteca activa, ni una biblioteca registrada con un juego compatible.

## Observaciones del informe fisico recibido

Fuente: `A:/ULTRA/informe/gpu-7.3.0-rc7-pc-g32c52ec87d62.txt`, sesion terminada
el 22 de octubre a las 19:47:14 de Peru, Hyprland/NVIDIA, salida 0. SHA-256:
`e5364b8808f67841e05e7362413772cc3bdc1cde757360781c8834e2c7addd89`.
El informe original queda en respaldos locales excluidos de Git.

| Medida observada | Resultado |
| --- | --- |
| RAM usada estimada, 39 muestras | 0.68 a 2.66 GiB; ultima 2.52 GiB |
| VRAM NVIDIA, 39 muestras | Maximo 489 MiB; ultima 465 MiB |
| GPU | Utilizacion maxima muestreada 45 %; temperatura maxima 48 C |
| Ethernet | 100 Mb/s; contadores de interfaz sin errores ni descartes |

Son muestras de esta sesion, no picos garantizados ni mediciones de FPS. Los
contadores TCP incluyen retransmisiones: cero errores de interfaz no demuestra
cero problemas de red extremo a extremo. No se comparan con informes viejos de
otros perfiles como si fueran una prueba controlada.

Pendientes observados: fallo FUSE del portal de documentos y aviso del driver
sobre el pixel clock HDMI anunciado. La salida normal de Hyprland no elimina
estos avisos; esta entrega del panel de discos no los corrige.

## Verificacion

Nueve pruebas de estados, registro, errores, reloj y navegacion. El reloj de
60 segundos se comprueba con reloj simulado en esa prueba unitaria; el arranque
QEMU comprueba tambien un minuto real y el panel lanzado con Super+Espacio.
Se repiten once regresiones de montajes/capas en un espacio de montajes aislado,
sin discos fisicos. Un primer fallo unitario fue un doble de prueba mal
instalado; se conserva su informe junto al resultado corregido.
El primer arranque paso la apertura desde Super+Espacio, pero fallo en un
segundo lanzamiento del ensayo: faltaba XDG_RUNTIME_DIR al invocar el IPC.
Se corrigio el entorno del ensayo y se conserva ese informe fallido.

Entrega: ambos perfiles copiados a A:, kernels conservados y hash de datos
personales sin cambios. Genérico: 136 comprobaciones; PC: 137; cero fallos
finales. Inventarios, capturas e intentos previos:
`evidencias/discos/ce5f1a5f29c243d9abdd2737ec9db4a3/`.
